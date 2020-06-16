use crate::errors::Error;
use crate::lang::{
    ast, component::Component, context::Context, structure_builder::ASTBuilder,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use ast::{Control, Enable, GuardExpr};

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
    fn finish_seq(
        &mut self,
        s: &ast::Seq,
        comp: &mut Component,
        ctx: &Context,
    ) -> VisResult {
        let st = &mut comp.structure;

        // Create a new group for the seq related structure.
        let seq_group: ast::Id = st.namegen.gen_name("seq").into();
        st.insert_group(&seq_group)?;

        let fsm_reg = st.new_primitive(&ctx, "fsm", "std_reg", &[32])?;
        let fsm_in_port = st.port_ref(&fsm_reg, "in")?.clone();
        let (num, num_port) = st.new_constant(0, 32)?;
        st.insert_edge(
            (num, &num_port),
            (fsm_reg, &fsm_in_port),
            Some(seq_group.clone()),
            Vec::new(),
        )?;

        // Assigning 1 to tell groups to go.
        let (signal_const, signal_const_port) = st.new_constant(1, 1)?;

        let mut fsm_counter = 0;
        for con in &s.stmts {
            match con {
                Control::Enable {
                    data: Enable { comp: group_name },
                } => {
                    /* group[go] = fsm.out == value(fsm_counter) ? 1 */
                    let group = *st.get_node_by_name(&group_name)
                        .expect("Malformed AST. Group referenced in control is missing from structure");
                    let group_port = st.port_ref(&group, "go")?.clone();

                    let fsm_out_port = st.port_ref(&fsm_reg, "out")?.clone();
                    let fsm_st_const = st.new_constant(fsm_counter, 32)?;

                    let go_guard = GuardExpr::Eq(
                        st.to_atom(&fsm_reg, fsm_out_port),
                        st.to_atom(&fsm_st_const.0, fsm_st_const.1),
                    );
                    st.insert_edge(
                        (signal_const, &signal_const_port),
                        (group, &group_port),
                        Some(seq_group.clone()),
                        vec![go_guard],
                    )?;

                    fsm_counter += 1;

                    /* fsm.in = group[done] ? 1 */
                    let (new_state_const, new_state_port) =
                        st.new_constant(fsm_counter, 32)?;
                    let group_done_port = st.port_ref(&group, "done")?.clone();
                    let done_guard = GuardExpr::Eq(
                        st.to_atom(&group, group_done_port),
                        st.to_atom(&signal_const, signal_const_port.clone()),
                    );
                    st.insert_edge(
                        (new_state_const, &new_state_port),
                        (fsm_reg, &fsm_in_port),
                        Some(seq_group.clone()),
                        vec![done_guard],
                    )?;
                }
                _ => {
                    return Err(Error::MalformedControl(
                        "Cannot compile non-group statement inside sequence"
                            .to_string(),
                    ))
                }
            }
        }
        Ok(Action::Continue)
    }
}
