use crate::analysis::StaticSchedule;
use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use ir::{
    build_assignments, guard, structure, Attributes, Nothing, StaticTiming, RRC,
};
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
            if let ir::StaticControl::Enable(sen) = s {
                // Build a StaticSchedule object, realize it and add assignments
                // as continuous assignments.
                let mut sch = StaticSchedule::from(vec![Rc::clone(&sen.group)]);
                let mut builder = ir::Builder::new(comp, sigs);
                let (mut assigns, fsm) =
                    sch.realize_schedule(&mut builder, true);
                builder
                    .component
                    .continuous_assignments
                    .extend(assigns.pop_front().unwrap());
                let comp_sig = Rc::clone(&builder.component.signature);
                if builder.component.attributes.has(ir::BoolAttr::Promoted) {
                    // If necessary, add the logic to produce a done signal.
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
            // In this case, we don't need an FSM; we just guard the assignments
            // with comp.go.
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
