//use crate::errors::Error;
use crate::lang::component::Component;
use crate::lang::{
    ast,
    ast::Id,
    context::Context,
    //structure::NodeData,
    //structure_iter::NodeType,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitable, Visitor};
use petgraph::{graph::NodeIndex, Graph};
use std::collections::HashMap;

/// A pass that discovers parallelism using a FuTIL program's control.
/// It proceeds in several steps:
/// 1. Build a *conflict graph* where two groups have an edge between them
///    if they might be executed in parallel.
#[derive(Default)]
pub struct ResourceSharing {
    /// The overall conflict graph. Two group Ids have an edge between them
    /// if they might execute together.
    conflict_graph: Graph<Id, ()>,
    /// Represents set of groups observed in a given control statement.
    current_nodes: Vec<Id>,
    /// XXX(rachit): Do I have to store the NodeIndex? I could find a way to
    /// get the NodeIndex given an Id.
    index_map: HashMap<Id, NodeIndex>,
}

impl Named for ResourceSharing {
    fn name() -> &'static str {
        "resource-sharing"
    }

    fn description() -> &'static str {
        "Share resources when possible."
    }
}

impl Visitor for ResourceSharing {
    // When visiting an enable, add it to the current set of nodes.
    fn finish_enable(
        &mut self,
        st: &ast::Enable,
        _comp: &mut Component,
        _c: &Context,
    ) -> VisResult {
        // If the node has already been added to the graph, don't add it
        // again.
        if !self.index_map.contains_key(&st.comp) {
            let node_index = self.conflict_graph.add_node(st.comp.clone());
            self.index_map.insert(st.comp.clone(), node_index);
        }
        self.current_nodes.push(st.comp.clone());
        Ok(Action::Continue)
    }

    // For each `par`, collect the set of nodes recursively present in the control
    // program in each statement and create conflict edges.
    fn start_par(
        &mut self,
        st: &ast::Par,
        comp: &mut Component,
        c: &Context,
    ) -> VisResult {
        let mut conflicting_groups: Vec<Vec<Id>> = Vec::new();
        // Save the set of groups seen up till this point.
        let cur_groups: Vec<Id> = self.current_nodes.drain(..).collect();

        for stmt in &st.stmts {
            // Clear the current set of nodes.
            self.current_nodes = Vec::new();

            // Visit the statements to collect the set of groups mentioned in
            // sub program.
            stmt.visit_immutable(self, comp, c)?;

            conflicting_groups.push(self.current_nodes.drain(..).collect());
        }

        for g1 in 0..conflicting_groups.len() {
            for g2 in g1 + 1..conflicting_groups.len() {
                for i in &conflicting_groups[g1] {
                    for j in &conflicting_groups[g2] {
                        self.conflict_graph.add_edge(
                            self.index_map[i],
                            self.index_map[j],
                            (),
                        );
                    }
                }
            }
        }

        // Recover the set of groups seen and add all the groups seen in this
        // sub-program.
        self.current_nodes = cur_groups;
        conflicting_groups
            .drain(..)
            .for_each(|mut g| self.current_nodes.append(&mut g));

        // XXX(rachit): This re-traverses the whole tree!
        Ok(Action::Continue)
    }

    fn finish(&mut self, _c: &mut Component, _ctx: &Context) -> VisResult {
        println!("{:#?}", self.conflict_graph);
        Ok(Action::Continue)
    }
}
