use crate::analysis::{IncompleteTransition, StaticSchedule};
use crate::traversal::{Action, ConstructVisitor, Named, Visitor};
use calyx_ir::{self as ir, BoolAttr, GetAttributes};
use calyx_utils::CalyxResult;
use core::ops::Not;
use itertools::Itertools;
const ACYCLIC: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::ACYCLIC);
pub struct FSMBuilder {}

pub struct AbstractFSMTypeTodo {}

impl Named for FSMBuilder {
    fn name() -> &'static str {
        "fsm-builder"
    }
    fn description() -> &'static str {
        "generates medium fsms in one pass for static and dynamic"
    }
}

impl ConstructVisitor for FSMBuilder {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self> {
        Ok(FSMBuilder {})
    }
    fn clear_data(&mut self) {}
}

impl StaticSchedule<'_, '_> {
    fn build_abstract_acyclic() -> AbstractFSMTypeTodo {
        // allocate one state per cycle
        todo!()
    }
    fn build_abstract_cyclic(
        &mut self,
        control: &ir::StaticControl,
    ) -> AbstractFSMTypeTodo {
        // similar to one-state behavior
        todo!()
    }
    fn fsm_build() -> ir::RRC<ir::FSM> {
        todo!()
    }
}

impl Visitor for FSMBuilder {
    fn enable(
        &mut self,
        sen: &mut calyx_ir::Enable,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        if matches!(sen.get_attributes().get(ACYCLIC), Some(1)) {
            todo!()
        }
        Ok(Action::Continue)
    }

    fn finish_static_control(
        &mut self,
        scon: &mut calyx_ir::StaticControl,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        // non-promoted static components are static islands

        // otherwise need to do a dynamic handshake
        Ok(Action::Continue)
    }
}
