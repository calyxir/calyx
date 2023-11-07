use super::compile_static::make_assign_dyn;
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
            let (beg, end) = t.get_interval();
            if beg != 0 || end != 1 {
                unreachable!("This function is implemented for 1 cycle static components, only %0 can exist as timing guard")
            } else {
                let g = guard!(comp_sig["go"]);
                g
            }
        }
        ir::Guard::CompOp(op, l, r) => ir::Guard::CompOp(op, l, r),
        ir::Guard::Port(p) => ir::Guard::Port(p),
        ir::Guard::True => ir::Guard::True,
    }
}

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
                make_assign_dyn(
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
            let penultimate_state = constant(latency-1, fsm_size);
        );
        let g1: Guard<Nothing> = guard!(this["go"]);
        let g2: Guard<Nothing> = guard!(fsm["out"] == first_state["out"]);
        let trigger_guard = ir::Guard::and(g1, g2);
        let g3: Guard<Nothing> = guard!(fsm["out"] != first_state["out"]);
        let g4: Guard<Nothing> = guard!(fsm["out"] != penultimate_state["out"]);
        let incr_guard = ir::Guard::and(g3, g4);
        let stop_guard: Guard<Nothing> =
            guard!(fsm["out"] == penultimate_state["out"]);
        let fsm_incr_assigns = build_assignments!(
          builder;
          // increments the fsm
          adder["left"] = ? fsm["out"];
          adder["right"] = ? const_one["out"];
          fsm["write_en"] = ? signal_on["out"];
          fsm["in"] = trigger_guard ? const_one["out"];
          fsm["in"] = incr_guard ? adder["out"];
           // resets the fsm early
          fsm["in"] = stop_guard ? first_state["out"];
        );
        dyn_assigns.extend(fsm_incr_assigns);

        dyn_assigns
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
            let latency = s.get_latency();
            if let ir::StaticControl::Enable(sen) = s {
                let mut builder = ir::Builder::new(comp, sigs);
                let fsm_size = get_bit_width_from(latency + 1);
                structure!( builder;
                    let fsm = prim std_reg(fsm_size);
                );
                let mut assignments =
                    std::mem::take(&mut sen.group.borrow_mut().assignments);
                let comp_sig = Rc::clone(&builder.component.signature);
                let dyn_assigns = self
                    .make_early_reset_assigns_static_component(
                        &mut assignments,
                        s.get_latency(),
                        fsm,
                        &mut builder,
                        comp_sig,
                    );
                builder.component.continuous_assignments.extend(dyn_assigns);
            }
        } else if comp.is_static() && s.get_latency() == 1 {
            if let ir::StaticControl::Enable(sen) = s {
                let assignments =
                    std::mem::take(&mut sen.group.borrow_mut().assignments);
                for assign in assignments {
                    let comp_sig = Rc::clone(&comp.signature);
                    comp.continuous_assignments.push(
                      make_assign_dyn_one_cycle_static_comp(assign, comp_sig)
                    );
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
        if comp.is_static() {
            //let _c = std::mem::replace(&mut comp.control, Rc::new(RefCell::new(ir::Control::Static(ir::StaticControl::Empty(ir::Empty{attributes:Attributes::default()})))));
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
