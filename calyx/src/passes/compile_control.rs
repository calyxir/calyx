use crate::errors::{Error, Extract};
use crate::lang::{
    ast, component::Component, context::Context, structure_builder::ASTBuilder,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use ast::{Control, Enable, GuardExpr, Port};

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

#[macro_export]
macro_rules! port {
    ($struct:expr; $node:ident.$port:ident) => {
        ($node, $struct.port_ref($node, $port)?.clone())
    };
    ($struct:expr; $node:ident[$port:ident]) => {
        ($node, $struct.port_ref($node, $port)?.clone())
    };
    ($struct:expr; $node:ident.$port:literal) => {
        ($node, $struct.port_ref($node, $port)?.clone())
    };
    ($struct:expr; $node:ident[$port:literal]) => {
        ($node, $struct.port_ref($node, $port)?.clone())
    };
}

macro_rules! new_structure {
    ($struct:expr,
     $ctx:expr,
     $( $var:ident = prim $comp:ident( $($n:literal),* ) );* $(;)?) => {
        $(
            let $var = $struct.new_primitive(
                $ctx,
                stringify!($var),
                stringify!($comp),
                &[$($n),*]
            )?;
        )*
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
        let if_group_node = st.insert_group(&if_group)?;

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

        // generate necessary hardware
        // let cond_computed =
        //     st.new_primitive(&ctx, "cond_computed", "std_reg", &[1])?;
        // let cond_stored =
        //     st.new_primitive(&ctx, "cond_stored", "std_reg", &[1])?;
        // let branch_or = st.new_primitive(&ctx, "if_or", "std_or", &[1])?;

        new_structure!(
            st, &ctx,
            cond_computed = prim std_reg(1);
            cond_stored = prim std_reg(1);
            branch_or = prim std_or(1);
        );

        // cond[go] = !cond_computed.out ? 1'b1;
        let cond_go_guard =
            GuardExpr::Not(st.to_atom(port!(st; cond_computed."out")));
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            // (cond_group_node, st.port_ref(cond_group_node, "go")?.clone()),
            port!(st; cond_group_node["go"]),
            Some(if_group.clone()),
            vec![cond_go_guard],
        )?;

        // cond_computed.in = cond[done] ? 1'b1;
        let cond_computed_guard =
            GuardExpr::Atom(st.to_atom(port!(st; cond_group_node["done"])));
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            port!(st; cond_computed."in"),
            Some(if_group.clone()),
            vec![cond_computed_guard],
        )?;

        // cond_stored.in = cond[done] ? comp.out;
        let cond_stored_guard =
            GuardExpr::Atom(st.to_atom(port!(st; cond_group_node["done"])));
        st.insert_edge(
            port!(st; cond_node.cond_port),
            // (cond_node, cond_port),
            port!(st; cond_stored."in"),
            // (cond_stored_reg, st.port_ref(cond_stored_reg, "in")?.clone()),
            Some(if_group.clone()),
            vec![cond_stored_guard],
        )?;

        // true[go] = cond_computed.out & cond_stored.out ? 1'b1;
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            port!(st; true_group_node["go"]),
            Some(if_group.clone()),
            vec![
                GuardExpr::Atom(st.to_atom(port!(st; cond_computed."out"))),
                GuardExpr::Atom(st.to_atom(port!(st; cond_stored."out"))),
            ],
        )?;

        // false[go] = cond_computed.out & !cond_stored.out ? 1'b1;
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            port!(st; false_group_node["go"]),
            Some(if_group.clone()),
            vec![
                GuardExpr::Atom(st.to_atom(port!(st; cond_computed."out"))),
                GuardExpr::Not(st.to_atom(port!(st; cond_stored."out"))),
            ],
        )?;

        // or.right = true[done] ? 1'b1;
        // or.left = false[done] ? 1'b1;
        // this[done] = or.out;
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            port!(st; branch_or."left"),
            Some(if_group.clone()),
            vec![GuardExpr::Atom(
                st.to_atom(port!(st; true_group_node["done"])),
            )],
        )?;
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            port!(st; branch_or."right"),
            Some(if_group.clone()),
            vec![GuardExpr::Atom(
                st.to_atom(port!(st; false_group_node["done"])),
            )],
        )?;
        st.insert_edge(
            (branch_or, st.port_ref(branch_or, "out")?.clone()),
            (if_group_node, st.port_ref(if_group_node, "done")?.clone()),
            Some(if_group.clone()),
            vec![],
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
        let while_group_node = st.insert_group(&while_group)?;

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
        let cond_computed =
            st.new_primitive(&ctx, "cond_computed", "std_reg", &[1])?;
        let cond_stored =
            st.new_primitive(&ctx, "cond_stored", "std_reg", &[1])?;

        // cond[go] = !cond_computed.out ? 1'b1;
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            // (cond_group_node, st.port_ref(cond_group_node, "go")?.clone()),
            port!(st; cond_group_node["go"]),
            Some(while_group.clone()),
            vec![GuardExpr::Not(st.to_atom(port!(st; cond_computed."out")))],
        )?;

        // cond_computed.in = cond[done] ? 1'b1;
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            port!(st; cond_computed."in"),
            // (
            //     cond_computed_reg,
            //     st.port_ref(cond_computed_reg, "in")?.clone(),
            // ),
            Some(while_group.clone()),
            vec![GuardExpr::Atom(
                st.to_atom(port!(st; cond_group_node["done"])),
            )],
        )?;

        // cond_stored.in = cond[done] ? lt.out;
        st.insert_edge(
            // (cond_node, cond_port.clone()),
            port!(st; cond_node.cond_port),
            // (cond_stored_reg, st.port_ref(cond_stored_reg, "in")?.clone()),
            port!(st; cond_stored."in"),
            Some(while_group.clone()),
            vec![GuardExpr::Atom(
                st.to_atom(port!(st; cond_group_node["done"])),
            )],
        )?;

        // body[go] = cond_computed.out & cond_stored.out ? 1'b1;
        // let body_go_guard = GuardExpr::Atom(st.to_atom(
        //     cond_stored_reg,
        //     st.port_ref(cond_stored_reg, "out")?.clone(),
        // ));
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            // (body_group_node, st.port_ref(body_group_node, "go")?.clone()),
            port!(st; body_group_node["go"]),
            Some(while_group.clone()),
            vec![GuardExpr::Atom(st.to_atom(port!(st; cond_stored."out")))],
        )?;

        // cond_computed.in = body[done] ? 1'b0
        let num = st.new_constant(0, 1)?;
        st.insert_edge(
            num,
            // (
            //     cond_computed_reg,
            //     st.port_ref(cond_computed_reg, "in")?.clone(),
            // ),
            port!(st; cond_computed."in"),
            Some(while_group.clone()),
            vec![GuardExpr::Atom(st.to_atom(port!(st;
                body_group_node["done"]
            )))],
        )?;

        // while0[done] = cond_computed.out & !cond_stored.out ? 1'b1;
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            // (
            //     while_group_node,
            //     st.port_ref(while_group_node, "done")?.clone(),
            // ),
            port!(st; while_group_node["done"]),
            Some(while_group.clone()),
            vec![
                GuardExpr::Atom(st.to_atom(port!(st; cond_computed."out"))),
                GuardExpr::Not(st.to_atom(port!(st; cond_stored."out"))),
            ],
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
        let seq_group_node = st.insert_group(&seq_group)?;

        let fsm_reg = st.new_primitive(&ctx, "fsm", "std_reg", &[32])?;
        let num = st.new_constant(0, 32)?;
        st.insert_edge(
            num,
            (fsm_reg, st.port_ref(fsm_reg, "in")?.clone()),
            Some(seq_group.clone()),
            Vec::new(),
        )?;

        // Assigning 1 to tell groups to go.
        let signal_const = st.new_constant(1, 1)?;

        // Generate fsm to drive the sequence
        let mut fsm_counter = 0;
        for (idx, con) in s.stmts.iter().enumerate() {
            match con {
                Control::Enable {
                    data: Enable { comp: group_name },
                } => {
                    /* group[go] = fsm.out == value(fsm_counter) ? 1 */
                    let group = st
                        .get_node_by_name(&group_name)
                        .extract(group_name.clone())?;
                    let group_port = st.port_ref(group, "go")?.clone();

                    let fsm_out_port = st.port_ref(fsm_reg, "out")?.clone();
                    let fsm_st_const = st.new_constant(fsm_counter, 32)?;

                    let go_guard = GuardExpr::Eq(
                        st.to_atom((fsm_reg, fsm_out_port)),
                        st.to_atom(fsm_st_const.clone()),
                    );
                    st.insert_edge(
                        signal_const.clone(),
                        (group, group_port),
                        Some(seq_group.clone()),
                        vec![go_guard],
                    )?;

                    fsm_counter += 1;

                    /* fsm.in = group[done] ? 1 */
                    let (new_state_const, new_state_port) =
                        st.new_constant(fsm_counter, 32)?;
                    let group_done_port = st.port_ref(group, "done")?.clone();
                    let done_guard = GuardExpr::Atom(
                        st.to_atom((group, group_done_port.clone())),
                    );
                    st.insert_edge(
                        (new_state_const, new_state_port),
                        (fsm_reg, st.port_ref(fsm_reg, "in")?.clone()),
                        Some(seq_group.clone()),
                        vec![done_guard],
                    )?;

                    // If this is the last group, generate the done condition
                    // for the seq group.
                    if idx == s.stmts.len() - 1 {
                        let seq_group_done =
                            st.port_ref(seq_group_node, "done")?.clone();
                        st.insert_edge(
                            (group, group_done_port),
                            (seq_group_node, seq_group_done),
                            Some(seq_group.clone()),
                            Vec::new(),
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
        let new_control = Control::Enable {
            data: Enable { comp: seq_group },
        };
        Ok(Action::Change(new_control))
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
        let par_group_idx = st.insert_group(&par_group)?;

        let mut par_group_done: Vec<GuardExpr> = Vec::new();

        for con in s.stmts.iter() {
            match con {
                Control::Enable {
                    data: Enable { comp: group_name },
                } => {
                    let group_idx = st
                        .get_node_by_name(&group_name)
                        .extract(group_name.clone())?;
                    let group_go_port = st.port_ref(group_idx, "go")?.clone();
                    // Hook up this group's go signal with parent's
                    // go.
                    let num = st.new_constant(1, 1)?;
                    st.insert_edge(
                        num,
                        (group_idx, group_go_port),
                        Some(par_group.clone()),
                        Vec::new(),
                    )?;

                    // Add this group's done signal to parent's
                    // done signal.
                    let group_done_port =
                        st.port_ref(group_idx, "done")?.clone();
                    let guard = GuardExpr::Atom(
                        st.to_atom((group_idx, group_done_port)),
                    );
                    par_group_done.push(guard);
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
        let par_group_done_port = st.port_ref(par_group_idx, "done")?.clone();
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
