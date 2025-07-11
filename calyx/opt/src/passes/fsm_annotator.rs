use crate::{
    analysis::StatePossibility,
    traversal::{Action, ConstructVisitor, Named, VisResult, Visitor},
};
use calyx_ir::{self as ir};
use calyx_utils::CalyxResult;

const NODE_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::NODE_ID);
pub struct FSMAnnotator {}

impl Named for FSMAnnotator {
    fn name() -> &'static str {
        "fsm-annotator"
    }
    fn description() -> &'static str {
        "annotate a control program, determining how FSMs should be allocated"
    }
}
impl ConstructVisitor for FSMAnnotator {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self> {
        Ok(FSMAnnotator {})
    }
    fn clear_data(&mut self) {}
}

impl FSMAnnotator {
    fn update_node_with_id(
        ctrl: &mut ir::Control,
        id: u64,
        (attr, attr_val): (ir::Attribute, u64),
    ) {
        if let Some(node_id_val) = ctrl.get_attribute(NODE_ID) {
            if node_id_val == id {
                ctrl.insert_attribute(attr, attr_val);
                return;
            }
        }

        return;
    }
}

impl Visitor for FSMAnnotator {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        //
        let mut ctrl_ref = comp.control.borrow_mut();
        let (mut st_poss, _) =
            StatePossibility::build_from_control(&mut ctrl_ref, 0);

        println!("BEFORE");

        println!("{:?}", st_poss);

        println!();
        println!("AFTER");

        st_poss.post_order_analysis();

        println!("{:?}", st_poss);
        println!();

        Ok(Action::Continue)
    }
}
