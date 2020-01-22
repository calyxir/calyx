use crate::errors;
use crate::lang::ast;
use petgraph::dot::{Config, Dot};
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableGraph;
use std::collections::HashMap;

/// store the structure ast node so that we can reconstruct
/// the ast if we need to
type NodeData = Option<ast::Structure>;

/// store the src port and dst port on edge
type EdgeData = (ast::Port, ast::Port);

/// private graph type. the data in the node is the identifier
/// for the corresponding component, and the data on the edge
/// is (src port, dest port)
type StructG = StableGraph<NodeData, EdgeData>;

// I want to keep the fields of this struct private so that it is easy to swap
// out implementations / add new ways of manipulating this
/// Structure holds information about the structure of the current component
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
    pub fn structure_graph(&self) -> Result<StructureGraph, errors::Error> {
        let mut g = StructG::new();
        let mut node_hash = HashMap::new();

        // add vertices for `inputs ++ outputs`
        for port in self.inputs.iter().chain(self.outputs.iter()) {
            node_hash.insert(port.name.clone(), g.add_node(None));
        }

        // add vertices first, ignoring wires so that order of structure doesn't matter
        for stmt in &self.structure {
            match stmt {
                ast::Structure::Decl { data } => {
                    node_hash.insert(
                        data.name.clone(),
                        g.add_node(Some(stmt.clone())),
                    );
                }
                ast::Structure::Std { data } => {
                    node_hash.insert(
                        data.name.clone(),
                        g.add_node(Some(stmt.clone())),
                    );
                }
                ast::Structure::Wire { .. } => (),
            }
        }

        // then add edges
        for stmt in &self.structure {
            match stmt {
                ast::Structure::Decl { .. } | ast::Structure::Std { .. } => (),
                ast::Structure::Wire { data } => {
                    match (
                        node_hash.get(data.src.get_id()),
                        node_hash.get(data.dest.get_id()),
                    ) {
                        (Some(s), Some(d)) => {
                            g.add_edge(
                                *s,
                                *d,
                                (data.src.clone(), data.dest.clone()),
                            );
                        }
                        // dest not found
                        (Some(_), None) => {
                            return Err(errors::Error::UndefinedComponent(
                                data.dest.get_id().clone(),
                            ));
                        }
                        // either source or dest not found, report src as error
                        _ => {
                            return Err(errors::Error::UndefinedComponent(
                                data.src.get_id().clone(),
                            ))
                        }
                    }
                }
            }
        }

        Ok(StructureGraph {
            node_hash,
            graph: g,
        })
    }
}

impl StructureGraph {
    pub fn visualize(&self) -> String {
        let config = &[Config::EdgeNoLabel];
        format!("{:?}", Dot::with_config(&self.graph, config))
    }
}

// Implement conversion of graph back into a structure ast vector
impl Into<Vec<ast::Structure>> for StructureGraph {
    fn into(self) -> Vec<ast::Structure> {
        let mut ret: Vec<ast::Structure> = vec![];
        for (_, idx) in self.node_hash {
            if let Some(st) = &self.graph[idx] {
                ret.push(st.clone());
            }
        }
        ret
    }
}
