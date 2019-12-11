use super::machine::FSM;
use petgraph::dot::{Config, Dot};
use petgraph::graph::Graph;
use std::collections::HashMap;

impl FSM {
    fn graph(&self) -> Graph<Vec<String>, Vec<String>> {
        let mut graph: Graph<Vec<String>, Vec<String>> = Graph::new();

        let mut nodes = HashMap::new();
        for (src, state) in &self.states {
            nodes.insert(
                src,
                graph.add_node(
                    state
                        .outputs
                        .iter()
                        .map(|(_, port, _)| port.clone())
                        .collect(),
                ),
            );
        }

        for (src, state) in &self.states {
            for (cond, dst) in &state.transitions {
                graph.add_edge(
                    *nodes.get(src).unwrap(),
                    *nodes.get(dst).unwrap(),
                    cond.iter().map(|(_, port, _)| port.clone()).collect(),
                );
            }
        }

        graph
    }

    pub fn visualize(&self) {
        let config = &[];
        println!("{:?}", Dot::with_config(&self.graph(), config))
    }
}
