use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures};
use std::collections::HashMap;

#[derive(Default)]
/// Removes NODE_ID, BEGIN_ID, and END_ID from each control statement
pub struct AttributePromotion {
    /// maps dynamic group names its corresponding upgraded static group
    /// Then we use this to replace regular groups
    upgraded_groups: HashMap<ir::Id, ir::RRC<ir::StaticGroup>>,
}

impl Named for AttributePromotion {
    fn name() -> &'static str {
        "attr-promotion"
    }

    fn description() -> &'static str {
        "upgrades @static and @bound annotations to appropriate static control"
    }
}

impl Visitor for AttributePromotion {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // drain groups
        let comp_groups: Vec<ir::RRC<ir::Group>> =
            comp.get_groups_mut().drain().collect();
        let mut builder = ir::Builder::new(comp, sigs);
        let mut non_upgraded_groups = Vec::new();

        for group in comp_groups {
            let group_rc_clone = std::rc::Rc::clone(&group);
            let mut gr = group.borrow_mut();
            // if group has static annotation, then create corresponding static group, and add to self.upgraded_groups
            // otherwise just add back to component.groups
            if let Some(lat) = gr.attributes.get(ir::NumAttr::Static) {
                let sg = builder.add_static_group(gr.name(), lat);
                for assignment in gr.assignments.drain(..) {
                    // don't add done signal to static group
                    if !(assignment.dst.borrow().is_hole()
                        && assignment.dst.borrow().name == "done")
                    {
                        let static_s = ir::Assignment::from(assignment);
                        sg.borrow_mut().assignments.push(static_s);
                    }
                }
                self.upgraded_groups.insert(gr.name(), sg);
            } else {
                non_upgraded_groups.push(group_rc_clone);
            }
        }

        builder
            .component
            .get_groups_mut()
            .append(non_upgraded_groups.into_iter());

        Ok(Action::Continue)
    }

    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // check if this groups is indeed an "upgraded_group". If so, then change
        // to static enable. Otherwise continue.
        if let Some(static_group) =
            self.upgraded_groups.get(&s.group.borrow().name())
        {
            let static_enable = ir::StaticControl::Enable(ir::StaticEnable {
                group: std::rc::Rc::clone(static_group),
                attributes: s.attributes.clone(),
            });
            Ok(Action::change(ir::Control::Static(static_enable)))
        } else {
            Ok(Action::Continue)
        }
    }
}
