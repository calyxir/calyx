use crate::errors::Error;
use crate::lang::{
    ast, component::Component, context::Context, structure_builder::ASTBuilder,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use crate::{add_wires, guard, port, structure};
use ast::{Control, Enable, GuardExpr};
use std::collections::HashMap;
use std::convert::TryInto;

#[derive(Default)]
pub struct CompileControl {}

impl CompileControl {
    const EMPTY_GROUP: &'static str = "_empty";
}

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
    /// ```C
    /// if comp.out with cond {
    ///   true;
    /// } else {
    ///   false;
    /// }
    /// ```
    /// into the following group:
    /// ```C
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
        cif: &ast::If,
        comp: &mut Component,
        ctx: &Context,
    ) -> VisResult {
        let st = &mut comp.structure;

        // create a new group for if related structure
        let if_group: ast::Id = st.namegen.gen_name("if").into();
        let if_group_node = st.insert_group(&if_group, HashMap::new())?;

        let cond_group_node = st.get_node_by_name(&cif.cond)?;
        let cond = cif.port.get_edge(st)?;

        // extract group names from control statement
        let (true_group, false_group) = match (&*cif.tbranch, &*cif.fbranch) {
            (Control::Enable { data: t }, Control::Enable { data: f }) => {
                Ok((&t.comp, &f.comp))
            }
            _ => Err(Error::MalformedControl(
                "Both branches of an if must be an enable.".to_string(),
            )),
        }?;
        let true_group_node = st.get_node_by_name(true_group)?;
        let false_group_node = st.get_node_by_name(false_group)?;

        structure!(
            st, &ctx,
            let cond_computed = prim std_reg(1);
            let cond_stored = prim std_reg(1);
            let signal_const = constant(1, 1);
            let signal_off = constant(0, 1);
            let done_reg = prim std_reg(1);
        );

        // Guard definitions
        let cond_go = !guard!(st; cond_computed["out"]);
        let is_cond_computed = guard!(st; cond_group_node["go"])
            & guard!(st; cond_group_node["done"]);
        let true_go =
            guard!(st; cond_computed["out"]) & guard!(st; cond_stored["out"]);
        let false_go =
            guard!(st; cond_computed["out"]) & !guard!(st; cond_stored["out"]);
        let done_guard = (true_go.clone()
            & guard!(st; true_group_node["done"]))
            | (false_go.clone() & guard!(st; false_group_node["done"]));
        let done_reg_high = guard!(st; done_reg["out"]);

        // New edges.
        add_wires!(
            st, Some(if_group.clone()),
            // Run the conditional group.
            cond_group_node["go"] = cond_go ? (signal_const.clone());
            cond_computed["in"] = is_cond_computed ? (signal_const.clone());
            cond_computed["write_en"] = is_cond_computed ? (signal_const.clone());
            cond_stored["in"] = is_cond_computed ? (cond.clone());
            cond_stored["write_en"] = is_cond_computed ? (cond);

            // Run a branch
            true_group_node["go"] = true_go ? (signal_const.clone());
            false_group_node["go"] = false_go ? (signal_const.clone());

            // Done condition for this group.
            done_reg["in"] = done_guard ? (signal_const.clone());
            done_reg["write_en"] = done_guard ? (signal_const.clone());
            if_group_node["done"] = done_reg_high ? (signal_const.clone());
        );

        // CLEANUP: done register resets one cycle after being high.
        add_wires!(
            st, None,
            done_reg["in"] = done_reg_high ? (signal_off.clone());
            done_reg["write_en"] = done_reg_high ? (signal_const.clone());
            cond_computed["in"] = done_reg_high ? (signal_off);
            cond_computed["write_en"] = done_reg_high ? (signal_const);
        );

        Ok(Action::Change(Control::enable(if_group)))
    }

    /// XXX(rachit): The explanation is not consistent with the code.
    /// Specifically, explain what the `done_reg` stuff is doing.
    // This compiles `while` statements of the following form:
    // ```C
    // while comp.out with cond {
    //   body;
    // }
    // ```
    // into the following group:
    // ```C
    // group while0 {
    //   // compute the condition if we haven't before or we are done with the body
    //   cond[go] = !cond_computed.out ? 1'b1;
    //   // save whether we have finished computing the condition
    //   cond_computed.in = cond[done] ? 1'b1;
    //   // save the result of the condition
    //   cond_stored.in = cond[done] ? lt.out;
    //
    //   // run the body if we have computed the condition and the condition was true
    //   body[go] = cond_computed.out & cond_stored.out ? 1'b1;
    //   // signal that we should recompute the condition when the body is done
    //   cond_computed.in = body[done] ? 1'b0;
    //   // this group is done when the condition is computed and it is false
    //   while0[done] = cond_computed.out & !cond_stored.out ? 1'b1;
    //   cond_computed.in = while0[done] ? 1'b1;
    //   cond_computed.write_en = while0[done] ? 1'b1;
    // }
    // ```
    // with 2 generated registers, `cond_computed` and `cond_stored`.
    // `cond_computed` tracks whether we have computed the condition. This
    // ensures that we don't recompute the condition when we are running the body.
    // `cond_stored` saves the result of the condition so that it is accessible
    // throughout the execution of `body`.
    //
    fn finish_while(
        &mut self,
        ctrl: &ast::While,
        comp: &mut Component,
        ctx: &Context,
    ) -> VisResult {
        let st = &mut comp.structure;

        // create group
        let while_group = st.namegen.gen_name("while").into();
        let while_group_node = st.insert_group(&while_group, HashMap::new())?;

        // cond group
        let cond_group_node = st.get_node_by_name(&ctrl.cond)?;

        let cond = ctrl.port.get_edge(&*st)?;

        // extract group names from control statement
        let body_group = match &*ctrl.body {
            Control::Enable { data } => Ok(&data.comp),
            _ => Err(Error::MalformedControl(
                "The body of a while must be an enable.".to_string(),
            )),
        }?;
        let body_group_node = st.get_node_by_name(body_group)?;

        // generate necessary hardware
        structure!(st, &ctx,
            let cond_computed = prim std_reg(1);
            let cond_stored = prim std_reg(1);
            let done_reg = prim std_reg(1);
            let signal_on = constant(1, 1);
            let signal_off = constant(0, 1);
        );

        let cond_go = !guard!(st; cond_computed["out"]);
        let is_cond_computed = guard!(st; cond_group_node["go"])
            & guard!(st; cond_group_node["done"]);
        let body_go = guard!(st; cond_stored["out"])
            & guard!(st; cond_computed["out"])
            & !guard!(st; body_group_node["done"]);
        let cond_recompute = guard!(st; cond_stored["out"])
            & guard!(st; cond_computed["out"])
            & guard!(st; body_group_node["done"]);
        let is_cond_false =
            guard!(st; cond_computed["out"]) & !guard!(st; cond_stored["out"]);
        let done_reg_high = guard!(st; done_reg["out"]);
        add_wires!(st, Some(while_group.clone()),
            // Initially compute the condition
            cond_group_node["go"] = cond_go ? (signal_on.clone());
            cond_computed["in"] = is_cond_computed ? (signal_on.clone());
            cond_computed["write_en"] = is_cond_computed ? (signal_on.clone());
            cond_stored["in"] = is_cond_computed ? (cond);
            cond_stored["write_en"] = is_cond_computed ? (signal_on.clone());

            // Enable the body
            body_group_node["go"] = body_go ? (signal_on.clone());

            // Re-compute the condition after the body is done.
            cond_computed["in"] = cond_recompute ? (signal_off.clone());
            cond_computed["write_en"] = cond_recompute ? (signal_on.clone());

            // Save done condition in done reg.
            done_reg["in"] = is_cond_false ? (signal_on.clone());
            done_reg["write_en"] = is_cond_false ? (signal_on.clone());

            // While group is done when done register is high.
            while_group_node["done"] = done_reg_high ? (signal_on.clone());

            // CLEANUP: Reset cond register state when done.
            cond_computed["in"] = is_cond_false ? (signal_off.clone());
            cond_computed["write_en"] = is_cond_false ? (signal_on.clone());
        );

        // CLEANUP: done register resets one cycle after being high.
        add_wires!(st, None,
            done_reg["in"] = done_reg_high ? (signal_off);
            done_reg["write_en"] = done_reg_high ? (signal_on);
        );

        Ok(Action::Change(Control::enable(while_group)))
    }

    fn finish_seq(
        &mut self,
        s: &ast::Seq,
        comp: &mut Component,
        ctx: &Context,
    ) -> VisResult {
        let st = &mut comp.structure;

        // Create a new group for the seq related structure.
        let seq_group: ast::Id = st.namegen.gen_name("seq").into();
        let seq_group_node = st.insert_group(&seq_group, HashMap::new())?;
        let fsm_size = 32;

        // new structure
        structure!(st, &ctx,
            let fsm = prim std_reg(fsm_size);
            let signal_on = constant(1, 1);
        );

        // Generate fsm to drive the sequence
        for (idx, con) in s.stmts.iter().enumerate() {
            match con {
                Control::Enable {
                    data: Enable { comp: group_name },
                } => {
                    let my_idx: u64 = idx.try_into().unwrap();
                    /* group[go] = fsm.out == idx & !group[done] ? 1 */
                    let group = st.get_node_by_name(&group_name)?;

                    structure!(st, &ctx,
                        let fsm_cur_state = constant(my_idx, fsm_size);
                        let fsm_nxt_state = constant(my_idx + 1, fsm_size);
                    );

                    let group_go = (guard!(st; fsm["out"])
                        .eq(st.to_guard(fsm_cur_state.clone())))
                        & !guard!(st; group["done"]);

                    let group_done = (guard!(st; fsm["out"])
                        .eq(st.to_guard(fsm_cur_state.clone())))
                        & guard!(st; group["done"]);

                    add_wires!(st, Some(seq_group.clone()),
                        // Turn this group on.
                        group["go"] = group_go ? (signal_on.clone());

                        // Update the FSM state when this group is done.
                        fsm["in"] = group_done ? (fsm_nxt_state.clone());
                        fsm["write_en"] = group_done ? (signal_on.clone());
                    );
                }
                _ => {
                    return Err(Error::MalformedControl(
                        "Cannot compile non-group statement inside sequence"
                            .to_string(),
                    ))
                }
            }
        }

        let final_state_val: u64 = s.stmts.len().try_into().unwrap();
        structure!(st, &ctx,
            let reset_val = constant(0, fsm_size);
            let fsm_final_state = constant(final_state_val, fsm_size);
        );
        let seq_done = guard!(st; fsm["out"]).eq(st.to_guard(fsm_final_state));

        // Condition for the seq group being done.
        add_wires!(st, Some(seq_group.clone()),
            seq_group_node["done"] = seq_done ? (signal_on.clone());
        );

        // CLEANUP: Reset the FSM state one cycle after the done signal is high.
        add_wires!(st, None,
            fsm["in"] = seq_done ? (reset_val);
            fsm["write_en"] = seq_done ? (signal_on);
        );

        // Replace the control with the seq group.
        Ok(Action::Change(Control::enable(seq_group)))
    }

    /// Par compilation generates 1-bit registers to hold `done` values
    /// for each group and generates go signals that are guarded by these
    /// `done` registers being low.
    fn finish_par(
        &mut self,
        s: &ast::Par,
        comp: &mut Component,
        ctx: &Context,
    ) -> VisResult {
        let st = &mut comp.structure;

        // Name of the parent group.
        let par_group: ast::Id = st.namegen.gen_name("par").into();
        let par_group_idx = st.insert_group(&par_group, HashMap::new())?;
        let mut par_group_done: Vec<GuardExpr> =
            Vec::with_capacity(s.stmts.len());
        let mut par_done_regs = Vec::with_capacity(s.stmts.len());

        structure!(st, &ctx,
            let signal_on = constant(1, 1);
            let signal_off = constant(0, 1);
            let par_reset = prim std_reg(1);
        );

        for con in s.stmts.iter() {
            match con {
                Control::Enable {
                    data: Enable { comp: group_name },
                } => {
                    let group_idx = st.get_node_by_name(&group_name)?;

                    // Create register to hold this group's done signal.
                    structure!(st, &ctx,
                        let par_done_reg = prim std_reg(1);
                    );

                    let group_go = !(guard!(st; par_done_reg["out"])
                        | guard!(st; group_idx["done"]));
                    let group_done = guard!(st; group_idx["done"]);

                    add_wires!(st, Some(par_group.clone()),
                        group_idx["go"] = group_go ? (signal_on.clone());

                        par_done_reg["in"] = group_done ? (signal_on.clone());
                        par_done_reg["write_en"] = group_done ? (signal_on.clone());
                    );

                    par_done_regs.push(par_done_reg);
                    // Add this group's done signal to parent's
                    // done signal.
                    par_group_done.push(guard!(st; par_done_reg["out"]));
                }
                _ => {
                    return Err(Error::MalformedControl(
                        "Cannot compile non-group statement inside sequence"
                            .to_string(),
                    ))
                }
            }
        }

        // Hook up parent's done signal to all children.
        let par_done = GuardExpr::and_vec(par_group_done);
        let par_reset_out = guard!(st; par_reset["out"]);
        add_wires!(st, Some(par_group.clone()),
            par_reset["in"] = par_done ? (signal_on.clone());
            par_reset["write_en"] = par_done ? (signal_on.clone());
            par_group_idx["done"] = par_reset_out ? (signal_on.clone());
        );

        // reset wires
        add_wires!(st, None,
            par_reset["in"] = par_reset_out ? (signal_off.clone());
            par_reset["write_en"] = par_reset_out ? (signal_on.clone());
        );
        for par_done_reg in par_done_regs {
            add_wires!(st, None,
                       par_done_reg["in"] = par_reset_out ? (signal_off.clone());
                       par_done_reg["write_en"] = par_reset_out ? (signal_on.clone());
            );
        }

        Ok(Action::Change(Control::enable(par_group)))
    }

    fn finish_empty(
        &mut self,
        _s: &ast::Empty,
        comp: &mut Component,
        _x: &Context,
    ) -> VisResult {
        let st = &mut comp.structure;
        // Create a group that always outputs done if it doesn't exist.
        let empty_group: ast::Id = CompileControl::EMPTY_GROUP.into();
        // Try to get the empty group.
        if st.get_node_by_name(&empty_group).is_err() {
            let empty_group_node =
                st.insert_group(&empty_group, HashMap::new())?;

            structure!(st, &ctx,
                let signal_on = constant(1, 1);
            );

            add_wires!(
                st, Some(empty_group.clone()),
                empty_group_node["done"] = (signal_on);
            );
        }
        Ok(Action::Change(Control::enable(empty_group)))
    }
}
