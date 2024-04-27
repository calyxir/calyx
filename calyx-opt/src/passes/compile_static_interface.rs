use crate::analysis::StaticSchedule;
use crate::passes::math_utilities::get_bit_width_from;
use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use ir::{
    build_assignments, guard, structure, Attributes, Guard, Nothing,
    StaticTiming, RRC,
};
use itertools::Itertools;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
pub struct CompileStaticInterface;

impl Named for CompileStaticInterface {
    fn name() -> &'static str {
        "compile-static-interface"
    }

    fn description() -> &'static str {
        "Compiles Static Component Interface"
    }
}

// Looks recursively thru guard to %[0:n] into %0 | %[1:n].
fn separate_first_cycle(
    guard: ir::Guard<StaticTiming>,
) -> ir::Guard<StaticTiming> {
    match guard {
        ir::Guard::Info(st) => {
            let (beg, end) = st.get_interval();
            if beg == 0 && end != 1 {
                let first_cycle =
                    ir::Guard::Info(ir::StaticTiming::new((0, 1)));
                let after = ir::Guard::Info(ir::StaticTiming::new((1, end)));
                let cong = ir::Guard::or(first_cycle, after);
                return cong;
            }
            guard
        }
        ir::Guard::And(l, r) => {
            let left = separate_first_cycle(*l);
            let right = separate_first_cycle(*r);
            ir::Guard::and(left, right)
        }
        ir::Guard::Or(l, r) => {
            let left = separate_first_cycle(*l);
            let right = separate_first_cycle(*r);
            ir::Guard::or(left, right)
        }
        ir::Guard::Not(g) => {
            let a = separate_first_cycle(*g);
            ir::Guard::Not(Box::new(a))
        }
        _ => guard,
    }
}

// Looks recursively thru assignment's guard to %[0:n] into %0 | %[1:n].
fn separate_first_cycle_assign(
    assign: ir::Assignment<StaticTiming>,
) -> ir::Assignment<StaticTiming> {
    ir::Assignment {
        src: assign.src,
        dst: assign.dst,
        attributes: assign.attributes,
        guard: Box::new(separate_first_cycle(*assign.guard)),
    }
}

// Used for guards in a one cycle static component.
// Replaces %0 with comp.go.
fn make_guard_dyn_one_cycle_static_comp(
    guard: ir::Guard<StaticTiming>,
    comp_sig: RRC<ir::Cell>,
) -> ir::Guard<Nothing> {
    match guard {
        ir::Guard::Or(l, r) => {
            let left =
                make_guard_dyn_one_cycle_static_comp(*l, Rc::clone(&comp_sig));
            let right =
                make_guard_dyn_one_cycle_static_comp(*r, Rc::clone(&comp_sig));
            ir::Guard::or(left, right)
        }
        ir::Guard::And(l, r) => {
            let left =
                make_guard_dyn_one_cycle_static_comp(*l, Rc::clone(&comp_sig));
            let right =
                make_guard_dyn_one_cycle_static_comp(*r, Rc::clone(&comp_sig));
            ir::Guard::and(left, right)
        }
        ir::Guard::Not(g) => {
            let f =
                make_guard_dyn_one_cycle_static_comp(*g, Rc::clone(&comp_sig));
            ir::Guard::Not(Box::new(f))
        }
        ir::Guard::Info(t) => {
            match t.get_interval() {
                (0, 1) => guard!(comp_sig["go"]),
                _ => unreachable!("This function is implemented for 1 cycle static components, only %0 can exist as timing guard"),

            }
        }
        ir::Guard::CompOp(op, l, r) => ir::Guard::CompOp(op, l, r),
        ir::Guard::Port(p) => ir::Guard::Port(p),
        ir::Guard::True => ir::Guard::True,
    }
}

// Used for assignments in a one cycle static component.
// Replaces %0 with comp.go in the assignment's guard.
fn make_assign_dyn_one_cycle_static_comp(
    assign: ir::Assignment<StaticTiming>,
    comp_sig: RRC<ir::Cell>,
) -> ir::Assignment<Nothing> {
    ir::Assignment {
        src: assign.src,
        dst: assign.dst,
        attributes: assign.attributes,
        guard: Box::new(make_guard_dyn_one_cycle_static_comp(
            *assign.guard,
            comp_sig,
        )),
    }
}

impl CompileStaticInterface {
    // Takes the assignments within a static component, and instantiates
    // an FSM (i.e., counter) to convert %[i:j] into i<= fsm < j.
    // Also includes logic to make fsm reset to 0 once it gets to n-1.
    fn make_early_reset_assigns_static_component(
        &mut self,
        sgroup_assigns: &mut Vec<ir::Assignment<ir::StaticTiming>>,
        latency: u64,
        fsm: ir::RRC<ir::Cell>,
        builder: &mut ir::Builder,
        comp_sig: RRC<ir::Cell>,
    ) -> Vec<ir::Assignment<Nothing>> {
        let fsm_name = fsm.borrow().name();
        let fsm_size = fsm
            .borrow()
            .find("out")
            .unwrap_or_else(|| unreachable!("no `out` port on {fsm_name}"))
            .borrow()
            .width;
        let mut assigns = sgroup_assigns
            .drain(..)
            .map(separate_first_cycle_assign)
            .collect_vec();
        let mut dyn_assigns = assigns
            .drain(..)
            .map(|assign| {
                StaticSchedule::make_assign_dyn(
                    assign,
                    &fsm,
                    fsm_size,
                    builder,
                    true,
                    Some(Rc::clone(&comp_sig)),
                )
            })
            .collect_vec();
        let this = Rc::clone(&comp_sig);
        structure!( builder;
            // done hole will be undefined bc of early reset
            let signal_on = constant(1,1);
            let adder = prim std_add(fsm_size);
            let const_one = constant(1, fsm_size);
            let first_state = constant(0, fsm_size);
            let final_state = constant(latency-1, fsm_size);
        );
        let g1: Guard<Nothing> = guard!(this["go"]);
        let g2: Guard<Nothing> = guard!(fsm["out"] == first_state["out"]);
        let trigger_guard = ir::Guard::and(g1, g2);
        let g3: Guard<Nothing> = guard!(fsm["out"] != first_state["out"]);
        let g4: Guard<Nothing> = guard!(fsm["out"] != final_state["out"]);
        let incr_guard = ir::Guard::and(g3, g4);
        let stop_guard: Guard<Nothing> =
            guard!(fsm["out"] == final_state["out"]);
        let fsm_incr_assigns = build_assignments!(
          builder;
          // Incrementsthe fsm
          adder["left"] = ? fsm["out"];
          adder["right"] = ? const_one["out"];
          // Always write into fsm.
          fsm["write_en"] = ? signal_on["out"];
          // If fsm == 0 and comp.go is high, then we start an execution.
          fsm["in"] = trigger_guard ? const_one["out"];
          // If 1 < fsm < n - 1, then we unconditionally increment the fsm.
          fsm["in"] = incr_guard ? adder["out"];
          // If fsm == n -1 , then we reset the FSM.
          fsm["in"] = stop_guard ? first_state["out"];
          // Otherwise the FSM is not assigned to, so it defaults to 0.
          // If we want, we could add an explicit assignment here that sets it
          // to zero.
        );
        dyn_assigns.extend(fsm_incr_assigns);

        dyn_assigns
    }

    // Makes `done` signal for promoted static<n> component.
    fn make_done_signal_for_promoted_component(
        &mut self,
        fsm: ir::RRC<ir::Cell>,
        builder: &mut ir::Builder,
        comp_sig: RRC<ir::Cell>,
    ) -> Vec<ir::Assignment<Nothing>> {
        let fsm_size = fsm
            .borrow()
            .find("out")
            .unwrap_or_else(|| {
                unreachable!("no `out` port on {}", fsm.borrow().name())
            })
            .borrow()
            .width;
        structure!(builder;
          let sig_reg = prim std_reg(1);
          let one = constant(1, 1);
          let zero = constant(0, 1);
          let first_state = constant(0, fsm_size);
        );
        let go_guard = guard!(comp_sig["go"]);
        let not_go_guard = !guard!(comp_sig["go"]);
        let first_state_guard = guard!(fsm["out"] == first_state["out"]);
        let comp_done_guard =
            guard!(fsm["out"] == first_state["out"]) & guard!(sig_reg["out"]);
        let assigns = build_assignments!(builder;
          // Only write to sig_reg when fsm == 0
          sig_reg["write_en"] = first_state_guard ? one["out"];
          // If fsm == 0 and comp.go is high, it means we are starting an execution,
          // so we set signal_reg to high. Note that this happens regardless of
          // whether comp.done is high.
          sig_reg["in"] = go_guard ? one["out"];
          // Otherwise, we set `sig_reg` to low.
          sig_reg["in"] = not_go_guard ? zero["out"];
          // comp.done is high when FSM == 0 and sig_reg is high,
          // since that means we have just finished an execution.
          comp_sig["done"] = comp_done_guard ? one["out"];
        );
        assigns.to_vec()
    }

    fn make_done_signal_for_promoted_component_one_cycle(
        &mut self,
        builder: &mut ir::Builder,
        comp_sig: RRC<ir::Cell>,
    ) -> Vec<ir::Assignment<Nothing>> {
        structure!(builder;
          let sig_reg = prim std_reg(1);
          let one = constant(1, 1);
          let zero = constant(0, 1);
        );
        let go_guard = guard!(comp_sig["go"]);
        let not_go = !guard!(comp_sig["go"]);
        let signal_on_guard = guard!(sig_reg["out"]);
        let assigns = build_assignments!(builder;
          // For one cycle components, comp.done is just whatever comp.go
          // was during the previous cycle.
          // signal_reg serves as a forwarding register that delays
          // the `go` signal for one cycle.
          sig_reg["in"] = go_guard ? one["out"];
          sig_reg["in"] = not_go ? zero["out"];
          sig_reg["write_en"] = ? one["out"];
          comp_sig["done"] = signal_on_guard ? one["out"];
        );
        assigns.to_vec()
    }
}

impl Visitor for CompileStaticInterface {
    fn start_static_control(
        &mut self,
        s: &mut ir::StaticControl,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if comp.is_static() && s.get_latency() > 1 {
            // Handle components with latency > 1.
            let latency = s.get_latency();
            if let ir::StaticControl::Enable(sen) = s {
                let mut sch = StaticSchedule::from(vec![Rc::clone(&sen.group)]);
                let mut builder = ir::Builder::new(comp, sigs);
                let (mut assigns, fsm) = sch.realize_schedule(&mut builder);
                builder
                    .component
                    .continuous_assignments
                    .extend(assigns.pop_front().unwrap());
                let comp_sig = Rc::clone(&builder.component.signature);
                if builder.component.attributes.has(ir::BoolAttr::Promoted) {
                    let done_assigns = self
                        .make_done_signal_for_promoted_component(
                            Rc::clone(&fsm),
                            &mut builder,
                            comp_sig,
                        );
                    builder
                        .component
                        .continuous_assignments
                        .extend(done_assigns);
                }
            }
        } else if comp.is_static() && s.get_latency() == 1 {
            // Handle components with latency == 1.
            if let ir::StaticControl::Enable(sen) = s {
                let assignments =
                    std::mem::take(&mut sen.group.borrow_mut().assignments);
                for assign in assignments {
                    let comp_sig = Rc::clone(&comp.signature);
                    comp.continuous_assignments.push(
                        make_assign_dyn_one_cycle_static_comp(assign, comp_sig),
                    );
                }
                if comp.attributes.has(ir::BoolAttr::Promoted) {
                    let mut builder = ir::Builder::new(comp, sigs);
                    let comp_sig = Rc::clone(&builder.component.signature);
                    let done_assigns = self
                        .make_done_signal_for_promoted_component_one_cycle(
                            &mut builder,
                            comp_sig,
                        );
                    builder
                        .component
                        .continuous_assignments
                        .extend(done_assigns);
                }
            }
        }
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Remove the control.
        if comp.is_static() {
            let _c = std::mem::replace(
                &mut comp.control,
                Rc::new(RefCell::new(ir::Control::Empty(ir::Empty {
                    attributes: Attributes::default(),
                }))),
            );
        }
        Ok(Action::Stop)
    }
}
