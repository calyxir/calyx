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
    fn finish_repeat(
        &mut self,
        s: &mut ir::Repeat,
        comp: &mut ir::Component,
        ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let num_repeats = s.num_repeats;
        if num_repeats == 0 {
            // 0 repeats is the same thing as an empty control statement.
            Ok(Action::change(ir::Control::empty()))
        } else if num_repeats == 1 {
            // 1 repeat means we can just replace the repeat stmt with the body.
            Ok(Action::change(s.body.take_control()))
        } else {
            // Otherwise we should build a while loop.
            let mut builder = ir::Builder::new(comp, ctx);
            let idx_size = get_bit_width_from(num_repeats + 1);
            structure!( builder;
                // holds the idx of the iteration
                let idx = prim std_reg(idx_size);
                // cond_reg.out will be condition port for the while loop
                let cond_reg = prim std_reg(1);
                let adder = prim std_add(idx_size);
                let lt = prim std_lt(idx_size);
                let const_zero = constant(0, idx_size);
                let const_one = constant(1, idx_size);
                let num_repeats = constant(num_repeats, idx_size);
                let signal_on = constant(1,1);
            );
            // regs_done is `cond_reg.done & idx.done`
            let regs_done: ir::Guard<ir::Nothing> =
                guard!(cond_reg["done"] & idx["done"]);
            // init_group sets cond_reg to 1 and idx to 0
            let init_group = builder.add_group("init_repeat");
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
            init_group
                .borrow_mut()
                .attributes
                .insert(ir::NumAttr::Promotable, 1);
            // incr_group:
            // 1) writes results of idx + 1 into idx (i.e., increments idx)
            // 2) writes the result of (idx + 1 < num_repeats) into cond_reg,
            let incr_group = builder.add_group("incr_repeat");
            let idx_incr_assigns = build_assignments!(
              builder;
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
            incr_group
                .borrow_mut()
                .attributes
                .insert(ir::NumAttr::Promotable, 1);
            // create control:
            // init_group; while cond_reg.out {repeat_body; incr_group;}
            let while_body = ir::Control::seq(vec![
                s.body.take_control(),
                ir::Control::enable(incr_group),
            ]);
            let while_loop = ir::Control::while_(
                cond_reg.borrow().get("out"),
                None,
                Box::new(while_body),
            );
            let while_seq = ir::Control::Seq(ir::Seq {
                stmts: vec![ir::Control::enable(init_group), while_loop],
                attributes: std::mem::take(&mut s.attributes),
            });
            Ok(Action::change(while_seq))
        }
    }
}
