use super::math_utilities::get_bit_width_from;
use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use calyx_ir::{guard, structure, GetAttributes};
use ir::{build_assignments, Nothing, StaticTiming};
use itertools::Itertools;

#[derive(Default)]
/// Compiles Static Islands
pub struct CompileStatic;

impl Named for CompileStatic {
    fn name() -> &'static str {
        "compile-static"
    }

    fn description() -> &'static str {
        "Compiles Static Islands"
    }
}

fn make_guard_dyn(
    guard: Box<ir::Guard<StaticTiming>>,
    fsm: &ir::RRC<ir::Cell>,
    fsm_size: u64,
    builder: &mut ir::Builder,
) -> Box<ir::Guard<Nothing>> {
    match *guard {
        ir::Guard::Or(l, r) => Box::new(ir::Guard::Or(
            make_guard_dyn(l, fsm, fsm_size, builder),
            make_guard_dyn(r, fsm, fsm_size, builder),
        )),
        ir::Guard::And(l, r) => Box::new(ir::Guard::And(
            make_guard_dyn(l, fsm, fsm_size, builder),
            make_guard_dyn(r, fsm, fsm_size, builder),
        )),
        ir::Guard::CompOp(op, l, r) => Box::new(ir::Guard::CompOp(op, l, r)),
        ir::Guard::Not(g) => {
            Box::new(ir::Guard::Not(make_guard_dyn(g, fsm, fsm_size, builder)))
        }
        ir::Guard::Port(p) => Box::new(ir::Guard::Port(p)),
        ir::Guard::True => Box::new(ir::Guard::True),
        ir::Guard::Info(static_timing) => {
            let (beg, end) = static_timing.get_interval();
            if beg == end {
                let interval_const = builder.add_constant(beg, fsm_size);
                let g = guard!(fsm["out"]).eq(guard!(interval_const["out"]));
                Box::new(g)
            } else {
                let beg_const = builder.add_constant(beg, fsm_size);
                let end_const = builder.add_constant(end, fsm_size);
                let beg_guard: ir::Guard<Nothing> =
                    guard!(fsm["out"]).ge(guard!(beg_const["out"]));
                let end_guard: ir::Guard<Nothing> =
                    guard!(fsm["out"]).le(guard!(end_const["out"]));
                Box::new(ir::Guard::And(
                    Box::new(beg_guard),
                    Box::new(end_guard),
                ))
            }
        }
    }
}

fn make_assign_dyn(
    assign: ir::Assignment<StaticTiming>,
    fsm: &ir::RRC<ir::Cell>,
    fsm_size: u64,
    builder: &mut ir::Builder,
) -> ir::Assignment<Nothing> {
    ir::Assignment {
        src: assign.src,
        dst: assign.dst,
        attributes: assign.attributes,
        guard: make_guard_dyn(assign.guard, fsm, fsm_size, builder),
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
        // converting static assignments to dynamic assignments
        let assigns = sgroup
            .assignments
            .drain(..)
            .map(|assign| make_assign_dyn(assign, &fsm, fsm_size, &mut builder))
            .collect_vec();
        // still need to add continuous assignments
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
        // adding the actual group
        let g = builder.add_group(sgroup.name());
        g.borrow_mut().assignments = assigns;
        g.borrow_mut().attributes = sgroup.attributes.clone();
        let mut e = ir::Control::enable(g);
        let attrs = std::mem::take(&mut s.attributes);
        *e.get_mut_attributes() = attrs;
        Ok(Action::Change(Box::new(e)))
    }
}
