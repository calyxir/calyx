use crate::lang::{ast, component};
use crate::{errors, utils::NameGenerator};
use ast::{Atom, Cell, Connection, Group, Port, Wire};
use component::Component;
use errors::{Error, Result};
use itertools::Itertools;
use petgraph::dot::{Config, Dot};
use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::stable_graph::StableDiGraph;
use petgraph::Direction;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;

/// store the structure ast node so that we can reconstruct the ast
#[derive(Clone, Debug)]
pub enum NodeData {
    /// An instantiated subcomponent
    Cell(ast::Cell),
    Constant(u64),
    /// A go/done hole
    Hole,
    /// A port for this component
    Port,
}

/// The data that we store in each Petgraph Node
#[derive(Clone, Debug)]
pub struct Node {
    pub name: ast::Id,
    pub data: NodeData,
    pub signature: ast::Signature,
}

/// Iterator for ports
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

impl Node {
    pub fn get_component_type(&self) -> Result<&ast::Id> {
        match &self.data {
            NodeData::Port | NodeData::Constant(_) | NodeData::Hole => {
                Err(errors::Error::NotSubcomponent)
            }
            NodeData::Cell(structure) => match structure {
                Cell::Prim { data } => Ok(&data.instance.name),
                Cell::Decl { data } => Ok(&data.component),
            },
        }
    }

    pub fn out_ports(&self) -> PortIter {
        PortIter {
            items: self.signature.outputs.clone(),
        }
    }

    pub fn in_ports(&self) -> PortIter {
        PortIter {
            items: self.signature.inputs.clone(),
        }
    }

    /// Create a constant node for the number `num`
    fn new_constant(namegen: &mut NameGenerator, num: &ast::BitNum) -> Self {
        let name =
            ast::Id::new(namegen.gen_name("$const"), num.span.clone());
        Node {
            name,
            data: NodeData::Constant(num.val),
            signature: ast::Signature {
                inputs: vec![],
                outputs: vec![("out", num.width).into()],
            },
        }
    }

    /// Create a new hole node for the group `name`
    fn new_hole(name: ast::Id) -> Self {
        Node {
            name,
            data: NodeData::Hole,
            signature: ast::Signature {
                // include both go/done in input and output because you both write and read from these
                inputs: vec![("go", 1).into(), ("done", 1).into()],
                outputs: vec![("go", 1).into(), ("done", 1).into()],
            },
        }
    }
}
/// store the src port and dst port on edge
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct EdgeData {
    pub src: Port,
    pub dest: Port,
    pub width: u64,
    pub group: Option<ast::Id>,
    pub guard: ast::Guard,
}

/// private graph type. the data in the node stores information
/// for the corresponding node type, and the data on the edge
/// is (src port, dest port, group, guard). We use stable graph so that NodeIndexes
/// remain valid after removals. the graph is directed
type StructG = StableDiGraph<Node, EdgeData>;

// I want to keep the fields of this struct private so that it is easy to swap
// out implementations / add new ways of manipulating this
/// Structure holds information about the structure of the current component
#[derive(Clone, Debug)]
pub struct StructureGraph {
    /// The node that holds signature for this component
    io: NodeIndex,
    /// Maps Ids to their corresponding node
    nodes: HashMap<ast::Id, NodeIndex>,
    /// maps Ids to Vec<Edge> which represents the group
    /// the set of edges belong to. None refers to edges
    /// that are in no group.
    groups: HashMap<Option<ast::Id>, Vec<EdgeIndex>>,
    pub graph: StructG,
    pub namegen: NameGenerator,
}

impl Default for StructureGraph {
    fn default() -> Self {
        let mut graph = StructG::new();
        // add a node for the ports for this component. This starts out empty.
        let io = graph.add_node(Node {
            name: "this".into(),
            data: NodeData::Port,
            signature: ast::Signature::default(),
        });
        let mut nodes = HashMap::new();
        nodes.insert("this".into(), io);
        StructureGraph {
            io,
            nodes,
            groups: HashMap::new(),
            graph,
            namegen: NameGenerator::default(),
        }
    }
}

/// Represents flow of data to/from ports. Used to select edges from
/// ports and nodes.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(usize)]
pub enum DataDirection {
    /// reads for this node/port.
    Read = 0,
    /// writes for this node/port.
    Write = 1,
}

impl Into<petgraph::Direction> for DataDirection {
    fn into(self) -> petgraph::Direction {
        match self {
            DataDirection::Read => Direction::Outgoing,
            DataDirection::Write => Direction::Incoming,
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
    ) -> Result<Self> {
        let mut structure = StructureGraph::default();

        structure.add_signature(&compdef.signature);

        // add vertices first, ignoring wires so that order of structure
        // doesn't matter
        for stmt in &compdef.cells {
            match stmt {
                Cell::Decl { data } => {
                    // lookup signature for data.component
                    let sig =
                        comp_sigs.get(&data.component).ok_or_else(|| {
                            Error::UndefinedComponent(data.component.clone())
                        })?;
                    // create node for this decl
                    let instance = Node {
                        name: data.name.clone(),
                        data: NodeData::Cell(stmt.clone()),
                        signature: sig.clone(),
                    };
                    // insert the node into the graph
                    structure.nodes.insert(
                        data.name.clone(),
                        structure.graph.add_node(instance),
                    );
                }
                Cell::Prim { data } => {
                    // resolve param signature and add it to hashmap so that
                    //  we keep a reference to it
                    let sig = prim_sigs.get(&data.name).ok_or_else(|| {
                        Error::UndefinedComponent(data.name.clone())
                    })?;
                    // create node for this prim
                    let instance = Node {
                        name: data.name.clone(),
                        data: NodeData::Cell(stmt.clone()),
                        signature: sig.clone(),
                    };
                    // insert the node into the graph
                    structure.nodes.insert(
                        data.name.clone(),
                        structure.graph.add_node(instance),
                    );
                }
            }
        }

        // flatten connections into Vec<(group_name: Option<Id>, wire: Wire)>
        let wires: Vec<_> = compdef
            .connections
            .iter()
            .map(|stmt| match stmt {
                Connection::Wire(wire) => vec![(None, wire)],
                Connection::Group(group) => {
                    // create group if it does not exist
                    if !structure.groups.contains_key(&Some(group.name.clone()))
                    {
                        // create a new group
                        structure
                            .groups
                            .insert(Some(group.name.clone()), vec![]);

                        // add go/done hole
                        structure.nodes.insert(
                            group.name.clone(),
                            structure
                                .graph
                                .add_node(Node::new_hole(group.name.clone())),
                        );
                    }

                    group
                        .wires
                        .iter()
                        .map(|w| (Some(group.name.clone()), w))
                        .collect()
                }
            })
            .flatten()
            .collect();

        // then add edges
        for (group, wire) in wires {
            // get src node and port in graph
            let (src_node, src_port) = match &wire.src.expr {
                Atom::Port(p) => match p {
                    Port::Comp { component: c, port }
                    | Port::Hole {
                        group: c,
                        name: port,
                    } => (
                        *structure.nodes.get(c).ok_or_else(|| {
                            Error::UndefinedComponent(c.clone())
                        })?,
                        port.clone(),
                    ),
                    Port::This { port } => (structure.io, port.clone()),
                },
                Atom::Num(n) => {
                    let constant_node =
                        Node::new_constant(&mut structure.namegen, n);
                    let idx = structure.graph.add_node(constant_node);
                    let port = ast::Id::new("out", n.span.clone());
                    (idx, port)
                }
            };

            // get dest node and port in graph
            let (dest_node, dest_port) = match &wire.dest {
                Port::Comp { component: c, port }
                | Port::Hole {
                    group: c,
                    name: port,
                } => (
                    *structure
                        .nodes
                        .get(c)
                        .ok_or_else(|| Error::UndefinedComponent(c.clone()))?,
                    port.clone(),
                ),
                Port::This { port } => (structure.io, port.clone()),
            };
            let ed_idx = structure.insert_edge(
                src_node,
                &src_port,
                dest_node,
                &dest_port,
                group.clone(),
                wire.src.clone(),
            )?;
            structure
                .groups
                .entry(group)
                .and_modify(|edges| edges.push(ed_idx))
                .or_insert_with(|| vec![ed_idx]);
        }
        Ok(structure)
    }

    /// Adds nodes for input and output ports to the structure graph.
    /// Input/output ports are defined in the component signature.
    ///
    /// # Arguments
    ///   * `sig` - the signature for the component
    pub fn add_signature(&mut self, sig: &ast::Signature) {
        let mut data = &mut self.graph[self.io];
        let (inputs, outputs) = (sig.inputs.clone(), sig.outputs.clone());
        data.signature = ast::Signature {
            inputs: outputs,
            outputs: inputs,
        };
    }

    /// Adds a subcomponent node to the structure graph.
    ///
    /// # Arguments
    ///   * `id` - the subcomponent identifier
    ///   * `comp` - the component object
    ///   * `cell` - TODO
    pub fn add_subcomponent(
        &mut self,
        id: ast::Id,
        comp: &component::Component,
        cell: ast::Cell,
    ) -> NodeIndex {
        let idx = self.graph.add_node(Node {
            name: id.clone(),
            data: NodeData::Cell(cell),
            signature: comp.signature.clone(),
        });
        self.nodes.insert(id, idx);
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
    pub fn add_primitive<S: AsRef<str>>(
        &mut self,
        id: ast::Id,
        name: S,
        comp: &Component,
        params: &[u64],
    ) -> NodeIndex {
        let cell =
            Cell::prim(id.clone(), name.as_ref().into(), params.to_vec());
        self.add_subcomponent(id, comp, cell)
    }

    /* ============= Helper Methods ============= */

    /// Returns a (Node, EdgeData) iterator for edges in a particular
    /// petgraph::Direction
    fn node_directed<'a>(
        &'a self,
        direction: DataDirection,
        node: NodeIndex,
    ) -> impl Iterator<Item = (&'a Node, &'a EdgeData)> + 'a {
        let edge_iter = self
            .graph
            .edges_directed(node, direction.into())
            .map(|e| e.weight());
        let node_iter = self
            .graph
            .neighbors_directed(node, direction.into())
            .map(move |idx| &self.graph[idx]);
        node_iter.zip(edge_iter)
    }

    /// Returns a (Node, EdgeData) iterator for edges with `node.port` in
    /// the given DataDirection.
    pub fn port_directed<'a, S: 'a + PartialEq<String>>(
        &'a self,
        direction: DataDirection,
        node: NodeIndex,
        port: S,
    ) -> impl Iterator<Item = (&'a Node, &'a EdgeData)> + 'a {
        self.node_directed(direction, node)
            .filter(move |(_nd, ed)| port == ed.src.port_name().to_string())
    }

    /// Returns an iterator over all the edges.
    pub fn edges<'a>(
        &'a self,
    ) -> impl Iterator<Item = (EdgeIndex, &'a EdgeData)> + 'a {
        self.groups
            .values()
            .flatten()
            .map(move |idx| (*idx, &self.graph[*idx]))
    }

    pub fn nodes<'a>(
        &'a self,
    ) -> impl Iterator<Item = (NodeIndex, &'a Node)> + 'a {
        self.nodes
            .values()
            .map(move |idx| (*idx, &self.graph[*idx]))
    }

    /// TODO(rachit): Sam, check if this documentation is correct.
    /// Add a new input port to the component that owns this Graph.
    pub fn insert_input_port(&mut self, port: &ast::Portdef) {
        let sig = &mut self.graph[self.io].signature;
        // add to outputs because was want to use input ports as sources for
        // wires in self
        sig.outputs.push(port.clone())
    }

    /// TODO(rachit): Sam, check if this documentation is correct.
    /// Add a new output port to the component that owns this Graph.
    pub fn insert_output_port(&mut self, port: &ast::Portdef) {
        let sig = &mut self.graph[self.io].signature;
        // add to inputs because was want to use input ports as sources for
        // wires in self
        sig.inputs.push(port.clone())
    }

    /// Add a new named group into the structure.
    pub fn insert_group(
        &mut self,
        name: ast::Id
    ) -> Result<()> {
        let key = Some(name.clone());
        if self.groups.contains_key(&key) {
            return Err(errors::Error::DuplicateGroup(name))
        }
        self.groups.insert(key, Vec::new());
        Ok(())
    }

    /// Construct and insert an edge given two node indices with a group and a guard
    pub fn insert_edge(
        &mut self,
        src_node: NodeIndex,
        src_port: &ast::Id,
        dest_node: NodeIndex,
        dest_port: &ast::Id,
        group: Option<ast::Id>,
        guard: ast::Guard,
    ) -> Result<EdgeIndex> {
        // If the group is not defined, error out.
        if let Some(ref group_name) = group {
            if !self.groups.contains_key(&group) {
                return Err(errors::Error::UndefinedGroup(group_name.clone()));
            }
        }
        let find_width = |port_to_find: &ast::Id, portdefs: &[ast::Portdef]| {
            portdefs
                .iter()
                .find_map(|x| {
                    if &x.name == port_to_find {
                        Some(x.width)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| Error::UndefinedPort(port_to_find.clone()))
        };
        let src_width =
            find_width(src_port, &self.graph[src_node].signature.outputs)?;
        let dest_width =
            find_width(dest_port, &self.graph[dest_node].signature.inputs)?;

        // if widths dont match, throw error.
        if src_width != dest_width {
            return Err(Error::MismatchedPortWidths(
                self.construct_port(src_node, src_port),
                src_width,
                self.construct_port(dest_node, dest_port),
                dest_width,
            ))
        }

        // Add edge data and update the groups mapping.
        let edge_data = EdgeData {
            src: self.construct_port(src_node, src_port),
            dest: self.construct_port(dest_node, dest_port),
            width: src_width,
            group: group.clone(),
            guard,
        };
        Ok(self.graph.add_edge(src_node, dest_node, edge_data))
    }

    /// Returns the node representing this component
    pub fn this(&self) -> &Node {
        &self.graph[self.io]
    }

    /// Returns the idx for the node representing this component
    pub fn this_idx(&self) -> NodeIndex {
        self.io
    }

    pub fn get(&self, idx: NodeIndex) -> &Node {
        &self.graph[idx]
    }

    pub fn get_idx(&self, port: &ast::Id) -> Result<NodeIndex> {
        match self.nodes.get(port) {
            Some(idx) => Ok(*idx),
            None => Err(Error::UndefinedPort(port.clone())),
        }
    }

    /// Constructs a ast::Port from a NodeIndex and Id
    fn construct_port(&self, idx: NodeIndex, port: &ast::Id) -> ast::Port {
        let node = &self.graph[idx];
        match node.data {
            NodeData::Port => Port::This { port: port.clone() },
            NodeData::Cell(..) | NodeData::Constant(..) | NodeData::Hole => {
                Port::Comp {
                    component: node.name.clone(),
                    port: port.clone(),
                }
            }
        }
    }

    pub fn visualize(&self) -> String {
        let config = &[Config::EdgeNoLabel];
        format!(
            "{}",
            Dot::with_attr_getters(
                &self.graph,
                config,
                &|_g, _edgeref| { "".to_string() },
                &|_g, (_idx, node)| {
                    match node.data {
                        NodeData::Hole => "shape=diamond".to_string(),
                        NodeData::Cell(..) => "shape=box".to_string(),
                        _ => "".to_string(),
                    }
                }
            )
        )
    }
}

// Define visualization for edges
impl fmt::Display for EdgeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.width)
    }
}

// Define visualization for nodes
impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name.to_string())
    }
}

// Implement conversion of graph back into a structure ast vector
impl Into<(Vec<ast::Cell>, Vec<ast::Connection>)> for StructureGraph {
    fn into(self) -> (Vec<ast::Cell>, Vec<ast::Connection>) {
        let cells = self
            .nodes
            .iter()
            .filter_map(|(_name, idx)| {
                if let NodeData::Cell(data) = &self.graph[*idx].data {
                    Some(data.clone())
                } else {
                    None
                }
            })
            .sorted()
            .collect();

        // add wire structure stmts for edges
        let connections = self
            .groups
            .iter()
            .map(|(name, group_wires)| match name {
                None => group_wires
                    .iter()
                    .map(|ed| {
                        Connection::Wire(Wire {
                            src: self.graph[*ed].guard.clone(),
                            dest: self.graph[*ed].dest.clone(),
                        })
                    })
                    .collect(),
                Some(name) => vec![Connection::Group(Group {
                    name: name.clone(),
                    wires: group_wires
                        .iter()
                        .map(|ed| Wire {
                            src: self.graph[*ed].guard.clone(),
                            dest: self.graph[*ed].dest.clone(),
                        })
                        .collect(),
                })],
            })
            .flatten()
            .sorted()
            .collect();

        (cells, connections)
    }
}
