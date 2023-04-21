use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir};
use ir::{Builder, StaticEnable};
use std::rc::Rc;
#[derive(Default)]
/// This pass turns dynamic groups with static attributes into static groups
/// Upon visiting an Enable of a group with static attribute, it removes 
/// the done condition of the group, and copies all other assignments
/// into a static group, and changes the Enable control into a StaticEnable
pub struct DynamicToStatic;


impl Named for DynamicToStatic {
    fn name() -> &'static str {
        "dynamic-to-static"
    }

    fn description() -> &'static str {
        "Turn dynamic groups with latency attributes into static groups"
    }
}

impl Visitor for DynamicToStatic {
    fn enable(
        &mut self,
        s: &mut ir::Enable,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = Builder::new(comp, sigs);
        if let Some(&n) =
            s.group.borrow().get_attributes().unwrap().get("static")
        {
            builder
                .component
                .get_groups_mut()
                .remove(s.group.borrow().name());
            let sg = builder.add_static_group(s.group.borrow().name(), n);
            for assignment in s.group.borrow().assignments.iter() {
                if !(assignment.dst.borrow().is_hole()
                    && assignment.dst.borrow().name == "done")
                {
                    let static_s = assignment.into_static();
                    sg.borrow_mut().assignments.push(static_s);
                }
            }
            let s_enable = ir::StaticControl::Enable(StaticEnable {
                group: Rc::clone(&sg),
                attributes: s.attributes.clone(),
            });
            return Ok(Action::change(ir::Control::Static(s_enable)));
        }

        Ok(Action::Continue)
    }
}
