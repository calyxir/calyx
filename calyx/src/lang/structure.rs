use crate::lang::ast;
use petgraph::dot::{Config, Dot};
use petgraph::graph::{Graph, NodeIndex};
use std::collections::HashMap;

/// private graph type
type StructG = Graph<ast::Id, ()>;

// I want to keep the fields of this struct private so that it is easy to swap
// out implementations / add new ways of manipulating this
/** Structure holds information about the structure of the current component. */
#[derive(Clone, Debug)]
pub struct StructureGraph {
    node_hash: HashMap<ast::Id, NodeIndex>,
    graph: StructG,
}

impl ast::Port {
    fn get_id(&self) -> &ast::Id {
        match self {
            ast::Port::Comp { component, .. } => component,
            ast::Port::This { port } => port,
        }
    }
}

impl ast::Component {
    // Control the creation method of Structure
    pub fn structure_graph(&self) -> StructureGraph {
        let mut g = StructG::new();
        let mut node_hash = HashMap::new();

        // add vertices for inputs / outputs
        for port in self.inputs.iter().chain(self.outputs.iter()) {
            node_hash.insert(port.name.clone(), g.add_node(port.name.clone()));
        }

        // add vertices
        for stmt in &self.structure {
            match stmt {
                ast::Structure::Decl { data } => {
                    node_hash.insert(
                        data.name.clone(),
                        g.add_node(data.name.clone()),
                    );
                }
                ast::Structure::Std { data } => {
                    node_hash.insert(
                        data.name.clone(),
                        g.add_node(data.name.clone()),
                    );
                }
                ast::Structure::Wire { .. } => (),
            }
        }

        // add edges
        for stmt in &self.structure {
            match stmt {
                ast::Structure::Decl { .. } | ast::Structure::Std { .. } => (),
                ast::Structure::Wire { data } => {
                    match (
                        node_hash.get(data.src.get_id()),
                        node_hash.get(data.dest.get_id()),
                    ) {
                        (Some(s), Some(d)) => {
                            g.add_edge(*s, *d, ());
                        }
                        _ => {
                            panic!(
                                "Used an undeclared component in a connection while parsing {:?}: {:?} -> {:?}",
                                self.name,
                                data.src.get_id(),
                                data.dest.get_id()
                            )
                        },
                    }
                }
            }
        }

        StructureGraph {
            node_hash,
            graph: g,
        }
    }
}

impl StructureGraph {
    pub fn visualize(&self) -> String {
        let config = &[Config::EdgeNoLabel];
        format!("{:?}", Dot::with_config(&self.graph, config))
    }
}
