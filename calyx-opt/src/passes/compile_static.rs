use super::math_utilities::get_bit_width_from;
use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use calyx_ir::{guard, structure};
use ir::{build_assignments, Builder, Nothing, StaticTiming};
use itertools::Itertools;

#[derive(Default)]
/// Compiles Static Islands
pub struct CompileStatic;

impl Named for CompileStatic {
    fn name() -> &'static str {
        "compile-static"
    }

    fn description() -> &'static str {
        "Compiles Static  Islands"
    }
}

impl Visitor for CompileStatic {
    fn static_enable(
        &mut self,
        s: &mut ir::StaticEnable,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        let mut sgroup = s.group.borrow_mut();
        let latency = sgroup.get_latency();
        let fsm_size =
            get_bit_width_from(latency + 1 /* represent 0..latency */);
        structure!( builder;
            let fsm = prim std_reg(fsm_size);
            let adder = prim std_add(fsm_size);
            let const_one = constant(1, fsm_size);
            let first_state = constant(0, fsm_size);
            let last_state = constant(latency, fsm_size);
        );
        let assigns = sgroup.assignments.drain(..).collect_vec();
        for mut assign in assigns {
            assign.for_each_interval(|static_timing| {
                let (beg, end) = static_timing.get_interval();
                if beg == end {
                    let interval_const = builder.add_constant(beg, fsm_size);
                    let g =
                        guard!(fsm["out"]).eq(guard!(interval_const["out"]));
                    g
                } else {
                    let beg_const = builder.add_constant(beg, fsm_size);
                    let end_const = builder.add_constant(end, fsm_size);
                    let beg_guard: ir::Guard<StaticTiming> =
                        guard!(fsm["out"]).ge(guard!(beg_const["out"]));
                    let end_guard: ir::Guard<StaticTiming> =
                        guard!(fsm["out"]).le(guard!(end_const["out"]));
                    ir::Guard::And(Box::new(beg_guard), Box::new(end_guard))
                }
            })
        }

        let last_state_guard: ir::Guard<ir::Nothing> =
            guard!(fsm["out"]).eq(guard!(last_state["out"]));
        let not_last_state_guard: ir::Guard<ir::Nothing> =
            guard!(fsm["out"]).neq(guard!(last_state["out"]));
        let fsm_incr_assigns = build_assignments!(
          builder;
          adder["left"] = ? fsm["out"];
          adder["right"] = ? const_one["out"];
          fsm["in"] = not_last_state_guard ? adder["out"];
          fsm["in"] = last_state_guard ? first_state["out"];
        );
        builder.add_continuous_assignments(fsm_incr_assigns.to_vec());
        Ok(Action::Continue)
    }
}
