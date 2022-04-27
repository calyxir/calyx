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

        for (key, value) in static_group {
            if value.len() != 1 {
                let mut builder = ir::Builder::new(comp, sigs);
                let grp = builder.add_group("msp");
                let mut assignments: Vec<ir::Assignment> = Vec::new();
                for group in value.iter() {
                    assignments.extend(group.borrow().assignments.clone());
                }

                let (done_asmts, asmts): (
                    Vec<ir::Assignment>,
                    Vec<ir::Assignment>,
                ) = mem::take(&mut assignments)
                    .into_iter()
                    .partition(|x| x.dst.borrow().is_hole());

                grp.borrow_mut().assignments.extend(asmts);

                let mut fin_grd: ir::Guard = ir::Guard::True;
                for asmt in done_asmts.clone() {
                    let grd: ir::Guard = ir::Guard::Port(asmt.src);
                    fin_grd &= grd;
                    fin_grd &= *asmt.guard;
                }

                let cst = builder.add_constant(1, 1);

                let done_asmt = builder.build_assignment(
                    grp.borrow().get("done"),
                    cst.borrow().get("out"),
                    fin_grd,
                );

                grp.borrow_mut().assignments.push(done_asmt);

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
                    group: Rc::clone(&value[0]),
                    attributes: Attributes::default(),
                };
                enable.attributes.insert("static", key);
                s.stmts.push(ir::Control::Enable(enable));
            }
        }

        Ok(Action::Continue)
    }
}
