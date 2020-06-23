use crate::errors::Error;
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

impl Visitor for CompileControl {
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

        let cond_group = cif.cond.as_ref().expect("Unimplemented");
        let cond_group_node = st.get_node_by_name(cond_group).unwrap();
        let (cond_node, cond_port) = match &cif.port {
            Port::Comp { component, port } => {
                Ok((st.get_node_by_name(component).unwrap(), port))
            }
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
        let true_group_node = st.get_node_by_name(true_group).unwrap();
        let false_group_node = st.get_node_by_name(false_group).unwrap();

        // generate necessary hardware
        let cond_computed_reg =
            st.new_primitive(&ctx, "cond_computed", "std_reg", &[1])?;
        let cond_stored_reg =
            st.new_primitive(&ctx, "cond_stored", "std_reg", &[1])?;
        let branch_or = st.new_primitive(&ctx, "if_or", "std_or", &[1])?;

        // cond_computed.in = this[go] & cond[done] ? 1'b1;
        let cond_computed_guard =
            GuardExpr::Atom(st.to_atom(cond_group_node, "done".into()));
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            (
                cond_computed_reg,
                st.port_ref(cond_computed_reg, "in")?.clone(),
            ),
            Some(if_group.clone()),
            vec![cond_computed_guard],
        )?;

        // cond_stored.in = cond[done] ? comp.out;
        let cond_stored_guard =
            GuardExpr::Atom(st.to_atom(cond_group_node, "done".into()));
        st.insert_edge(
            (cond_node, cond_port.clone()),
            (cond_stored_reg, st.port_ref(cond_stored_reg, "in")?.clone()),
            Some(if_group.clone()),
            vec![cond_stored_guard],
        )?;

        // cond[go] = !cond_computed.out ? 1'b1
        let cond_go_guard =
            GuardExpr::Not(st.to_atom(cond_computed_reg, "out".into()));
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            (cond_group_node, st.port_ref(cond_group_node, "go")?.clone()),
            Some(if_group.clone()),
            vec![cond_go_guard],
        )?;

        // tbranch[go] = cond_computed.out & cond_stored.out ? 1'b1;
        let tbranch_guard =
            GuardExpr::Atom(st.to_atom(cond_stored_reg, "out".into()));
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            (true_group_node, st.port_ref(true_group_node, "go")?.clone()),
            Some(if_group.clone()),
            vec![tbranch_guard],
        )?;

        // fbranch[go] = cond_computed.out & !cond_stored.out ? 1'b1;
        let fbranch_guard =
            GuardExpr::Not(st.to_atom(cond_stored_reg, "out".into()));
        let num = st.new_constant(1, 1)?;
        st.insert_edge(
            num,
            (
                false_group_node,
                st.port_ref(false_group_node, "go")?.clone(),
            ),
            Some(if_group.clone()),
            vec![fbranch_guard],
        )?;

        // or.right = true[done];
        st.insert_edge(
            (
                true_group_node,
                st.port_ref(true_group_node, "done")?.clone(),
            ),
            (branch_or, st.port_ref(branch_or, "left")?.clone()),
            Some(if_group.clone()),
            vec![],
        )?;

        // or.left = false[done];
        st.insert_edge(
            (
                false_group_node,
                st.port_ref(false_group_node, "done")?.clone(),
            ),
            (branch_or, st.port_ref(branch_or, "right")?.clone()),
            Some(if_group.clone()),
            vec![],
        )?;

        // this[done] = or.out;
        st.insert_edge(
            (branch_or, st.port_ref(branch_or, "out")?.clone()),
            (if_group_node, st.port_ref(if_group_node, "done")?.clone()),
            Some(if_group.clone()),
            vec![],
        )?;

        Ok(Action::Change(Control::enable(if_group)))
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
                    let group = st.get_node_by_name(&group_name)
                        .expect("Malformed AST. Group referenced in control is missing from structure");
                    let group_port = st.port_ref(group, "go")?.clone();

                    let fsm_out_port = st.port_ref(fsm_reg, "out")?.clone();
                    let fsm_st_const = st.new_constant(fsm_counter, 32)?;

                    let go_guard = GuardExpr::Eq(
                        st.to_atom(fsm_reg, fsm_out_port),
                        st.to_atom(fsm_st_const.0, fsm_st_const.1),
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
                    let done_guard = GuardExpr::Eq(
                        st.to_atom(group, group_done_port.clone()),
                        st.to_atom(signal_const.0, signal_const.1.clone()),
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
        let par_group_go_port = st.port_ref(par_group_idx, "go")?.clone();

        let mut par_group_done: Vec<GuardExpr> = Vec::new();

        for con in s.stmts.iter() {
            match con {
                Control::Enable {
                    data: Enable { comp: group_name },
                } => {
                    let group_idx = st.get_node_by_name(&group_name).expect(
                        "Malformed structure: Group node is not defined.",
                    );
                    let group_go_port = st.port_ref(group_idx, "go")?.clone();
                    // Hook up this group's go signal with parent's
                    // go.
                    st.insert_edge(
                        (par_group_idx, par_group_go_port.clone()),
                        (group_idx, group_go_port),
                        Some(par_group.clone()),
                        Vec::new(),
                    )?;

                    // Add this group's done signal to parent's
                    // done signal.
                    let group_done_port =
                        st.port_ref(group_idx, "done")?.clone();
                    let guard =
                        GuardExpr::Atom(st.to_atom(group_idx, group_done_port));
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
