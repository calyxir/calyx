use crate::analysis;
use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{
    self as ir, build_assignments, BoolAttr, Cell, LibrarySignatures,
};
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
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(ctx);

        Ok(Instrument {})
    }

    fn clear_data(&mut self) {
        /* All data can be transferred between components */
    }
}

impl Visitor for Instrument {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let group_names = comp
            .groups
            .iter()
            .map(|group| group.borrow().name())
            .collect::<Vec<_>>();
        let mut asgn_and_cell = Vec::with_capacity(group_names.len());
        {
            let mut builder = ir::Builder::new(comp, sigs);
            let one = builder.add_constant(1, 1);
            for group_name in group_names.into_iter() {
                let name = format!("{}_inst", group_name);
                let inst_cell = builder.add_primitive(name, "std_wire", &[1]);
                let asgn: [ir::Assignment<ir::Nothing>; 1] = build_assignments!(
                    builder;
                    inst_cell["in"] = ? one["out"];
                );
                inst_cell.borrow_mut().add_attribute(BoolAttr::Protected, 1);
                asgn_and_cell.push((asgn[0].clone(), inst_cell));
            }
        }
        for (group, (asgn, inst_cell)) in
            comp.groups.iter().zip(asgn_and_cell.into_iter())
        {
            group.borrow_mut().assignments.push(asgn);
            comp.cells.add(inst_cell);
        }

        Ok(Action::Stop)
    }
}
