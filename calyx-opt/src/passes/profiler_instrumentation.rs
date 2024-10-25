use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir, build_assignments, BoolAttr};
use calyx_utils::CalyxResult;

/// Adds probe wires to each group to detect when a group is active.
/// Used by the profiler.
pub struct ProfilerInstrumentation {}

impl Named for ProfilerInstrumentation {
    fn name() -> &'static str {
        "profiler-instrumentation"
    }

    fn description() -> &'static str {
        "Add instrumentation for profiling"
    }

    fn opts() -> Vec<crate::traversal::PassOpt> {
        vec![]
    }
}

impl ConstructVisitor for ProfilerInstrumentation {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        Ok(ProfilerInstrumentation {})
    }

    fn clear_data(&mut self) {}
}

impl Visitor for ProfilerInstrumentation {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // collect names of all groups (to construct group-specific cells)
        let group_names = comp
            .groups
            .iter()
            .map(|group| group.borrow().name())
            .collect::<Vec<_>>();
        let comp_name = comp.name;
        // for each group, construct a instrumentation cell and instrumentation assignment
        let mut asgn_and_cell = Vec::with_capacity(group_names.len());
        {
            let mut builder = ir::Builder::new(comp, sigs);
            let one: std::rc::Rc<std::cell::RefCell<calyx_ir::Cell>> =
                builder.add_constant(1, 1);
            for group_name in group_names.into_iter() {
                // store group and component name (differentiate between groups of the same name under different components)
                let name =
                    format!("{}__{}_probe", group_name, comp_name.to_string());
                let inst_cell = builder.add_primitive(name, "std_wire", &[1]);
                let asgn: [ir::Assignment<ir::Nothing>; 1] = build_assignments!(
                    builder;
                    inst_cell["in"] = ? one["out"];
                );
                // the probes should be @control because they should have value 0 whenever the corresponding group is not active.
                inst_cell.borrow_mut().add_attribute(BoolAttr::Control, 1);
                inst_cell.borrow_mut().add_attribute(BoolAttr::Protected, 1);
                asgn_and_cell.push((asgn[0].clone(), inst_cell));
            }
        }
        // add cells and assignments
        for (group, (asgn, inst_cell)) in
            comp.groups.iter().zip(asgn_and_cell.into_iter())
        {
            group.borrow_mut().assignments.push(asgn);
            comp.cells.add(inst_cell);
        }
        Ok(Action::Stop)
    }
}
