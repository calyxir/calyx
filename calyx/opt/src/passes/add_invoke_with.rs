use std::fmt::format;

use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_frontend::SetAttr;
use calyx_ir::{self as ir, Nothing};
use calyx_utils::{CalyxResult, OutputFile};
use serde::Serialize;

// Very simple

pub struct AddInvokeWith {}

impl Named for AddInvokeWith {
    fn name() -> &'static str {
        "add-invoke-with"
    }

    fn description() -> &'static str {
        "Add a unique comb group to every invoke."
    }

    fn opts() -> Vec<crate::traversal::PassOpt> {
        vec![]
    }
}

impl ConstructVisitor for AddInvokeWith {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        Ok(AddInvokeWith {})
    }

    fn clear_data(&mut self) {}
}

impl Visitor for AddInvokeWith {
    fn invoke(
        &mut self,
        s: &mut calyx_ir::Invoke,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        let invoked_cell_name = s.comp.borrow().name();
        let comb_cell_prefix = format!("invoke_{invoked_cell_name}");
        match &s.comb_group {
            Some(_comb_group_ref) => {}
            None => {
                // create new comb group
                builder.add_comb_group(comb_cell_prefix);
            }
        }
        Ok(Action::Continue)
    }
}
