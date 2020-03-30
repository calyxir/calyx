use crate::errors;
use crate::lang::{
    ast, ast::Control, ast::Structure, component::Component, context::Context,
};
use crate::passes::visitor::{Action, VisResult, Visitor};
use crate::utils::NameGenerator;
use itertools::Itertools;
use petgraph::stable_graph::NodeIndex;

/// For Control of the form (top level seq with only enables as children):
/// ```lisp
/// (seq (enable A B)
///      (enable B C)
///       ...)
/// ```
/// this pass generates an FSM that simulates the behavior
/// of the `seq` in structure. It works by generating a `state`
/// component for each `enable` statement and generating
/// signals that connect to the `write_en` port of each register
/// in the `enable` sub-graph.
pub struct FsmSeq<'a> {
    names: &'a mut NameGenerator,
}

impl<'a> FsmSeq<'a> {
    pub fn new(names: &'a mut NameGenerator) -> Self {
        FsmSeq { names }
    }

    fn add_state_comp(
        &mut self,
        this_comp: &mut Component,
        ctx: &Context,
        regs: &[(NodeIndex, &ast::Id)],
    ) -> Result<(), errors::Error> {
        // construct new component from signature
        let sig = ast::Signature {
            inputs: vec![("valid", 1).into()],
            outputs: regs
                .iter()
                .map(|(_, id)| (id.as_ref(), 1).into())
                .collect(),
        };
        let mut new_comp =
            Component::from_signature(self.names.gen_name("fsm_expander"), sig);

        // instantiate new fsm comp in this comp
        let new_idx = this_comp.structure.add_instance(
            &new_comp.name,
            &new_comp,
            Structure::decl(new_comp.name.clone(), new_comp.name.clone()),
        );

        // add internal edges to new component
        let st = &mut new_comp.structure;
        let valid_idx = st.get_io_index("valid")?;
        for (idx, output) in regs.iter() {
            st.insert_edge(
                valid_idx,
                "valid",
                st.get_io_index(&output)?,
                &output,
            )?;

            this_comp
                .structure
                .insert_edge(new_idx, &output, *idx, "write_en")?;
        }

        // add the new component to the namespace
        ctx.insert_component(new_comp);

        Ok(())
    }
}

impl Visitor for FsmSeq<'_> {
    fn name(&self) -> String {
        "Fsm Seq".to_string()
    }

    fn finish_seq(
        &mut self,
        s: &mut ast::Seq,
        comp: &mut Component,
        ctx: &Context,
    ) -> VisResult {
        let mut enables: Vec<ast::Enable> = vec![];
        for stmt in &s.stmts {
            match stmt {
                Control::Enable { data } => enables.push(data.clone()),
                _ => return Ok(Action::Continue),
            }
        }

        let regs: Vec<_> = enables
            .iter()
            .map(|x| {
                x.comps
                    .iter()
                    .map(|c| {
                        let idx = comp.structure.get_inst_index(&c)?;
                        if comp.structure.graph[idx].get_component_type()?
                            == "std_reg"
                        {
                            Ok((idx, c))
                        } else {
                            Err(errors::Error::Misc(
                                "thing not found".to_string(),
                            ))
                        }
                    })
                    .filter_map(Result::ok)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        for reg in regs {
            self.add_state_comp(comp, ctx, &reg)?;
        }

        Ok(Action::Continue)
    }
}
