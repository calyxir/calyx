use crate::traversal::{Named, Visitor};
use calyx_ir::{self as ir};
pub struct StaticFSMAllocation {}

impl Named for StaticFSMAllocation {
    fn name() -> &'static str {
        "static-fsm-alloc"
    }
    fn description() -> &'static str {
        "compiles a static schedule into an FSM construct"
    }
}

/// An instance of [StaticSchedule] is constrainted to live at least as long as
/// the component in which the static island that it represents lives.
struct StaticSchedule<'a> {
    /// Builder construct to add hardware to the component it's built from
    builder: ir::Builder<'a>,
    /// Number of cycles to which the static schedule should count up
    latency: u64,
    /// List of state-guarded assignments that exist within the static island
    assigns: Vec<ir::Assignment<ir::StaticTiming>>,
}

impl<'a> From<ir::Builder<'a>> for StaticSchedule<'a> {
    fn from(builder: ir::Builder<'a>) -> Self {
        StaticSchedule {
            builder,
            latency: 0,
            assigns: Vec::new(),
        }
    }
}

impl<'a> StaticSchedule<'a> {
    /// Given an empty [StaticSchedule] and a static control node,
    fn construct_schedule(&mut self, scon: &ir::StaticControl) {}
}

impl Visitor for StaticFSMAllocation {}
