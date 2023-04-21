use super::math_utilities::get_bit_width_from;
use crate::passes::TopDownCompileControl;
use crate::{build_assignments, guard, structure};
use calyx_ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures,
};
use calyx_utils::Error;
use std::convert::TryInto;
use std::rc::Rc;

#[derive(Default)]
/// **Reference lowering pass**. Traverses a control program bottom-up and
/// transforms each control sub-program into a single enable statement.
/// *Not used in the default compilation pipeline.*
///
/// This pass uses an older compilation strategy that generates worse FSM-controller.
/// It is left in-tree because it serves as a second source of truth for the
/// lowering process.
pub struct CompileControl;

impl Named for CompileControl {
    fn name() -> &'static str {
        "compile-control"
    }

    fn description() -> &'static str {
        "Compile away all control language constructs into structure"
    }
}

impl Visitor for CompileControl {
    /// This compiles `if` statements of the following form:
    /// ```
    /// if comp.out with cond {
    ///   true;
    /// } else {
    ///   false;
    /// }
    /// ```
    /// into the following group:
    /// ```
    /// if0 {
    ///   // compute the condition if we haven't computed it before
    ///   cond[go] = !cond_computed.out ? 1'b1;
    ///   // save whether we are done computing the condition
    ///   cond_computed.in = cond[done] ? 1'b1;
    ///   cond_computed.write_en = cond[done] ? 1'b1;
    ///   // when the cond is done, store the output of the condition
    ///   cond_stored.in = cond[done] ? comp.out;
    ///   // run the true branch if we have computed the condition and it was true
    ///   true[go] = cond_computed.out & cond_stored.out ? 1'b1;
    ///   // run the false branch if we have computed the condition and it was false
    ///   false[go] = cond_computed.out & !cond_stored.out ? 1'b1;
    ///   // this group is done if either branch is done
    ///   or.right = true[done];
    ///   or.left = false[done];
    ///   if0[done] = or.out;
    /// }
    /// ```
    /// with 2 generated registers, `cond_computed` and `cond_stored`.
    /// `cond_computed` keeps track of whether the condition has been
    /// computed or not. This ensures that we only compute the condition once.
    /// `cond_stored` stores the output of the condition in a register so that
    /// we can use it for any number of cycles.
    ///
    /// We also generate a logical `or` component with the `done` holes
    /// of the two bodies as inputs. The generated `if0` group is `done`
    /// when either branch is done.
    fn finish_if(
        &mut self,
        cif: &mut ir::If,
        comp: &mut ir::Component,
        ctx: &LibrarySignatures,
    ) -> VisResult {
        todo!("compile-control support for if-with")

        /* let mut builder = ir::Builder::new(comp, ctx);

        // create a new group for if related structure
        let if_group = builder.add_group("if");

        if cif.cond.is_some() {
            return Err(Error::MalformedStructure(format!(
                "{}: if without `with` is not supported. Use `{}` instead",
                Self::name(),
                TopDownCompileControl::name()
            )));
        }

        let cond_group = Rc::clone(cif.cond.as_ref().unwrap());
        let cond = Rc::clone(&cif.port);

        // extract group names from control statement
        let (tru, fal) = match (&*cif.tbranch, &*cif.fbranch) {
            (ir::Control::Enable(t), ir::Control::Enable(f)) => {
                Ok((Rc::clone(&t.group), Rc::clone(&f.group)))
            }
            _ => Err(Error::PassAssumption(
                Self::name().to_string(),
                "Both branches of an if must be an enable.".to_string(),
            )),
        }?;

        structure!(builder;
            let signal_on = constant(1, 1);
            let signal_off = constant(0, 1);
            let cond_computed = prim std_reg(1);
            let cond_stored = prim std_reg(1);
            let done_reg = prim std_reg(1);
        );

        // Guard definitions
        let cond_go = !guard!(cond_computed["out"]);
        let is_cond_computed =
            guard!(cond_group["go"]) & guard!(cond_group["done"]);

        let true_turn =
            guard!(cond_computed["out"]) & guard!(cond_stored["out"]);
        let true_go = !guard!(tru["done"]) & true_turn.clone();

        let false_turn =
            guard!(cond_computed["out"]) & !guard!(cond_stored["out"]);
        let false_go = !guard!(fal["done"]) & false_turn.clone();

        let done_guard = (true_turn & guard!(tru["done"]))
            | (false_turn & guard!(fal["done"]));
        let done_reg_high = guard!(done_reg["out"]);

        let mut cond_save_assigns = vec![
            builder.build_assignment(
                cond_stored.borrow().get("in"),
                Rc::clone(&cond),
                is_cond_computed.clone(),
            ),
            builder.build_assignment(
                cond_stored.borrow().get("write_en"),
                Rc::clone(&cond),
                is_cond_computed.clone(),
            ),
        ];
        if_group
            .borrow_mut()
            .assignments
            .append(&mut cond_save_assigns);
        let mut group_assigns = build_assignments!(builder;
            // Run the conditional group.
            cond_group["go"] = cond_go ? signal_on["out"];
            cond_computed["in"] = is_cond_computed ? signal_on["out"];
            cond_computed["write_en"] = is_cond_computed ? signal_on["out"];

            // Run a branch
            tru["go"] = true_go ? signal_on["out"];
            fal["go"] = false_go ? signal_on["out"];

            // Done condition for this group.
            done_reg["in"] = done_guard ? signal_on["out"];
            done_reg["write_en"] = done_guard ? signal_on["out"];
            if_group["done"] = done_reg_high ? signal_on["out"];
        );
        if_group.borrow_mut().assignments.append(&mut group_assigns);

        // CLEANUP: done register resets one cycle after being high.
        let mut cleanup_assigns = build_assignments!(builder;
            done_reg["in"] = done_reg_high ? signal_off["out"];
            done_reg["write_en"] = done_reg_high ? signal_on["out"];
            cond_computed["in"] = done_reg_high ? signal_off["out"];
            cond_computed["write_en"] = done_reg_high ? signal_on["out"];
            cond_stored["in"] = done_reg_high ? signal_off["out"];
            cond_stored["write_en"] = done_reg_high ? signal_on["out"];
        );
        comp.continuous_assignments.append(&mut cleanup_assigns);

        Ok(Action::Change(ir::Control::enable(if_group))) */
    }

    /// XXX(rachit): The explanation is not consistent with the code.
    /// Specifically, explain what the `done_reg` stuff is doing.
    /// This compiles `while` statements of the following form:
    fn finish_while(
        &mut self,
        wh: &mut ir::While,
        comp: &mut ir::Component,
        ctx: &LibrarySignatures,
    ) -> VisResult {
        todo!()
        /* let mut builder = ir::Builder::new(comp, ctx);

        // create group
        let while_group = builder.add_group("while");

        // cond group
        let cond_group = Rc::clone(&wh.cond);
        let cond = Rc::clone(&wh.port);

        // extract group names from control statement
        let body_group = match &*wh.body {
            ir::Control::Enable(data) => &data.group,
            _ => unreachable!("The body of a while must be an enable."),
        };

        // generate necessary hardware
        structure!(builder;
            let cond_computed = prim std_reg(1);
            let cond_stored = prim std_reg(1);
            let done_reg = prim std_reg(1);
            let signal_on = constant(1, 1);
            let signal_off = constant(0, 1);
        );

        let cond_go = !guard!(cond_computed["out"]);
        let is_cond_computed =
            guard!(cond_group["go"]) & guard!(cond_group["done"]);
        let body_go = guard!(cond_stored["out"])
            & guard!(cond_computed["out"])
            & !guard!(body_group["done"]);
        let cond_recompute = guard!(cond_stored["out"])
            & guard!(cond_computed["out"])
            & guard!(body_group["done"]);
        let is_cond_false =
            guard!(cond_computed["out"]) & !guard!(cond_stored["out"]);
        let done_reg_high = guard!(done_reg["out"]);

        let cond_val_assign = builder.build_assignment(
            cond_stored.borrow().get("in"),
            cond,
            is_cond_computed.clone(),
        );
        while_group.borrow_mut().assignments.push(cond_val_assign);
        let mut while_assigns = build_assignments!(builder;
            // Initially compute the condition
            cond_group["go"] = cond_go ? signal_on["out"];
            cond_computed["in"] = is_cond_computed ? signal_on["out"];
            cond_computed["write_en"] = is_cond_computed ? signal_on["out"];
            cond_stored["write_en"] = is_cond_computed ? signal_on["out"];

            // Enable the body
            body_group["go"] = body_go ? signal_on["out"];

            // Re-compute the condition after the body is done.
            cond_computed["in"] = cond_recompute ? signal_off["out"];
            cond_computed["write_en"] = cond_recompute ? signal_on["out"];

            // Save done condition in done reg.
            done_reg["in"] = is_cond_false ? signal_on["out"];
            done_reg["write_en"] = is_cond_false ? signal_on["out"];

            // While group is done when done register is high.
            while_group["done"] = done_reg_high ? signal_on["out"];

            // CLEANUP: Reset cond register state when done.
            cond_computed["in"] = is_cond_false ? signal_off["out"];
            cond_computed["write_en"] = is_cond_false ? signal_on["out"];
        );
        while_group
            .borrow_mut()
            .assignments
            .append(&mut while_assigns);

        // CLEANUP: done register resets one cycle after being high.
        let mut clean_assigns = build_assignments!(builder;
            done_reg["in"] = done_reg_high ? signal_off["out"];
            done_reg["write_en"] = done_reg_high ? signal_on["out"];
        );
        comp.continuous_assignments.append(&mut clean_assigns);

        Ok(Action::Change(ir::Control::enable(while_group))) */
    }

    fn finish_seq(
        &mut self,
        s: &mut ir::Seq,
        comp: &mut ir::Component,
        ctx: &LibrarySignatures,
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, ctx);

        // Create a new group for the seq related structure.
        let seq_group = builder.add_group("seq");
        let fsm_size = get_bit_width_from(1 + s.stmts.len() as u64);

        // new structure
        structure!(builder;
            let fsm = prim std_reg(fsm_size);
            let signal_on = constant(1, 1);
        );

        // Generate fsm to drive the sequence
        for (idx, con) in s.stmts.iter().enumerate() {
            match con {
                ir::Control::Enable(ir::Enable { group, .. }) => {
                    let my_idx: u64 = idx.try_into().unwrap();
                    /* group[go] = fsm.out == idx & !group[done] ? 1 */
                    structure!(builder;
                        let fsm_cur_state = constant(my_idx, fsm_size);
                        let fsm_nxt_state = constant(my_idx + 1, fsm_size);
                    );

                    let group_go = guard!(fsm["out"])
                        .eq(guard!(fsm_cur_state["out"]))
                        .and(!guard!(group["done"]));

                    let group_done = guard!(fsm["out"])
                        .eq(guard!(fsm_cur_state["out"]))
                        .and(guard!(group["done"]));

                    let mut assigns = build_assignments!(builder;
                        // Turn this group on.
                        group["go"] = group_go ? signal_on["out"];

                        // Update the FSM state when this group is done.
                        fsm["in"] = group_done ? fsm_nxt_state["out"];
                        fsm["write_en"] = group_done ? signal_on["out"];
                    );
                    seq_group.borrow_mut().assignments.append(&mut assigns);
                }
                _ => {
                    unreachable!("Children of `seq` statement should be groups")
                }
            }
        }

        let final_state_val: u64 = s.stmts.len().try_into().unwrap();
        structure!(builder;
            let reset_val = constant(0, fsm_size);
            let fsm_final_state = constant(final_state_val, fsm_size);
        );
        let seq_done = guard!(fsm["out"]).eq(guard!(fsm_final_state["out"]));

        // Condition for the seq group being done.
        let mut assigns = build_assignments!(builder;
            seq_group["done"] = seq_done ? signal_on["out"];
        );
        seq_group.borrow_mut().assignments.append(&mut assigns);

        // CLEANUP: Reset the FSM state one cycle after the done signal is high.
        let mut assigns = build_assignments!(builder;
            fsm["in"] = seq_done ? reset_val["out"];
            fsm["write_en"] = seq_done ? signal_on["out"];
        );
        comp.continuous_assignments.append(&mut assigns);

        // Replace the control with the seq group.
        Ok(Action::Change(ir::Control::enable(seq_group)))
    }

    /// Par compilation generates 1-bit registers to hold `done` values
    /// for each group and generates go signals that are guarded by these
    /// `done` registers being low.
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        comp: &mut ir::Component,
        ctx: &LibrarySignatures,
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, ctx);

        // Name of the parent group.
        let par_group = builder.add_group("par");

        let mut par_group_done: Vec<ir::Guard> =
            Vec::with_capacity(s.stmts.len());
        let mut par_done_regs = Vec::with_capacity(s.stmts.len());

        structure!(builder;
            let signal_on = constant(1, 1);
            let signal_off = constant(0, 1);
            let par_reset = prim std_reg(1);
        );

        for con in s.stmts.iter() {
            match con {
                ir::Control::Enable(ir::Enable { group, .. }) => {
                    // Create register to hold this group's done signal.
                    structure!(builder;
                        let par_done_reg = prim std_reg(1);
                    );

                    let group_go =
                        !(guard!(par_done_reg["out"]) | guard!(group["done"]));
                    let group_done = guard!(group["done"]);

                    let mut assigns = build_assignments!(builder;
                        group["go"] = group_go ? signal_on["out"];

                        par_done_reg["in"] = group_done ? signal_on["out"];
                        par_done_reg["write_en"] = group_done ? signal_on["out"];
                    );
                    par_group.borrow_mut().assignments.append(&mut assigns);

                    // Add this group's done signal to parent's
                    // done signal.
                    par_group_done.push(guard!(par_done_reg["out"]));
                    par_done_regs.push(par_done_reg);
                }
                _ => unreachable!(
                    "Children of `par` statement should be enables"
                ),
            }
        }

        // Hook up parent's done signal to all children.
        let par_done = par_group_done
            .into_iter()
            .fold(ir::Guard::True, ir::Guard::and);
        let par_reset_out = guard!(par_reset["out"]);
        let mut assigns = build_assignments!(builder;
            par_reset["in"] = par_done ? signal_on["out"];
            par_reset["write_en"] = par_done ? signal_on["out"];
            par_group["done"] = par_reset_out ? signal_on["out"];
        );
        par_group.borrow_mut().assignments.append(&mut assigns);

        // reset wires
        let mut assigns = build_assignments!(builder;
            par_reset["in"] = par_reset_out ? signal_off["out"];
            par_reset["write_en"] = par_reset_out ? signal_on["out"];
        );
        builder
            .component
            .continuous_assignments
            .append(&mut assigns);
        for par_done_reg in par_done_regs {
            let mut assigns = build_assignments!(builder;
               par_done_reg["in"] = par_reset_out ? signal_off["out"];
               par_done_reg["write_en"] = par_reset_out ? signal_on["out"];
            );
            builder
                .component
                .continuous_assignments
                .append(&mut assigns);
        }

        Ok(Action::Change(ir::Control::enable(par_group)))
    }
}
