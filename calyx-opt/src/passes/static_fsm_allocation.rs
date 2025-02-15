use crate::traversal::{Named, Visitor};
use calyx_ir::{self as ir};
use std::collections::{HashMap, HashSet};
pub struct StaticFSMAllocation {}

impl Named for StaticFSMAllocation {
    fn name() -> &'static str {
        "static-fsm-alloc"
    }
    fn description() -> &'static str {
        "compiles a static schedule into an FSM construct"
    }
}

struct StaticAssign {
    dest: ir::RRC<ir::Port>,
    src: ir::RRC<ir::Port>,
}

/// An instance of `StaticSchedule` is constrainted to live at least as long as
/// the component in which the static island that it represents lives.
struct StaticSchedule<'a> {
    /// Builder construct to add hardware to the component it's built from
    builder: ir::Builder<'a>,
    /// Number of cycles to which the static schedule should count up
    latency: u64,
    /// Maps every FSM state to assignments that should be active in that state
    state2assigns: HashMap<u64, Vec<ir::Assignment<ir::StaticTiming>>>,
}

impl<'a> From<ir::Builder<'a>> for StaticSchedule<'a> {
    fn from(builder: ir::Builder<'a>) -> Self {
        StaticSchedule {
            builder,
            latency: 0,
            state2assigns: HashMap::new(),
        }
    }
}

impl<'a> StaticSchedule<'a> {
    /// Provided a static control node, calling this method on an empty `StaticSchedule`
    /// `sch` will build out the `latency` and `state2assigns` fields of `sch`, in
    /// preparation to replace the `StaticControl` node with an instance of `ir::FSM`.
    fn construct_schedule(&mut self, scon: &ir::StaticControl) {
        match scon {
            ir::StaticControl::Enable(sen) => {
                sen.group.borrow().assignments.iter().for_each(|sassign| {
                    sassign
                        .guard
                        .compute_live_states(sen.group.borrow().latency)
                        .into_iter()
                        .for_each(|state| {
                            self.state2assigns
                                .entry(state)
                                .and_modify(|other_assigns| {
                                    other_assigns.push(sassign.clone())
                                })
                                .or_insert(vec![sassign.clone()]);
                        })
                });
            }
            _ => (),
        }
    }
}

impl Visitor for StaticFSMAllocation {}
