use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir, build_assignments, BoolAttr};
use calyx_utils::CalyxResult;

pub struct Instrument {}

impl Named for Instrument {
    fn name() -> &'static str {
        "instrument"
    }

    fn description() -> &'static str {
        "Add instrumentation"
    }

    fn opts() -> Vec<crate::traversal::PassOpt> {
        vec![]
    }
}

impl ConstructVisitor for Instrument {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        Ok(Instrument {})
    }

    fn clear_data(&mut self) {}
}

impl Visitor for Instrument {
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
        // for each group, construct a instrumentation cell and instrumentation assignment
        let mut asgn_and_cell = Vec::with_capacity(group_names.len());
        {
            let mut builder = ir::Builder::new(comp, sigs);
            let one = builder.add_constant(1, 1);
            for group_name in group_names.into_iter() {
                let name = format!("{}_inst", group_name);
                let inst_cell = builder.add_primitive(name, "std_probe", &[1]);
                let asgn: [ir::Assignment<ir::Nothing>; 1] = build_assignments!(
                    builder;
                    inst_cell["in"] = ? one["out"];
                );
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
