use crate::errors;
use crate::lang::{ast, component};
use ast::{Port, Structure};
use component::Component;
use errors::Error;
use petgraph::dot::{Config, Dot};
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableDiGraph;
use petgraph::Direction;
use std::cmp::Ordering;
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

pub struct PortIter {
    items: Vec<ast::Portdef>,
}

impl Iterator for PortIter {
    type Item = ast::Id;

    fn next(&mut self) -> Option<Self::Item> {
        match self.items.len().cmp(&1) {
            Ordering::Greater | Ordering::Equal => {
                let ret = Some(self.items[0].name.clone());
                self.items = (&self.items[1..]).to_vec();
                ret
            }
            Ordering::Less => None,
        }
    }
}

impl NodeData {
    pub fn get_name(&self) -> &ast::Id {
        match self {
            NodeData::Input(pd) => &pd.name,
            NodeData::Output(pd) => &pd.name,
            NodeData::Instance { name, .. } => &name,
        }
    }

    pub fn get_component_type(&self) -> Result<&ast::Id, errors::Error> {
        match self {
            NodeData::Input { .. } | NodeData::Output { .. } => {
                Err(errors::Error::NotSubcomponent)
            }
            NodeData::Instance { structure, .. } => match structure {
                Structure::Wire { .. } => Err(Error::Impossible(
                    "There should be no wires in nodes".to_string(),
                )),
                Structure::Std { data } => Ok(&data.instance.name),
                Structure::Decl { data } => Ok(&data.component),
            },
        }
    }

    pub fn out_ports(&self) -> PortIter {
        match self {
            NodeData::Input(pd) => PortIter {
                items: vec![pd.clone()],
            },
            NodeData::Output(pd) => PortIter {
                items: vec![pd.clone()],
            },
            NodeData::Instance { signature, .. } => PortIter {
                items: signature.outputs.clone(),
            },
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
    portdef_map: HashMap<ast::Id, NodeIndex>,
    inst_map: HashMap<ast::Id, NodeIndex>,
    pub graph: StructG,
}

impl Default for StructureGraph {
    fn default() -> Self {
        StructureGraph {
            portdef_map: HashMap::new(),
            inst_map: HashMap::new(),
            graph: StructG::new(),
        }
    }
}

impl StructureGraph {
    /* ============= Constructor Functions ============= */

    /// Creates a new structure graph from a component definition
    ///
    /// # Arguments
    ///   * `compdef` - the component definition
    ///   * `comp_sigs` - map of component signatures
    ///   * `prim_sigs` - map of primitive component signatures
    pub fn new(
        compdef: &ast::ComponentDef,
        comp_sigs: &HashMap<ast::Id, ast::Signature>,
        prim_sigs: &HashMap<ast::Id, ast::Signature>,
    ) -> Result<Self, Error> {
        let mut structure = StructureGraph::default();

        structure.add_signature(&compdef.signature);

        // add vertices first, ignoring wires so that order of structure
        // doesn't matter
        for stmt in &compdef.structure {
            match stmt {
                ast::Structure::Decl { data } => {
                    let sig =
                        comp_sigs.get(&data.component).ok_or_else(|| {
                            Error::SignatureResolutionFailed(
                                data.component.clone(),
                            )
                        })?;
                    let instance = NodeData::Instance {
                        name: data.name.clone(),
                        structure: stmt.clone(),
                        signature: sig.clone(),
                    };
                    structure.inst_map.insert(
                        data.name.clone(),
                        structure.graph.add_node(instance),
                    );
                }
                ast::Structure::Std { data } => {
                    // resolve param signature and add it to hashmap so that
                    //  we keep a reference to it
                    let sig = prim_sigs.get(&data.name).ok_or_else(|| {
                        Error::SignatureResolutionFailed(data.name.clone())
                    })?;
                    let instance = NodeData::Instance {
                        name: data.name.clone(),
                        structure: stmt.clone(),
                        signature: sig.clone(),
                    };
                    structure.inst_map.insert(
                        data.name.clone(),
                        structure.graph.add_node(instance),
                    );
                }
                ast::Structure::Wire { .. } => (),
            }
        }

        // then add edges
        for stmt in &compdef.structure {
            if let ast::Structure::Wire { data } = stmt {
                // get src node in graph and src port
                let (src_node, src_port) = match &data.src {
                    Port::Comp { component, port } => {
                        (structure.inst_map.get(component), port)
                    }
                    Port::This { port } => {
                        (structure.portdef_map.get(port), port)
                    }
                };

                // get dest node in graph and dest port
                let (dest_node, dest_port) = match &data.dest {
                    Port::Comp { component, port } => {
                        (structure.inst_map.get(component), port)
                    }
                    Port::This { port } => {
                        (structure.portdef_map.get(port), port)
                    }
                };

                match (src_node, dest_node) {
                    // both nodes were found, this is a valid edge!
                    (Some(&s), Some(&d)) => {
                        structure.insert_edge(s, src_port, d, dest_port)?;
                    }
                    // dest not found
                    (Some(_), None) => {
                        return Err(Error::UndefinedComponent(
                            data.dest.port_name().clone(),
                        ));
                    }
                    // either source or dest not found, report src as error
                    _ => {
                        return Err(Error::UndefinedComponent(
                            data.src.port_name().clone(),
                        ))
                    }
                }
            }
        }
        Ok(structure)
    }

    /// Adds nodes for input and output ports to the structure graph.
    /// Input/output ports are defined in the component signature.
    ///
    /// # Arguments
    ///   * `sig` - the signature for the component
    pub fn add_signature(&mut self, sig: &ast::Signature) {
        // add nodes for inputs and outputs
        for port in &sig.inputs {
            self.insert_io_port(port, NodeData::Input);
        }
        for port in &sig.outputs {
            self.insert_io_port(port, NodeData::Output);
        }
    }

    /// Adds a subcomponent node to the structure graph.
    ///
    /// # Arguments
    ///   * `id` - the subcomponent identifier
    ///   * `comp` - the component object
    ///   * `structure` - the AST structure of the subcomponent
    pub fn add_subcomponent(
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
        self.inst_map.insert(id.clone(), idx);
        idx
    }

    /// Adds a primitive component node to the structure graph.
    /// XXX(ken): Perhaps change this to allow implicit conversion
    /// to generate the primitive compinst?
    ///
    /// # Arguments
    ///   * `id` - the subcomponent identifier
    ///   * `name` - the subcomponent type
    ///   * `comp` - the component object
    ///   * `params` - the parameters for the component instance
    ///   * `structure` - the AST structure of the subcomponent
    pub fn add_primitive<S: AsRef<str>>(
        &mut self,
        id: &ast::Id,
        name: S,
        comp: &Component,
        params: &[u64],
    ) -> NodeIndex {
        let structure = ast::Structure::std(
            id.clone(),
            ast::Compinst {
                name: name.as_ref().into(),
                params: params.to_vec(),
            },
        );
        self.add_subcomponent(id, comp, structure)
    }
    /* ============= Helper Methods ============= */

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
    pub fn insert_edge<S: AsRef<str>, U: AsRef<str>>(
        &mut self,
        src_node: NodeIndex,
        src_port: S,
        dest_node: NodeIndex,
        dest_port: U,
    ) -> Result<(), Error> {
        let src_port: &str = src_port.as_ref();
        let dest_port: &str = dest_port.as_ref();

        let find_width =
            |port_to_find: &str, portdefs: &[ast::Portdef]| match portdefs
                .iter()
                .find(|x| &x.name == port_to_find)
            {
                Some(port) => Ok(port.width),
                None => Err(Error::UndefinedPort(port_to_find.to_string())),
            };

        use NodeData::{Input, Instance, Output};
        let src_width = match &self.graph[src_node] {
            Instance { signature, .. } => {
                find_width(src_port, &signature.outputs)
            }
            Input(portdef) => Ok(portdef.width),
            Output(_) => Err(Error::UndefinedPort(src_port.to_string())),
        }?;
        let dest_width = match &self.graph[dest_node] {
            Instance { signature, .. } => {
                find_width(dest_port, &signature.inputs)
            }
            Input(_) => Err(Error::UndefinedPort(dest_port.to_string())),
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
            Err(Error::MismatchedPortWidths(
                self.construct_port(src_node, src_port),
                src_width,
                self.construct_port(dest_node, dest_port),
                dest_width,
            ))
        }
    }

    pub fn get_inst_index(&self, port: &ast::Id) -> Result<NodeIndex, Error> {
        match self.inst_map.get(port) {
            Some(idx) => Ok(*idx),
            None => Err(Error::UndefinedPort(port.to_string())),
        }
    }

    /// Return the `NodeIndex` for a named port. If not present, return an Error.
    pub fn get_io_index<S: AsRef<str>>(
        &self,
        port: S,
    ) -> Result<NodeIndex, Error> {
        let port: &str = port.as_ref();
        match self.portdef_map.get(&port.into()) {
            Some(idx) => Ok(*idx),
            None => Err(Error::UndefinedPort(port.to_string())),
        }
    }

    fn construct_port(&self, idx: NodeIndex, port: &str) -> ast::Port {
        match &self.graph[idx] {
            NodeData::Input(portdef) => Port::This {
                port: portdef.name.clone(),
            },
            NodeData::Output(portdef) => Port::This {
                port: portdef.name.clone(),
            },
            NodeData::Instance { name, .. } => Port::Comp {
                component: name.clone(),
                port: port.into(),
            },
        }
    }

    #[allow(unused)]
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

        ret.sort_by(|a, b| a.cmp(b));
        ret
    }
}
