use crate::ir::Attributes;
use crate::ir::Enable;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    RRC,
};
use linked_hash_map::LinkedHashMap;
use std::iter::Iterator;
use std::mem;
use std::rc::Rc;

#[derive(Default)]
/// under a par control block, if multiple groups have the same static attribute, then
/// merge them together.
///
/// Running this pass removes unnecessary FSM transitions
///
/// #Example
/// 1. Under a par block
/// group A<"static"=1>{
/// a; b;
/// }
/// group B<"static"=1>{
/// c; d;
/// }
/// par {A; B;}
///
/// into
///
/// group msp<"static"=1>{
/// a; b; c; d;
/// }
/// par {msp; }
///
/// note that a, b, c, d are assignments
pub struct MergeStaticPar;

impl Named for MergeStaticPar {
    fn name() -> &'static str {
        "merge-static-par"
    }

    fn description() -> &'static str {
        "merge static pars when they have the same static time"
    }
}

impl Visitor for MergeStaticPar {
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut static_group: LinkedHashMap<u64, Vec<RRC<ir::Group>>> =
            LinkedHashMap::new();
        let (e_stmts, n_stmts): (Vec<ir::Control>, Vec<ir::Control>) =
            mem::take(&mut s.stmts).into_iter().partition(|stmt| {
                if let ir::Control::Enable(en) = stmt {
                    matches!(
                        en.group.borrow().attributes.get("static"),
                        Some(_)
                    )
                } else {
                    false
                }
            });

        s.stmts.extend(n_stmts);

        for stmt in e_stmts {
            if let ir::Control::Enable(data) = stmt {
                let group = &data.group;
                if let Some(static_time) =
                    group.borrow().attributes.get("static")
                {
                    if !static_group.contains_key(static_time) {
                        static_group.insert(*static_time, Vec::new());
                    }
                    static_group
                        .get_mut(static_time)
                        .unwrap()
                        .push(Rc::clone(group));
                }
            }
        }

        for (key, groups) in static_group {
            if groups.len() != 1 {
                let mut builder = ir::Builder::new(comp, sigs);
                // Because all the done conditions are expected to be asserted as the same time,
                // we just pick the first one
                let grp = builder
                    .add_group("msp", groups[0].borrow().done_cond.clone());

                grp.borrow_mut().assignments.extend(
                    groups.iter().flat_map(|g| g.borrow().assignments.clone()),
                );
                grp.borrow_mut().attributes.insert("static", key);
                comp.groups.add(Rc::clone(&grp));

                let mut enable: ir::Enable = Enable {
                    group: Rc::clone(&grp),
                    attributes: Attributes::default(),
                };
                enable.attributes.insert("static", key);
                s.stmts.push(ir::Control::Enable(enable));
            } else {
                let mut enable: ir::Enable = Enable {
                    group: Rc::clone(&groups[0]),
                    attributes: Attributes::default(),
                };
                enable.attributes.insert("static", key);
                s.stmts.push(ir::Control::Enable(enable));
            }
        }

        Ok(Action::Continue)
    }
}
