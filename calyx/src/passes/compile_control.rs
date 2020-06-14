//use crate::errors;
use crate::lang::component::Component;
use crate::lang::{ast, context::Context};
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
        //use ast::*;
        let st = &mut comp.structure;
        // Create a register to store the FSM state.
        let fsm_name = st.namegen.gen_name("fsm");
        let fsm_prim = ctx.instantiate_primitive(
            fsm_name.clone(),
            &"std_reg".into(),
            &[32],
        )?;
        st.add_primitive(
            fsm_name.clone().into(),
            "std_reg",
            &fsm_prim,
            &[32],
        );

        // Create a new group for the seq related structure.
        let seq_name = st.namegen.gen_name("seq");
        st.insert_group(seq_name.into())?;

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
