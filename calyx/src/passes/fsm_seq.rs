use crate::errors;
use crate::lang::{
    ast, ast::Control, ast::Structure, component::Component, context::Context,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
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

    /// Helper for constructing `fsm_expander` components.
    fn add_state_expander(
        &mut self,
        this_comp: &mut Component,
        ctx: &Context,
        regs: &[(NodeIndex, &ast::Id)],
    ) -> Result<(NodeIndex, String), errors::Error> {
        // construct new component from signature
        let sig = ast::Signature {
            inputs: vec![("valid", 1).into(), ("clk", 1).into()],
            outputs: regs
                .iter()
                .map(|(_, id)| (id.as_ref(), 1).into())
                .collect(),
        };
        let name = self.names.gen_name("fsm_expander");
        let mut new_comp = Component::from_signature(name.clone(), sig);

        // instantiate new fsm comp in this comp
        let new_idx = this_comp.structure.add_subcomponent(
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

        Ok((new_idx, name))
    }

    /// Helper for constructing `fsm_expander` components.
    fn add_fsm_state(
        &mut self,
        this_comp: &mut Component,
        ctx: &Context,
        expander: NodeIndex,
    ) -> Result<(NodeIndex, String), errors::Error> {
        // construct fsm state primitive
        let name = self.names.gen_name("fsm_state");
        let fsm_state_prim =
            ctx.instantiate_primitive(&name, &"std_fsm_state".into(), &[])?;
        let prim_idx = this_comp.structure.add_primitive(
            &name.clone().into(),
            "std_fsm_state",
            &fsm_state_prim,
            &[],
        );

        // add edges from state to component
        this_comp
            .structure
            .insert_edge(prim_idx, "out", expander, "valid")?;

        Ok((prim_idx, name))
    }
}

impl Named for FsmSeq<'_> {
    fn name() -> &'static str {
        "fsm-seq"
    }

    fn description() -> &'static str {
        "Generates an FSM for a seq of enables"
    }
}

impl Visitor for FsmSeq<'_> {
    fn finish_seq(
        &mut self,
        s: &ast::Seq,
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

        // much hacky
        let regs: Vec<_> = enables
            .iter()
            .map(|x| {
                x.comps
                    .iter()
                    .map(|c| {
                        let idx = comp.structure.get_inst_index(&c)?;
                        if comp.structure.graph[idx].get_component_type()?
                            == "std_reg"
                            && comp
                                .structure
                                .connected_incoming(idx, "in".to_string())
                                .any(|(r, _)| x.comps.contains(r.get_name()))
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

        // construct fsm start state
        let name = self.names.gen_name("start_fsm");
        let start_prim =
            ctx.instantiate_primitive(&name, &"std_start_fsm".into(), &[])?;
        let mut prev_idx = comp.structure.add_primitive(
            &name.into(),
            "std_start_fsm",
            &start_prim,
            &[],
        );

        let val = comp.structure.get_io_index("valid")?;
        comp.structure
            .insert_edge(val, "valid", prev_idx, "valid")?;

        let mut enable_names = vec![];

        for reg in regs.iter() {
            let (expander, ex_name) =
                self.add_state_expander(comp, ctx, &reg)?;
            let (state, st_name) = self.add_fsm_state(comp, ctx, expander)?;

            enable_names.push(ex_name);
            enable_names.push(st_name);

            // add edge from prev_idx to new state
            comp.structure.insert_edge(prev_idx, "out", state, "in")?;
            prev_idx = state;
        }

        let end_state_name = self.names.gen_name("end_state");
        let end_state = ctx.instantiate_primitive(
            &end_state_name,
            &"std_fsm_state".into(),
            &[],
        )?;
        let end_idx = comp.structure.add_primitive(
            &end_state_name.into(),
            "std_fsm_state",
            &end_state,
            &[],
        );
        comp.structure.insert_edge(prev_idx, "out", end_idx, "in")?;
        comp.structure.insert_edge(
            end_idx,
            "out",
            comp.structure.get_io_index("ready")?,
            "ready",
        )?;

        Ok(Action::Change(Control::enable(
            enable_names
                .into_iter()
                .map(|x| x.into())
                .chain(regs.into_iter().flatten().map(|(_, x)| x.clone()))
                .unique()
                .collect(),
        )))
    }
}
