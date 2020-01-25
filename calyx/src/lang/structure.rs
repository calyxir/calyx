use crate::errors;
use crate::lang::ast;
use crate::lang::context::LibraryContext;
use petgraph::dot::{Config, Dot};
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableDiGraph;
use std::collections::HashMap;

/// store the structure ast node so that we can reconstruct the ast
#[derive(Clone, Debug)]
enum NodeData {
    Input(ast::Portdef),
    Output(ast::Portdef),
    Instance {
        structure: ast::Structure,
        signature: ast::Signature,
    },
}

/// store the src port and dst port on edge
#[derive(Clone, Debug)]
struct EdgeData {
    wire: ast::Wire,
    width: u32,
}

/// private graph type. the data in the node is the identifier
/// for the corresponding component, and the data on the edge
/// is (src port, dest port). Use stable graph so that NodeIndexes
/// remain valid after removals. the graph is directed
type StructG = StableDiGraph<NodeData, EdgeData>;

// I want to keep the fields of this struct private so that it is easy to swap
// out implementations / add new ways of manipulating this
/// Structure holds information about the structure of the current component
#[derive(Clone, Debug)]
pub struct StructureGraph {
    // portdef map separate from inst_map so that we don't have name clash between
    // port names and instance identifiers
    portdef_map: HashMap<String, NodeIndex>,
    inst_map: HashMap<ast::Id, NodeIndex>,
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

impl ast::ComponentDef {
    // Control the creation method of Structure
    pub fn structure_graph(
        &self,
        sigs: &HashMap<ast::Id, ast::Signature>,
        libctx: &LibraryContext,
    ) -> Result<StructureGraph, errors::Error> {
        let mut graph = StructG::new();
        let mut portdef_map = HashMap::new();
        let mut inst_map = HashMap::new();

        // add vertices for `inputs`
        for port in &self.signature.inputs {
            portdef_map.insert(
                port.name.clone(),
                graph.add_node(NodeData::Input(port.clone())),
            );
        }

        // add vertices for `outputs`
        for port in &self.signature.outputs {
            portdef_map.insert(
                port.name.clone(),
                graph.add_node(NodeData::Output(port.clone())),
            );
        }

        // add vertices first, ignoring wires so that order of structure doesn't matter
        for stmt in &self.structure {
            match stmt {
                ast::Structure::Decl { data } => {
                    let instance = NodeData::Instance {
                        structure: stmt.clone(),
                        signature: sigs
                            .get(&data.component)
                            .expect("Signature not found")
                            .clone(),
                    };
                    inst_map
                        .insert(data.name.clone(), graph.add_node(instance));
                }
                ast::Structure::Std { data } => {
                    let instance = NodeData::Instance {
                        structure: stmt.clone(),
                        signature: libctx
                            .resolve(&data.instance.name, &data.instance.params)
                            .expect("Primitive Signature failed to resolve"),
                    };
                    inst_map
                        .insert(data.name.clone(), graph.add_node(instance));
                }
                ast::Structure::Wire { .. } => (),
            }
        }

        // then add edges
        for stmt in &self.structure {
            if let ast::Structure::Wire { data } = stmt {
                use ast::Port::{Comp, This};

                let find_width =
                    |port_to_find: &str, portdefs: &[ast::Portdef]| {
                        match portdefs.iter().find(|x| &x.name == port_to_find)
                        {
                            Some(port) => Ok(port.width),
                            None => Err(errors::Error::UndefinedPort(
                                port_to_find.to_string(),
                            )),
                        }
                    };

                // get src node in graph and width of src port
                let (src_node, src_port) = match &data.src {
                    Comp { component, port } => (inst_map.get(component), port),
                    This { port } => (portdef_map.get(port), port),
                };

                // get dest node in graph and width of dest port
                let (dest_node, dest_port) = match &data.dest {
                    Comp { component, port } => (inst_map.get(component), port),
                    This { port } => (portdef_map.get(port), port),
                };

                match (src_node, dest_node) {
                    (Some(s), Some(d)) => {
                        let (src_width, dest_width) = if let (
                            NodeData::Instance {
                                signature: src_sig, ..
                            },
                            NodeData::Instance {
                                signature: dest_sig,
                                ..
                            },
                        ) =
                            (&graph[*s], &graph[*d])
                        {
                            let src_width =
                                find_width(src_port, &src_sig.outputs)?;
                            let dest_width =
                                find_width(dest_port, &dest_sig.inputs)?;
                            (src_width, dest_width)
                        } else {
                            let src_width =
                                find_width(src_port, &self.signature.outputs)?;
                            let dest_width =
                                find_width(dest_port, &self.signature.inputs)?;
                            (src_width, dest_width)
                        };

                        if src_width == dest_width {
                            let edge_data = EdgeData {
                                wire: data.clone(),
                                width: src_width as u32,
                            };
                            graph.add_edge(*s, *d, edge_data);
                        } else {
                            return Err(errors::Error::MismatchedPortWidths(
                                data.src.clone(),
                                src_width,
                                data.dest.clone(),
                                dest_width,
                            ));
                        }
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

        Ok(StructureGraph {
            portdef_map,
            inst_map,
            graph,
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
        // add structure stmts for nodes
        for (_, idx) in self.inst_map {
            if let NodeData::Instance { structure, .. } = &self.graph[idx] {
                ret.push(structure.clone());
            }
        }

        // add wire structure stmts for edges
        for ed in self.graph.edge_indices() {
            ret.push(ast::Structure::Wire {
                data: self.graph[ed].wire.clone(),
            })
        }

        ret
    }
}
