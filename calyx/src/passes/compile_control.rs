//use crate::errors;
use crate::lang::{
    ast, component::Component, context::Context, structure_builder::ASTBuilder,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};

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
        _s: &ast::Seq,
        comp: &mut Component,
        ctx: &Context,
    ) -> VisResult {
        let st = &mut comp.structure;

        // Create a new group for the seq related structure.
        let seq_group: ast::Id = st.namegen.gen_name("seq").into();
        st.insert_group(&seq_group)?;

        let reg = st.new_primitive(&ctx, "fsm", "std_reg", &[32])?;
        let reg_port = st.port_ref(&reg, "in")?.clone();
        let init_st = st.new_constant(0, 32)?;
        st.insert_edge(
            (init_st.0, &init_st.1),
            (reg, &reg_port),
            Some(seq_group),
            Vec::new()
        )?;
        /*
        let guard = ast::GuardExpr::Eq(
            st.to_atom(reg, reg_port),
            st.to_atom(reg, reg_port),
        )
        */


        //let fsm_reg = st.new

        // Initial state of the FSM is 0
        //let init_val = structure::Node::new_constant(&mut st.namegen, zero);

        /*for con in &s.stmts {
            match con {
                ast::Control::Enable { data } => {}
                _ => {
                    return Err(Error::MalformedControl(
                        "Cannot compile non-group statement inside sequence"
                            .to_string(),
                    ))
                }
            }
        }*/
        Ok(Action::Continue)
    }
}
