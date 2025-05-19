use calyx_ir::{self as ir, FSM, RRC};

pub struct FSMCallGraph {
    /// A map from the canonical representation of an FSM to the FSM construct
    id2fsm: Vec<RRC<FSM>>,
    /// A map from the canonical representation of an FSM to its successors in
    /// the call graph.
    tree: Vec<Vec<u16>>,
}

impl FSMCallGraph {
    fn new(comp: &ir::Component) -> Self {
        let (id2fsm, tree) = comp
            .fsms
            .iter()
            .map(|fsm| (RRC::clone(fsm), Vec::new()))
            .unzip();

        Self { id2fsm, tree }
    }
}
