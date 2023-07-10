use super::math_utilities::get_bit_width_from;
use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::structure;
use calyx_ir::{self as ir, LibrarySignatures};

use ir::{build_assignments, guard};
/// Compiles [`ir::Invoke`](calyx_ir::Invoke) statements into an [`ir::Enable`](calyx_ir::Enable)
/// that runs the invoked component.
#[derive(Default)]
pub struct CompileRepeat;

impl Named for CompileRepeat {
    fn name() -> &'static str {
        "compile-repeat"
    }

    fn description() -> &'static str {
        "Rewrites repeat statements into while loops"
    }
}

impl Visitor for CompileRepeat {
    fn start_repeat(
        &mut self,
        s: &mut ir::Repeat,
        comp: &mut ir::Component,
        ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let num_repeats = s.num_repeats;
        let empty = Box::new(ir::Control::empty());
        if num_repeats == 0 {
            Ok(Action::Change(empty))
        } else if num_repeats == 1 {
            let repeat_body = std::mem::replace(&mut s.body, empty);
            Ok(Action::Change(repeat_body))
        } else {
            let mut builder = ir::Builder::new(comp, ctx);
            let idx_size = get_bit_width_from(num_repeats + 1);
            structure!( builder;
                let idx = prim std_reg(idx_size);
                let cond_reg = prim std_reg(1);
                let adder = prim std_add(idx_size);
                let lt = prim std_lt(idx_size);
                // done hole will be undefined bc of early reset
                let const_zero = constant(0, idx_size);
                let const_one = constant(1, idx_size);
                let num_repeats = constant(num_repeats, idx_size);
                let signal_on = constant(1,1);
            );
            let init_group = builder.add_group("init_repeat");
            let regs_done: ir::Guard<ir::Nothing> =
                guard!(cond_reg["done"]).and(guard!(idx["done"]));
            let init_assigns = build_assignments!(
              builder;
              // initial state for idx and cond_reg;
              idx["write_en"] = ? signal_on["out"];
              idx["in"] = ? const_zero["out"];
              cond_reg["write_en"] = ? signal_on["out"];
              cond_reg["in"] = ? signal_on["out"];
              init_group["done"] = regs_done ? signal_on["out"];
            )
            .to_vec();
            init_group.borrow_mut().assignments = init_assigns;
            let incr_group = builder.add_group("incr_repeat");
            let idx_incr_assigns = build_assignments!(
              builder;
              // increments the fsm
              adder["left"] = ? idx["out"];
              adder["right"] = ? const_one["out"];
              lt["left"] = ? adder["out"];
              lt["right"] = ? num_repeats["out"];
              cond_reg["write_en"] = ? signal_on["out"];
              cond_reg["in"] = ? lt["out"];
              idx["write_en"] = ? signal_on["out"];
              idx["in"] = ? adder["out"];
              incr_group["done"] = regs_done ? signal_on["out"];
            )
            .to_vec();
            incr_group.borrow_mut().assignments = idx_incr_assigns;
            let repeat_body = std::mem::replace(&mut s.body, empty);
            let while_body = ir::Control::par(vec![
                *repeat_body,
                ir::Control::enable(incr_group),
            ]);
            let while_loop = ir::Control::while_(
                cond_reg.borrow().get("out"),
                None,
                Box::new(while_body),
            );
            let while_seq = ir::Control::seq(vec![
                ir::Control::enable(init_group),
                while_loop,
            ]);
            Ok(Action::Change(Box::new(while_seq)))
        }
    }
}
