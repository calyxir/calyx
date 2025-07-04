use calyx_ir::{self as ir};

struct FSMCallGraph {
    graph: Vec<AbstractFSM>,
}

struct AbstractFSM {
    states: Vec<StatePossibility>,
}

enum StatePossibility {
    Hardware {
        assignments: Vec<ir::Assignment<ir::Nothing>>,
        transitions: ir::Transition,
    },
    Call(FSMCallGraphNode),
}

enum FSMCallGraphNode {
    UserDefined {
        local_id: u64,
    },
    StaticRepeat {
        body_latency: u64,
        local_id: u64,
        num_repeats: u64,
    },
    StaticPar {
        max_thread_latency: u64,
        local_ids: Vec<u64>,
    },
    StaticSeq {
        latency: u64,
        local_id: u64,
    },
    StaticIf {
        max_branch_latency: u64,
        t_branch_id: u64,
        f_branch_id: u64,
    },
    DynamicSeq {
        local_id: u64,
    },
    DynamicPar {
        local_ids: Vec<u64>,
    },
    DynamicIf {
        t_branch_id: u64,
        f_branch_id: u64,
    },
    DynamicWhile {
        t_branch_id: u64,
        f_branch_id: u64,
    },
    DynamicRepeat {
        child_id: u64,
        num_repeats: u64,
    },
}

impl FSMCallGraphNode {
    fn is_static(&self) -> bool {
        match self {
            Self::StaticRepeat { .. }
            | Self::StaticPar { .. }
            | Self::StaticSeq { .. }
            | Self::StaticIf { .. } => true,
            _ => false,
        }
    }
}
