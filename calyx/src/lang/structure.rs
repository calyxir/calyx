use crate::errors;
use crate::lang::{ast, component};
use petgraph::dot::{Config, Dot};
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableDiGraph;
use petgraph::Direction;
use std::collections::HashMap;

/// store the structure ast node so that we can reconstruct the ast
#[derive(Clone, Debug)]
pub enum NodeData {
    Input(ast::Portdef),
    Output(ast::Portdef),
    Instance {
        name: ast::Id,
        structure: ast::Structure,
        signature: ast::Signature,
    },
}

impl NodeData {
    pub fn get_name(&self) -> &str {
        match self {
            NodeData::Input(pd) => &pd.name,
            NodeData::Output(pd) => &pd.name,
            NodeData::Instance { name, .. } => &name,
        }
    }
}

/// store the src port and dst port on edge
#[derive(Clone, Debug)]
pub struct EdgeData {
    pub src: String,
    pub dest: String,
    pub width: u64,
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
    pub graph: StructG,
}

impl StructureGraph {
    pub fn new() -> Self {
        StructureGraph {
            portdef_map: HashMap::new(),
            inst_map: HashMap::new(),
            graph: StructG::new(),
        }
    }

    // XXX(sam) bad name
    pub fn add_signature(&mut self, sig: &ast::Signature) {
        // add nodes for inputs and outputs
        for port in &sig.inputs {
            self.insert_io_port(port, NodeData::Input);
        }
        for port in &sig.outputs {
            self.insert_io_port(port, NodeData::Output);
        }
    }

    pub fn add_instance(
        &mut self,
        id: &ast::Id,
        comp: &component::Component,
        structure: ast::Structure,
    ) -> NodeIndex {
        let idx = self.graph.add_node(NodeData::Instance {
            name: id.clone(),
            structure,
            signature: comp.signature.clone(),
        });
        self.inst_map.insert(id.to_string(), idx);
        idx
    }

    // XXX(sam) this is a bad name
    pub fn add_component_def(
        &mut self,
        compdef: &ast::ComponentDef,
        comp_sigs: &HashMap<ast::Id, ast::Signature>,
        prim_sigs: &HashMap<ast::Id, ast::Signature>,
    ) -> Result<(), errors::Error> {
        self.add_signature(&compdef.signature);

        // add vertices first, ignoring wires so that order of structure doesn't matter
        for stmt in &compdef.structure {
            match stmt {
                ast::Structure::Decl { data } => {
                    let sig =
                        comp_sigs.get(&data.component).ok_or_else(|| {
                            errors::Error::SignatureResolutionFailed(
                                data.component.clone(),
                            )
                        })?;
                    let instance = NodeData::Instance {
                        name: data.name.clone(),
                        structure: stmt.clone(),
                        signature: sig.clone(),
                    };
                    self.inst_map.insert(
                        data.name.clone(),
                        self.graph.add_node(instance),
                    );
                }
                ast::Structure::Std { data } => {
                    // resolve param signature and add it to hashmap so that
                    //  we keep a reference to it
                    let sig = prim_sigs.get(&data.name).ok_or_else(|| {
                        errors::Error::SignatureResolutionFailed(
                            data.name.clone(),
                        )
                    })?;
                    let instance = NodeData::Instance {
                        name: data.name.clone(),
                        structure: stmt.clone(),
                        signature: sig.clone(),
                    };
                    self.inst_map.insert(
                        data.name.clone(),
                        self.graph.add_node(instance),
                    );
                }
                ast::Structure::Wire { .. } => (),
            }
        }

        // then add edges
        for stmt in &compdef.structure {
            if let ast::Structure::Wire { data } = stmt {
                use ast::Port::{Comp, This};

                // get src node in graph and src port
                let (src_node, src_port) = match &data.src {
                    Comp { component, port } => {
                        (self.inst_map.get(component), port)
                    }
                    This { port } => (self.portdef_map.get(port), port),
                };

                // get dest node in graph and dest port
                let (dest_node, dest_port) = match &data.dest {
                    Comp { component, port } => {
                        (self.inst_map.get(component), port)
                    }
                    This { port } => (self.portdef_map.get(port), port),
                };

                match (src_node, dest_node) {
                    // both nodes were found, this is a valid edge!
                    (Some(s), Some(d)) => {
                        // dereference s and d before their use because
                        // otherwise the self is borrowed with `self.insert_edge`
                        // before they are used
                        let (s, d) = (*s, *d);
                        self.insert_edge(s, src_port, d, dest_port)?;
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
        Ok(())
    }

    /// Returns an iterator over all the nodes in the structure graph
    pub fn instances(
        &self,
    ) -> impl Iterator<Item = (NodeIndex, NodeData)> + '_ {
        self.graph
            .node_indices()
            .map(move |ni| (ni, self.graph[ni].clone()))
    }

    fn connected_direction<'a>(
        &'a self,
        node: NodeIndex,
        port: String,
        direction: Direction,
    ) -> impl Iterator<Item = (&'a NodeData, &'a EdgeData)> + 'a {
        let edge_iter = self
            .graph
            .edges_directed(node, direction)
            .map(|e| e.weight());
        let node_iter = self
            .graph
            .neighbors_directed(node, direction)
            .map(move |idx| &self.graph[idx]);
        node_iter
            .zip(edge_iter)
            .filter_map(move |(nd, ed)| match direction {
                Direction::Incoming => {
                    if ed.dest == port {
                        Some((nd, ed))
                    } else {
                        None
                    }
                }
                Direction::Outgoing => {
                    if ed.src == port {
                        Some((nd, ed))
                    } else {
                        None
                    }
                }
            })
    }

    /// Returns an iterator over edges and destination nodes connected to `node` at `port`
    pub fn connected_to<'a>(
        &'a self,
        node: NodeIndex,
        port: String,
    ) -> impl Iterator<Item = (&'a NodeData, &'a EdgeData)> + 'a {
        self.connected_direction(node, port, Direction::Outgoing)
    }

    /// Returns an iterator over edges and src nodes connected to `node` at `port`
    pub fn connected_from<'a>(
        &'a self,
        node: NodeIndex,
        port: String,
    ) -> impl Iterator<Item = (&'a NodeData, &'a EdgeData)> + 'a {
        self.connected_direction(node, port, Direction::Incoming)
    }

    pub fn insert_input_port(&mut self, port: &ast::Portdef) {
        self.insert_io_port(&port, NodeData::Input)
    }

    pub fn insert_output_port(&mut self, port: &ast::Portdef) {
        self.insert_io_port(&port, NodeData::Output)
    }

    fn insert_io_port(
        &mut self,
        port: &ast::Portdef,
        constr: impl Fn(ast::Portdef) -> NodeData,
    ) {
        self.portdef_map.insert(
            port.name.clone(),
            self.graph.add_node(constr(port.clone())),
        );
    }

    /// Construct and insert an edge given two node indices
    pub fn insert_edge(
        &mut self,
        src_node: NodeIndex,
        src_port: &str,
        dest_node: NodeIndex,
        dest_port: &str,
    ) -> Result<(), errors::Error> {
        let find_width =
            |port_to_find: &str, portdefs: &[ast::Portdef]| match portdefs
                .iter()
                .find(|x| x.name == port_to_find)
            {
                Some(port) => Ok(port.width),
                None => {
                    Err(errors::Error::UndefinedPort(port_to_find.to_string()))
                }
            };

        use NodeData::{Input, Instance, Output};
        let src_width = match &self.graph[src_node] {
            Instance { signature, .. } => {
                find_width(src_port, &signature.outputs)
            }
            Input(portdef) => Ok(portdef.width),
            Output(_portdef) => {
                Err(errors::Error::UndefinedPort(src_port.to_string()))
            }
        }?;
        let dest_width = match &self.graph[dest_node] {
            Instance { signature, .. } => {
                find_width(dest_port, &signature.inputs)
            }
            Input(_portdef) => {
                Err(errors::Error::UndefinedPort(dest_port.to_string()))
            }
            Output(portdef) => Ok(portdef.width),
        }?;

        // if widths match, add edge to the graph
        if src_width == dest_width {
            let edge_data = EdgeData {
                src: src_port.into(),
                dest: dest_port.into(),
                width: src_width,
            };
            self.graph.add_edge(src_node, dest_node, edge_data);
            Ok(())
        } else {
            Err(errors::Error::MismatchedPortWidths(
                self.construct_port(src_node, src_port),
                src_width,
                self.construct_port(dest_node, dest_port),
                dest_width,
            ))
        }
    }

    pub fn get(&self, idx: NodeIndex) -> &NodeData {
        &self.graph[idx]
    }

    pub fn get_inst_index(
        &self,
        port: &ast::Id,
    ) -> Result<NodeIndex, errors::Error> {
        match self.inst_map.get(port) {
            Some(idx) => Ok(*idx),
            None => Err(errors::Error::UndefinedPort(port.to_string())),
        }
    }

    pub fn get_io_index(&self, port: &str) -> Result<NodeIndex, errors::Error> {
        match self.portdef_map.get(port) {
            Some(idx) => Ok(*idx),
            None => Err(errors::Error::UndefinedPort(port.to_string())),
        }
    }

    pub fn get_node_from_port(
        &self,
        port: &ast::Port,
    ) -> Result<NodeIndex, errors::Error> {
        match port {
            ast::Port::Comp { component, .. } => self.get_inst_index(component),
            ast::Port::This { port } => self.get_io_index(port),
        }
    }

    pub fn get_wire_width(
        &self,
        src_node: NodeIndex,
        src_port: &str,
        dest_node: NodeIndex,
        dest_port: &str,
    ) -> Result<u64, errors::Error> {
        let width = self.graph.find_edge(src_node, dest_node).and_then(|ei| {
            self.graph.edge_weight(ei).and_then(|edge| {
                if edge.src == src_port && edge.dest == dest_port {
                    Some(edge.width)
                } else {
                    None
                }
            })
        });

        match width {
            Some(w) => Ok(w),
            None => Err(errors::Error::UndefinedWire),
        }
    }

    fn construct_port(&self, idx: NodeIndex, port: &str) -> ast::Port {
        use ast::Port;
        use NodeData::*;
        match &self.graph[idx] {
            Input(portdef) => Port::This {
                port: portdef.name.clone(),
            },
            Output(portdef) => Port::This {
                port: portdef.name.clone(),
            },
            Instance { name, .. } => Port::Comp {
                component: name.to_string(),
                port: port.to_string(),
            },
        }
    }

    #[allow(unused)]
    pub fn visualize(&self) -> String {
        let config = &[Config::EdgeNoLabel];
        format!("{:?}", Dot::with_config(&self.graph, config))
    }
}

impl ast::Port {
    fn get_id(&self) -> &ast::Id {
        match self {
            ast::Port::Comp { component, .. } => component,
            ast::Port::This { port } => port,
        }
    }
}

// Implement conversion of graph back into a structure ast vector
impl Into<Vec<ast::Structure>> for StructureGraph {
    fn into(self) -> Vec<ast::Structure> {
        let mut ret: Vec<ast::Structure> = vec![];
        // add structure stmts for nodes
        for idx in self.inst_map.values() {
            if let NodeData::Instance { structure, .. } = &self.graph[*idx] {
                ret.push(structure.clone());
            }
        }

        // add wire structure stmts for edges
        for ed in self.graph.edge_indices() {
            if let Some((src, dest)) = self.graph.edge_endpoints(ed) {
                let src_port = self.construct_port(src, &self.graph[ed].src);
                let dest_port = self.construct_port(dest, &self.graph[ed].dest);
                ret.push(ast::Structure::wire(src_port, dest_port))
            }
        }

        ret
    }
}
