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

// Takes in a static guard `guard`, and returns equivalent dynamic guard
// The only thing that actually changes is the Guard::Info case
// We need to turn static_timing to dynamic guards using `fsm`.
// E.g.: %[2:3] gets turned into fsm.out >= 2 & fsm.out <= 3
fn make_guard_dyn(
    guard: ir::Guard<StaticTiming>,
    fsm: &ir::RRC<ir::Cell>,
    fsm_size: u64,
    builder: &mut ir::Builder,
) -> Box<ir::Guard<Nothing>> {
    match guard {
        ir::Guard::Or(l, r) => Box::new(ir::Guard::Or(
            make_guard_dyn(*l, fsm, fsm_size, builder),
            make_guard_dyn(*r, fsm, fsm_size, builder),
        )),
        ir::Guard::And(l, r) => Box::new(ir::Guard::And(
            make_guard_dyn(*l, fsm, fsm_size, builder),
            make_guard_dyn(*r, fsm, fsm_size, builder),
        )),
        ir::Guard::Not(g) => {
            Box::new(ir::Guard::Not(make_guard_dyn(*g, fsm, fsm_size, builder)))
        }
        ir::Guard::CompOp(op, l, r) => Box::new(ir::Guard::CompOp(op, l, r)),
        ir::Guard::Port(p) => Box::new(ir::Guard::Port(p)),
        ir::Guard::True => Box::new(ir::Guard::True),
        ir::Guard::Info(static_timing) => {
            let (beg, end) = static_timing.get_interval();
            if beg + 1 == end {
                // if beg + 1 == end then we only need to check if fsm == beg
                let interval_const = builder.add_constant(beg, fsm_size);
                let g = guard!(fsm["out"]).eq(guard!(interval_const["out"]));
                Box::new(g)
            } else if beg == 0 {
                // if beg == 0, then we only need to check if fsm < end
                let end_const = builder.add_constant(end, fsm_size);
                let lt: ir::Guard<Nothing> =
                    guard!(fsm["out"]).lt(guard!(end_const["out"]));
                Box::new(lt)
            } else {
                // otherwise, check if fsm >= beg & fsm < end
                let beg_const = builder.add_constant(beg, fsm_size);
                let end_const = builder.add_constant(end, fsm_size);
                let beg_guard: ir::Guard<Nothing> =
                    guard!(fsm["out"]).ge(guard!(beg_const["out"]));
                let end_guard: ir::Guard<Nothing> =
                    guard!(fsm["out"]).lt(guard!(end_const["out"]));
                Box::new(ir::Guard::And(
                    Box::new(beg_guard),
                    Box::new(end_guard),
                ))
            }
        }
    }
}

// Takes in static assignment `assign` and returns a dynamic assignments
// Mainly does two things:
// 1) if `assign` writes to a go/done hole, we should change that to the go/done
// hole of the new, dynamic group instead of the old static group
// 2) for each static Info guard (e.g. %[2:3]), we need to convert that to
// dynamic guards, using `fsm`.
// E.g.: %[2:3] gets turned into fsm.out >= 2 & fsm.out <= 3
fn make_assign_dyn(
    assign: ir::Assignment<StaticTiming>,
    dyn_group: &ir::RRC<ir::Group>,
    fsm: &ir::RRC<ir::Cell>,
    fsm_size: u64,
    builder: &mut ir::Builder,
) -> ir::Assignment<Nothing> {
    let new_dst = if assign.dst.borrow().is_hole() {
        // holes should be either go/done
        if assign.dst.borrow().name == "go" {
            dyn_group.borrow().get("go")
        } else {
            panic!("hole port other than go port")
        }
    } else {
        // if dst is not a hole, then we should keep it as is for the new assignment
        assign.dst
    };
    ir::Assignment {
        src: assign.src,
        dst: new_dst,
        attributes: assign.attributes,
        guard: make_guard_dyn(*assign.guard, fsm, fsm_size, builder),
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
        // create the builder/cells that we need to turn static group dynamic
        let mut builder = ir::Builder::new(comp, sigs);
        let mut sgroup = s.group.borrow_mut();
        let latency = sgroup.get_latency();
        let fsm_size =
            get_bit_width_from(latency + 1 /* represent 0..latency */);
        structure!( builder;
            let fsm = prim std_reg(fsm_size);
            let signal_on = constant(1,1);
            let adder = prim std_add(fsm_size);
            let const_one = constant(1, fsm_size);
            let first_state = constant(0, fsm_size);
            let last_state = constant(latency, fsm_size);
        );
        // create the dynamic group we will use to replace the static group
        let g = builder.add_group(sgroup.name());
        // converting static assignments to dynamic assignments
        let mut assigns = sgroup
            .assignments
            .drain(..)
            .map(|assign| {
                make_assign_dyn(assign, &g, &fsm, fsm_size, &mut builder)
            })
            .collect_vec();
        // assignments to increment the fsm
        let not_last_state_guard: ir::Guard<ir::Nothing> =
            guard!(fsm["out"]).neq(guard!(last_state["out"]));
        let last_state_guard: ir::Guard<ir::Nothing> =
            guard!(fsm["out"]).eq(guard!(last_state["out"]));
        let fsm_incr_assigns = build_assignments!(
          builder;
          adder["left"] = ? fsm["out"];
          adder["right"] = ? const_one["out"];
          fsm["write_en"] = not_last_state_guard ? signal_on["out"];
          fsm["in"] = not_last_state_guard ? adder["out"];
          // need done condition because we are creating a dynamic group
          g["done"] = last_state_guard ? signal_on["out"];
        );
        assigns.extend(fsm_incr_assigns.to_vec());
        // adding the assignments to the new dynamic group and creating a
        // new (dynamic) enable
        g.borrow_mut().assignments = assigns;
        g.borrow_mut().attributes = sgroup.attributes.clone();
        let mut e = ir::Control::enable(g);
        let attrs = std::mem::take(&mut s.attributes);
        *e.get_mut_attributes() = attrs;
        // need to add a continuous assignment to reset the fsm
        let fsm_reset_assigns = build_assignments!(builder;
            fsm["in"] = last_state_guard ? first_state["out"];
            fsm["write_en"] = last_state_guard ? signal_on["out"];
        );
        builder.add_continuous_assignments(fsm_reset_assigns.to_vec());
        Ok(Action::Change(Box::new(e)))
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // make sure static groups have no assignments, since
        // we should have already drained the assignments in static groups
        for g in comp.get_static_groups() {
            if !g.borrow().assignments.is_empty() {
                unreachable!("Should have converted all static groups to dynamic. {} still has assignments in it", g.borrow().name());
            }
        }
        // remove all static groups
        comp.get_static_groups_mut().retain(|_| false);
        Ok(Action::Continue)
    }
}
