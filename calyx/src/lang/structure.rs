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
    width: u64,
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
    pub fn resolve_primitives(
        &self,
        libctx: &LibraryContext,
    ) -> Result<HashMap<ast::Id, ast::Signature>, errors::Error> {
        let mut map = HashMap::new();

        for stmt in &self.structure {
            if let ast::Structure::Std { data } = stmt {
                let sig = libctx
                    .resolve(&data.instance.name, &data.instance.params)?;
                map.insert(data.name.clone(), sig);
            }
        }

        Ok(map)
    }

    /// Insert nodes for input and output ports of self
    fn insert_io_nodes(
        &self,
        graph: &mut StructG,
        map: &mut HashMap<ast::Id, NodeIndex>,
    ) {
        // add vertices for `inputs`
        for port in &self.signature.inputs {
            map.insert(
                port.name.clone(),
                graph.add_node(NodeData::Input(port.clone())),
            );
        }

        // add vertices for `outputs`
        for port in &self.signature.outputs {
            map.insert(
                port.name.clone(),
                graph.add_node(NodeData::Output(port.clone())),
            );
        }
    }

    /// Construct and insert an edge given two node indices
    fn insert_edge(
        &self,
        graph: &mut StructG,
        wire: ast::Wire,
        src_node: NodeIndex,
        src_port: &str,
        dest_node: NodeIndex,
        dest_port: &str,
    ) -> Result<(), errors::Error> {
        let find_width =
            |port_to_find: &str, portdefs: &[ast::Portdef]| match portdefs
                .iter()
                .find(|x| &x.name == port_to_find)
            {
                Some(port) => Ok(port.width),
                None => {
                    Err(errors::Error::UndefinedPort(port_to_find.to_string()))
                }
            };

        // get width of src and dest ports by looking up signature in node
        let (src_width, dest_width) = if let (
            NodeData::Instance {
                signature: src_sig, ..
            },
            NodeData::Instance {
                signature: dest_sig,
                ..
            },
        ) =
            (&graph[src_node], &graph[dest_node])
        {
            let src_width = find_width(src_port, &src_sig.outputs)?;
            let dest_width = find_width(dest_port, &dest_sig.inputs)?;
            (src_width, dest_width)
        } else {
            let src_width = find_width(src_port, &self.signature.outputs)?;
            let dest_width = find_width(dest_port, &self.signature.inputs)?;
            (src_width, dest_width)
        };

        // if widths match, add edge to the graph
        if src_width == dest_width {
            let edge_data = EdgeData {
                wire: wire,
                width: src_width,
            };
            graph.add_edge(src_node, dest_node, edge_data);
            Ok(())
        } else {
            Err(errors::Error::MismatchedPortWidths(
                wire.src.clone(),
                src_width,
                wire.dest.clone(),
                dest_width,
            ))
        }
    }

    // Control the creation method of Structure
    pub fn structure_graph<'a>(
        &self,
        comp_sigs: &HashMap<ast::Id, ast::Signature>,
        prim_sigs: &HashMap<ast::Id, ast::Signature>,
    ) -> Result<StructureGraph, errors::Error> {
        let mut portdef_map: HashMap<String, NodeIndex> = HashMap::new();
        let mut inst_map: HashMap<ast::Id, NodeIndex> = HashMap::new();
        let mut graph = StructG::new();

        self.insert_io_nodes(&mut graph, &mut portdef_map);

        // add vertices first, ignoring wires so that order of structure doesn't matter
        for stmt in &self.structure {
            match stmt {
                ast::Structure::Decl { data } => {
                    let sig = comp_sigs
                        .get(&data.component)
                        .expect("Signature not found");
                    let instance = NodeData::Instance {
                        structure: stmt.clone(),
                        signature: sig.clone(),
                    };
                    inst_map
                        .insert(data.name.clone(), graph.add_node(instance));
                }
                ast::Structure::Std { data } => {
                    // resolve param signature and add it to hashmap so that
                    //  we keep a reference to it
                    let sig =
                        prim_sigs.get(&data.name).expect("Signature not found");
                    let instance = NodeData::Instance {
                        structure: stmt.clone(),
                        signature: sig.clone(),
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

                // get src node in graph and src port
                let (src_node, src_port) = match &data.src {
                    Comp { component, port } => (inst_map.get(component), port),
                    This { port } => (portdef_map.get(port), port),
                };

                // get dest node in graph and dest port
                let (dest_node, dest_port) = match &data.dest {
                    Comp { component, port } => (inst_map.get(component), port),
                    This { port } => (portdef_map.get(port), port),
                };

                match (src_node, dest_node) {
                    // both nodes were found, this is a valid edge!
                    (Some(s), Some(d)) => {
                        self.insert_edge(
                            &mut graph,
                            data.clone(),
                            *s,
                            src_port,
                            *d,
                            dest_port,
                        )?;
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
