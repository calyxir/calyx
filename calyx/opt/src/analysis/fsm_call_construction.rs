use calyx_ir::{self as ir, FSM, PortParent, RRC};
use itertools::Itertools;

#[derive(Debug)]
pub struct FSMCallGraph {
    /// A map from the canonical representation of an FSM to the FSM construct
    fsms: Vec<RRC<FSM>>,
    /// A map from the canonical representation of an FSM to its successors in
    /// the call graph.
    tree: Vec<Vec<(usize, RRC<FSM>)>>,
}

impl FSMCallGraph {
    fn build(comp: &ir::Component) -> Self {
        let (fsms, tree) = comp
            .fsms
            .iter()
            .map(|fsm| {
                let fsm_calls = fsm
                    .borrow()
                    .assignments
                    .iter()
                    .enumerate()
                    .flat_map(|(state, asgns)| {
                        asgns
                            .iter()
                            .filter_map(|asgn| {
                                if let PortParent::FSM(sub_fsm) =
                                    &asgn.dst.borrow().parent
                                {
                                    Some((state, sub_fsm.upgrade()))
                                } else {
                                    None
                                }
                            })
                            .collect_vec()
                    })
                    .collect();

                (RRC::clone(fsm), fsm_calls)
            })
            .unzip();

        Self { fsms, tree }
    }
}
