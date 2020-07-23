use crate::errors::{Error, Extract};
use crate::lang::{
    ast, component::Component, context::Context, structure_builder::ASTBuilder,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use crate::{port, structure};
use ast::{Control, Enable, GuardExpr, Port};
use std::collections::HashMap;

#[derive(Default)]
pub struct CompileControl {}

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
        // XXX(rachit): Not adding any new attributes. Is this correct?
        let if_group_node = st.insert_group(&if_group, HashMap::new())?;

        let cond_group_node =
            st.get_node_by_name(&cif.cond).extract(cif.cond.clone())?;
        let (cond_node, cond_port) = match &cif.port {
            Port::Comp { component, port } => Ok((
                st.get_node_by_name(component).extract(component.clone())?,
                port,
            )),
            Port::This { port } => {
                Ok((st.get_node_by_name(&"this".into()).unwrap(), port))
            }
            Port::Hole { .. } => Err(Error::MalformedControl(
                "Can't use a hole as a condition.".to_string(),
            )),
        }?;

        // extract group names from control statement
        let (true_group, false_group) = match (&*cif.tbranch, &*cif.fbranch) {
            (Control::Enable { data: t }, Control::Enable { data: f }) => {
                Ok((&t.comp, &f.comp))
            }
            _ => Err(Error::MalformedControl(
                "Both branches of an if must be an enable.".to_string(),
            )),
        }?;
        let true_group_node = st
            .get_node_by_name(true_group)
            .extract(true_group.clone())?;
        let false_group_node = st
            .get_node_by_name(false_group)
            .extract(false_group.clone())?;

        structure!(st, &ctx,
            let cond_computed = prim std_reg(1);
            let cond_stored = prim std_reg(1);
            let signal_const = constant(1, 1);
        );

        // cond[go] = !cond_computed.out ? 1'b1;
        st.insert_edge(
            signal_const.clone(),
            // (cond_group_node, st.port_ref(cond_group_node, "go")?.clone()),
            port!(st; cond_group_node["go"]),
            Some(if_group.clone()),
            Some(!st.to_guard(port!(st; cond_computed."out"))),
        )?;

        // cond_computed.in = cond[go] & cond[done] ? 1'b1;
        // cond_computed.write_en = cond[go] & cond[done] ? 1'b1;
        st.insert_edge(
            signal_const.clone(),
            port!(st; cond_computed."in"),
            Some(if_group.clone()),
            Some(
                st.to_guard(port!(st; cond_group_node["go"]))
                    & st.to_guard(port!(st; cond_group_node["done"])),
            ),
        )?;
        st.insert_edge(
            signal_const.clone(),
            port!(st; cond_computed."write_en"),
            Some(if_group.clone()),
            Some(
                st.to_guard(port!(st; cond_group_node["go"]))
                    & st.to_guard(port!(st; cond_group_node["done"])),
            ),
        )?;

        // cond_stored.in = cond[go] & cond[done] ? comp.out;
        // cond_stored.write_en = cond[go] & cond[done] ? comp.out;
        st.insert_edge(
            port!(st; cond_node.cond_port),
            // (cond_node, cond_port),
            port!(st; cond_stored."in"),
            // (cond_stored_reg, st.port_ref(cond_stored_reg, "in")?.clone()),
            Some(if_group.clone()),
            Some(
                st.to_guard(port!(st; cond_group_node["go"]))
                    & st.to_guard(port!(st; cond_group_node["done"])),
            ),
        )?;
        st.insert_edge(
            port!(st; cond_node.cond_port),
            // (cond_node, cond_port),
            port!(st; cond_stored."write_en"),
            // (cond_stored_reg, st.port_ref(cond_stored_reg, "in")?.clone()),
            Some(if_group.clone()),
            Some(
                st.to_guard(port!(st; cond_group_node["go"]))
                    & st.to_guard(port!(st; cond_group_node["done"])),
            ),
        )?;

        // true[go] = cond_computed.out & cond_stored.out ? 1'b1;
        st.insert_edge(
            signal_const.clone(),
            port!(st; true_group_node["go"]),
            Some(if_group.clone()),
            Some(
                st.to_guard(port!(st; cond_computed."out"))
                    & st.to_guard(port!(st; cond_stored."out")),
            ),
        )?;

        // false[go] = cond_computed.out & !cond_stored.out ? 1'b1;
        st.insert_edge(
            signal_const.clone(),
            port!(st; false_group_node["go"]),
            Some(if_group.clone()),
            Some(
                st.to_guard(port!(st; cond_computed."out"))
                    & !st.to_guard(port!(st; cond_stored."out")),
            ),
        )?;

        // this[done] = true[done] | false[done] ? 1'b1;
        st.insert_edge(
            signal_const,
            port!(st; if_group_node["done"]),
            Some(if_group.clone()),
            Some(
                st.to_guard(port!(st; true_group_node["done"]))
                    | st.to_guard(port!(st; false_group_node["done"])),
            ),
        )?;

        Ok(Action::Change(Control::enable(if_group)))
    }

    /// This compiles `while` statements of the following form:
    /// ```C
    /// while comp.out with cond {
    ///   body;
    /// }
    /// ```
    /// into the following group:
    /// ```C
    /// group while0 {
    ///   // compute the condition if we haven't before or we are done with the body
    ///   cond[go] = !cond_computed.out ? 1'b1;
    ///   // save whether we have finished computing the condition
    ///   cond_computed.in = cond[done] ? 1'b1;
    ///   // save the result of the condition
    ///   cond_stored.in = cond[done] ? lt.out;
    ///
    ///   // run the body if we have computed the condition and the condition was true
    ///   body[go] = cond_computed.out & cond_stored.out ? 1'b1;
    ///   // signal that we should recompute the condition when the body is done
    ///   cond_computed.in = body[done] ? 1'b0;
    ///   // this group is done when the condition is computed and it is false
    ///   while0[done] = cond_computed.out & !cond_stored.out ? 1'b1;
    ///   cond_computed.in = while0[done] ? 1'b1;
    ///   cond_computed.write_en = while0[done] ? 1'b1;
    /// }
    /// ```
    /// with 2 generated registers, `cond_computed` and `cond_stored`.
    /// `cond_computed` tracks whether we have computed the condition. This
    /// ensures that we don't recompute the condition when we are running the body.
    /// `cond_stored` saves the result of the condition so that it is accessible
    /// throughout the execution of `body`.
    ///
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
        let cond_group_node = st
            .get_node_by_name(&ctrl.cond)
            .ok_or_else(|| Error::UndefinedGroup(ctrl.cond.clone()))?;
        let (cond_node, cond_port) = match &ctrl.port {
            Port::Comp { component, port } => Ok((
                st.get_node_by_name(component).extract(component.clone())?,
                port,
            )),
            Port::This { port } => {
                Ok((st.get_node_by_name(&"this".into()).unwrap(), port))
            }
            Port::Hole { .. } => Err(Error::MalformedControl(
                "Can't use a hole as a condition.".to_string(),
            )),
        }?;

        // extract group names from control statement
        let body_group = match &*ctrl.body {
            Control::Enable { data } => Ok(&data.comp),
            _ => Err(Error::MalformedControl(
                "The body of a while must be an enable.".to_string(),
            )),
        }?;
        let body_group_node = st
            .get_node_by_name(body_group)
            .extract(body_group.clone())?;

        // generate necessary hardware
        structure!(st, &ctx,
            let cond_computed = prim std_reg(1);
            let cond_stored = prim std_reg(1);
            let done_reg = prim std_reg(1);
            let signal_on = constant(1, 1);
            let signal_off = constant(0, 1);
        );

        // cond[go] = !cond_computed.out ? 1'b1;
        st.insert_edge(
            signal_on.clone(),
            // (cond_group_node, st.port_ref(cond_group_node, "go")?.clone()),
            port!(st; cond_group_node["go"]),
            Some(while_group.clone()),
            Some(!st.to_guard(port!(st; cond_computed."out"))),
        )?;

        // cond_computed.in = cond[go] & cond[done] ? 1'b1;
        // cond_computed.write_en = cond[go] & cond[done] ? 1'b1;
        st.insert_edge(
            signal_on.clone(),
            port!(st; cond_computed."in"),
            // (
            //     cond_computed_reg,
            //     st.port_ref(cond_computed_reg, "in")?.clone(),
            // ),
            Some(while_group.clone()),
            Some(
                st.to_guard(port!(st; cond_group_node["go"]))
                    & st.to_guard(port!(st; cond_group_node["done"])),
            ),
        )?;
        st.insert_edge(
            signal_on.clone(),
            port!(st; cond_computed."write_en"),
            Some(while_group.clone()),
            Some(
                st.to_guard(port!(st; cond_group_node["go"]))
                    & st.to_guard(port!(st; cond_group_node["done"])),
            ),
        )?;

        // cond_stored.in = cond[go] & cond[done] ? lt.out;
        st.insert_edge(
            // (cond_node, cond_port.clone()),
            port!(st; cond_node.cond_port),
            // (cond_stored_reg, st.port_ref(cond_stored_reg, "in")?.clone()),
            port!(st; cond_stored."in"),
            Some(while_group.clone()),
            Some(
                st.to_guard(port!(st; cond_group_node["go"]))
                    & st.to_guard(port!(st; cond_group_node["done"])),
            ),
        )?;
        st.insert_edge(
            // (cond_node, cond_port.clone()),
            signal_on.clone(),
            // (cond_stored_reg, st.port_ref(cond_stored_reg, "in")?.clone()),
            port!(st; cond_stored."write_en"),
            Some(while_group.clone()),
            Some(
                st.to_guard(port!(st; cond_group_node["go"]))
                    & st.to_guard(port!(st; cond_group_node["done"])),
            ),
        )?;

        // body[go] = cond_computed.out & cond_stored.out & !body[done] ? 1'b1;
        st.insert_edge(
            signal_on.clone(),
            port!(st; body_group_node["go"]),
            Some(while_group.clone()),
            Some(
                st.to_guard(port!(st; cond_stored."out"))
                    & st.to_guard(port!(st; cond_computed."out"))
                    & !st.to_guard(port!(st; body_group_node["done"])),
            ),
        )?;

        // cond_computed.in = cond_computed.out & cond_stored.out & body[done] ? 1'b0;
        // cond_computed.write_en = cond_computed.out & cond_stored.out & body[done] ? 1'b1;
        st.insert_edge(
            signal_off.clone(),
            port!(st; cond_computed."in"),
            Some(while_group.clone()),
            Some(
                st.to_guard(port!(st; cond_stored."out"))
                    & st.to_guard(port!(st; cond_computed."out"))
                    & st.to_guard(port!(st; body_group_node["done"])),
            ),
        )?;
        st.insert_edge(
            signal_on.clone(),
            port!(st; cond_computed."write_en"),
            Some(while_group.clone()),
            Some(
                st.to_guard(port!(st; cond_stored."out"))
                    & st.to_guard(port!(st; cond_computed."out"))
                    & st.to_guard(port!(st; body_group_node["done"])),
            ),
        )?;

        // done_reg.in = cond_computed.out & !cond_stored.out ? 1'b1;
        // done_reg.write_en = cond_computed.out & !cond_stored.out ? 1'b1;
        // while0[done] = done_reg.out ? 1'b1;
        st.insert_edge(
            signal_on.clone(),
            port!(st; done_reg."in"),
            Some(while_group.clone()),
            Some(
                st.to_guard(port!(st; cond_computed."out"))
                    & !st.to_guard(port!(st; cond_stored."out")),
            ),
        )?;
        st.insert_edge(
            signal_on.clone(),
            port!(st; done_reg."write_en"),
            Some(while_group.clone()),
            Some(
                st.to_guard(port!(st; cond_computed."out"))
                    & !st.to_guard(port!(st; cond_stored."out")),
            ),
        )?;
        st.insert_edge(
            signal_on.clone(),
            port!(st; while_group_node["done"]),
            Some(while_group.clone()),
            Some(st.to_guard(port!(st; done_reg."out"))),
        )?;
        // cond_computed.in = done_reg.out ? 1'b0;
        // cond_computed.write_en = done_reg.out ? 1'b1;
        st.insert_edge(
            signal_off.clone(),
            port!(st; cond_computed."in"),
            Some(while_group.clone()),
            Some(
                st.to_guard(port!(st; cond_computed."out"))
                    & !st.to_guard(port!(st; cond_stored."out")),
            ),
        )?;
        st.insert_edge(
            signal_on.clone(),
            port!(st; cond_computed."write_en"),
            Some(while_group.clone()),
            Some(
                st.to_guard(port!(st; cond_computed."out"))
                    & !st.to_guard(port!(st; cond_stored."out")),
            ),
        )?;
        // done_reg.in = done_reg.out ? 1'b0;
        // done_reg.write_en = done_reg.out ? 1'b1;
        st.insert_edge(
            signal_off,
            port!(st; done_reg."in"),
            None,
            Some(st.to_guard(port!(st; done_reg."out"))),
        )?;
        st.insert_edge(
            signal_on,
            port!(st; done_reg."write_en"),
            None,
            Some(st.to_guard(port!(st; done_reg."out"))),
        )?;

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

        // new structure
        structure!(st, &ctx,
            let fsm = prim std_reg(32);
            let signal_const = constant(1, 1);
        );

        // Generate fsm to drive the sequence
        let mut fsm_counter = 0;
        for (idx, con) in s.stmts.iter().enumerate() {
            match con {
                Control::Enable {
                    data: Enable { comp: group_name },
                } => {
                    /* group[go] = fsm.out == value(fsm_counter) & !group[done] ? 1 */
                    let group = st
                        .get_node_by_name(&group_name)
                        .extract(group_name.clone())?;

                    let fsm_st_const = st.new_constant(fsm_counter, 32)?;

                    st.insert_edge(
                        signal_const.clone(),
                        port!(st; group["go"]),
                        Some(seq_group.clone()),
                        Some(
                            (st.to_guard(port!(st; fsm."out"))
                                .eq(st.to_guard(fsm_st_const.clone())))
                                & !st.to_guard(port!(st; group["done"])),
                        ),
                    )?;

                    fsm_counter += 1;

                    /* fsm.in = fsm.out == value(fsm_counter) & group[done] ? 1 */
                    let fsm_num = st.new_constant(fsm_counter, 32)?;
                    let done_guard = (st
                        .to_guard(port!(st; fsm."out"))
                        .eq(st.to_guard(fsm_st_const.clone())))
                        & st.to_guard(port!(st; group["done"]));
                    st.insert_edge(
                        fsm_num.clone(),
                        port!(st; fsm."in"),
                        Some(seq_group.clone()),
                        Some(done_guard.clone()),
                    )?;
                    let num = st.new_constant(1, 1)?;
                    st.insert_edge(
                        num,
                        port!(st; fsm."write_en"),
                        Some(seq_group.clone()),
                        Some(done_guard),
                    )?;

                    // If this is the last group, generate the done condition
                    // for the seq group.
                    if idx == s.stmts.len() - 1 {
                        structure!(st, &ctx,
                            let signal_go = constant(1, 1);
                            let signal_stop = constant(0, 32);
                        );
                        st.insert_edge(
                            signal_go.clone(),
                            port!(st; seq_group_node["done"]),
                            Some(seq_group.clone()),
                            Some(
                                st.to_guard(port!(st; fsm."out"))
                                    .eq(st.to_guard(fsm_num.clone())),
                            ),
                        )?;

                        st.insert_edge(
                            signal_stop,
                            port!(st; fsm."in"),
                            None,
                            Some(
                                st.to_guard(port!(st; fsm."out"))
                                    .eq(st.to_guard(fsm_num.clone())),
                            ),
                        )?;
                        st.insert_edge(
                            signal_go,
                            port!(st; fsm."write_en"),
                            None,
                            Some(
                                st.to_guard(port!(st; fsm."out"))
                                    .eq(st.to_guard(fsm_num)),
                            ),
                        )?;
                    }
                }
                _ => {
                    return Err(Error::MalformedControl(
                        "Cannot compile non-group statement inside sequence"
                            .to_string(),
                    ))
                }
            }
        }

        // Replace the control with the seq group.
        Ok(Action::Change(Control::enable(seq_group)))
    }

    /// Compiling par is straightforward: Hook up the go signal
    /// of the par group to the sub-groups and the done signal
    /// par group is the conjunction of sub-groups.
    fn finish_par(
        &mut self,
        s: &ast::Par,
        comp: &mut Component,
        _ctx: &Context,
    ) -> VisResult {
        let st = &mut comp.structure;

        // Name of the parent group.
        let par_group: ast::Id = st.namegen.gen_name("par").into();
        let par_group_idx = st.insert_group(&par_group, HashMap::new())?;

        let mut par_group_done: Option<GuardExpr> = None;

        for con in s.stmts.iter() {
            match con {
                Control::Enable {
                    data: Enable { comp: group_name },
                } => {
                    let group_idx = st
                        .get_node_by_name(&group_name)
                        .extract(group_name.clone())?;
                    let group_go_port =
                        st.port_ref(group_idx, "go").unwrap().clone();
                    // Hook up this group's go signal with parent's go.
                    let num = st.new_constant(1, 1)?;
                    st.insert_edge(
                        num,
                        (group_idx, group_go_port),
                        Some(par_group.clone()),
                        None,
                    )?;

                    // Add this group's done signal to parent's
                    // done signal.
                    let group_done_port =
                        st.port_ref(group_idx, "done").unwrap().clone();
                    let guard = st.to_guard((group_idx, group_done_port));
                    // par_group_done.push(guard);
                    par_group_done = Some(match par_group_done {
                        Some(g) => g & guard,
                        None => guard,
                    });
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
        let par_group_done_port =
            st.port_ref(par_group_idx, "done").unwrap().clone();
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            (par_group_idx, par_group_done_port),
            Some(par_group.clone()),
            par_group_done,
        )?;

        let new_control = Control::Enable {
            data: Enable { comp: par_group },
        };
        Ok(Action::Change(new_control))
    }
}
