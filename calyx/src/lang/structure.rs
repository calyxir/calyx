use super::structure_iter;
use crate::{
    errors,
    lang::{ast, component},
    utils::NameGenerator,
};
use ast::{Atom, BitNum, Cell, Connection, Group, Port, Wire};
use component::Component;
use errors::{Error, Result};
use itertools::Itertools;
use petgraph::{
    graph::{EdgeIndex, NodeIndex},
    stable_graph::{EdgeReference, StableDiGraph},
    visit::IntoEdgeReferences,
    Direction,
};
use std::{cmp::Ordering, collections::HashMap, fmt};

/// store the structure ast node so that we can reconstruct the ast
#[derive(Clone, Debug)]
pub enum NodeData {
    /// An instantiated subcomponent
    Cell(ast::Cell),
    Constant(BitNum),
    /// A go/done hole for group `ast::Id`
    Hole(ast::Id),
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

/* TODO(rachit): Change the interface representation of ports to an opaque
 * handler.
/// Opaque handle to a port on a component. These can only be created by
/// calling Node::port_handle method and forces uses of ports to make sure
/// that they exist on the Node.
struct PortHandle<'a>(&'a ast::Id);
*/

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
            NodeData::Port | NodeData::Constant(_) | NodeData::Hole(_) => {
                Err(errors::Error::NotSubcomponent)
            }
            NodeData::Cell(structure) => match structure {
                Cell::Prim { data } => Ok(&data.instance.name),
                Cell::Decl { data } => Ok(&data.component),
            },
        }
    }

    // XXX(rachit): Why don't we return impl Iterator here instead of wrapping
    // things in a PortIter.
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

    pub fn find_port<S: AsRef<str>>(&self, port_name: S) -> Option<&ast::Id> {
        let port_id: ast::Id = port_name.as_ref().into();
        self.signature
            .inputs
            .iter()
            .chain(self.signature.outputs.iter())
            .find(|portdef| portdef.name == port_id)
            .map(|portdef| &portdef.name)
    }

    /// Create a constant node for the number `num`
    fn new_constant(
        namegen: &mut NameGenerator,
        num: &ast::BitNum,
    ) -> (ast::Id, Self) {
        let name = ast::Id::new(namegen.gen_name("$const"), num.span.clone());
        let node = Node {
            name: name.clone(),
            data: NodeData::Constant(num.clone()),
            signature: ast::Signature {
                inputs: vec![],
                outputs: vec![("out", num.width).into()],
            },
        };
        (name, node)
    }

    /// Create a new hole node for the group `group_name`
    fn new_hole(group_name: ast::Id) -> Self {
        Node {
            name: group_name.clone(),
            data: NodeData::Hole(group_name),
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
    pub guards: Vec<ast::GuardExpr>,
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
    /// The "fake" node that represents this component. It contains the
    /// input output ports for this component.
    io: NodeIndex,

    /// Maps Ids to their corresponding node
    nodes: HashMap<ast::Id, NodeIndex>,

    /// Mapping for defined constants. This allows us to avoid defining
    /// duplicate nodes for pre-existing constants. Indexed by (val, width)
    /// tuple.
    constants: HashMap<(u64, u64), NodeIndex>,

    /// maps Ids to Vec<Edge> which represents the group
    /// the set of edges belong to. None refers to edges
    /// that are in no group.
    groups: HashMap<Option<ast::Id>, Vec<EdgeIndex>>,
    graph: StructG,
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
            constants: HashMap::new(),
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
                    let name = group.name.clone();
                    let key = Some(name.clone());
                    if !structure.groups.contains_key(&key) {
                        // XXX(rachit): This is the wrong way to handle
                        // the Result<_> returned from insert_group.
                        structure.insert_group(&name).expect(
                            "Malformed input AST: Duplicate group names found.",
                        );
                    }

                    group.wires.iter().map(|w| (key.clone(), w)).collect()
                }
            })
            .flatten()
            .collect();

        // Create "default" group that contains all edges without a group.
        structure.groups.insert(None, Vec::new());
        // then add edges
        for (group, wire) in wires {
            if let Some(ref name) = group {
                if !structure.groups.contains_key(&group) {
                    structure.insert_group(name)?;
                }
            }
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
                Atom::Num(n) => structure.new_constant(n.val, n.width)?,
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
            structure.insert_edge(
                (src_node, &src_port),
                (dest_node, &dest_port),
                group.clone(),
                wire.src.guard.clone(),
            )?;
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

    /// Adds a node to the structure graph.
    /// # Arguments
    ///   * `id` - the subcomponent identifier
    ///   * `node` - the component object
    pub fn add_node(&mut self, id: ast::Id, node: Node) -> NodeIndex {
        let idx = self.graph.add_node(node);
        self.nodes.insert(id, idx);
        idx
    }

    /// Adds a cell node to the structure graph.
    ///
    /// # Arguments
    ///   * `id` - the subcomponent identifier
    ///   * `comp` - the component object
    ///   * `cell` - TODO
    pub fn add_cell(
        &mut self,
        id: ast::Id,
        comp: &component::Component,
        cell: ast::Cell,
    ) -> NodeIndex {
        let node = Node {
            name: id.clone(),
            data: NodeData::Cell(cell),
            signature: comp.signature.clone(),
        };
        self.add_node(id, node)
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
        self.add_cell(id, comp, cell)
    }

    /// Add a constant node into the structure graph. If the same (using
    /// (val, width) tuple) constant has already been defined, return the
    /// index of that node instead.
    ///
    /// Returns the NodeIndex for the constant node and the port on which it
    /// outputs the value.
    pub fn new_constant(
        &mut self,
        val: u64,
        width: u64,
    ) -> errors::Result<(NodeIndex, ast::Id)> {
        let key = &(val, width);
        let port = ast::Id::new("out", None);
        if self.constants.contains_key(&key) {
            return Ok((self.constants[key], port));
        }
        // If the given constant doesn't already exist, create it and add it.
        let bitnum = ast::BitNum {
            width,
            num_type: ast::NumType::Decimal,
            val,
            span: None,
        };
        let (name, node) = Node::new_constant(&mut self.namegen, &bitnum);
        let idx = self.add_node(name, node);
        self.constants.insert(*key, idx);
        Ok((idx, port))
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
    pub fn insert_group(&mut self, name: &ast::Id) -> Result<()> {
        let key = Some(name.clone());
        if self.groups.contains_key(&key) {
            return Err(errors::Error::DuplicateGroup(name.clone()));
        }
        // create a new group
        self.groups.insert(key, Vec::new());

        // Create fake node for this group and add go/done holes
        self.nodes.insert(
            name.clone(),
            self.graph.add_node(Node::new_hole(name.clone())),
        );
        Ok(())
    }

    /// Construct and insert an edge given two node indices with a group and a guard
    pub fn insert_edge(
        &mut self,
        (src_node, src_port): (NodeIndex, &ast::Id),
        (dest_node, dest_port): (NodeIndex, &ast::Id),
        group: Option<ast::Id>,
        guards: Vec<ast::GuardExpr>,
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
            ));
        }

        // Add edge data and update the groups mapping.
        let edge_data = EdgeData {
            src: self.construct_port(src_node, src_port),
            dest: self.construct_port(dest_node, dest_port),
            width: src_width,
            group: group.clone(),
            guards,
        };
        let idx = self.graph.add_edge(src_node, dest_node, edge_data);
        self.groups.get_mut(&group).unwrap().push(idx);
        Ok(idx)
    }

    pub fn visualize(&self) -> String {
        use petgraph::dot::{Config, Dot};
        let config = &[Config::EdgeNoLabel];
        format!(
            "{}",
            Dot::with_attr_getters(
                &self.graph,
                config,
                &|_g, _edgeref| { "".to_string() },
                &|_g, (_idx, node)| {
                    match node.data {
                        NodeData::Hole(..) => "shape=diamond".to_string(),
                        NodeData::Cell(..) => "shape=box".to_string(),
                        _ => "".to_string(),
                    }
                }
            )
        )
    }

    /* ============= Getter Methods ============= */
    pub fn get_node(&self, idx: &NodeIndex) -> &Node {
        &self.graph[*idx]
    }

    pub fn get_node_by_name(&self, name: &ast::Id) -> Option<&NodeIndex> {
        self.nodes.get(name)
    }

    /* ============= Helper Methods ============= */
    /// Constructs a ast::Port from a NodeIndex and Id
    fn construct_port(&self, idx: NodeIndex, port: &ast::Id) -> ast::Port {
        let node = &self.graph[idx];
        match &node.data {
            NodeData::Port => Port::This { port: port.clone() },
            NodeData::Cell(..) | NodeData::Constant(..) => Port::Comp {
                component: node.name.clone(),
                port: port.clone(),
            },
            NodeData::Hole(group) => Port::Hole {
                group: group.clone(),
                name: port.clone(),
            },
        }
    }

    /* ============= Iteration Methods ============= */
    /// Construct an immutable iteration pattern using an EdgeIterationBuilder.
    pub fn edge_iterator<'a>(
        &'a self,
        iter_spec: structure_iter::ConnectionIteration,
    ) -> impl Iterator<Item = &'a EdgeData> {
        let base: Box<dyn Iterator<Item = EdgeReference<EdgeData>>> =
            match (iter_spec.from_node, iter_spec.direction) {
                (Some(node), Some(dir)) => {
                    Box::new(self.graph.edges_directed(node, dir.into()))
                }
                (Some(node), None) => Box::new(self.graph.edges(node)),
                /* Iterate all edges and select port direction */
                (None, Some(_)) | (None, None) => {
                    Box::new(self.graph.edge_references())
                }
            };

        base.map(|edge| edge.weight()).filter(move |ed| {
            match (iter_spec.with_port.as_ref(), iter_spec.direction.as_ref()) {
                (Some(port), Some(DataDirection::Read)) => {
                    ed.src.port_name().to_string() == *port
                }
                (Some(port), Some(DataDirection::Write)) => {
                    ed.dest.port_name().to_string() == *port
                }
                (Some(port), _) => {
                    ed.dest.port_name().to_string() == *port
                        || ed.src.port_name().to_string() == *port
                }
                (None, _) => true,
            }
        })
    }

    /// Construct an immutable iteration pattern using an EdgeIterationBuilder.
    pub fn edge_iterator_mut<'a>(
        &'a mut self,
        iter_spec: structure_iter::ConnectionIteration,
    ) -> Result<impl Iterator<Item = &'a mut EdgeData>> {
        // XXX(rachit): Unfortunately couldn't find any good way to iterate
        // over edges while filtering for a given node. The heavyweight approach
        // would be to store the name of the Node inside the EdgeData.
        if let Some(_) = iter_spec.from_node {
            return Err(errors::Error::Impossible("Cannot create a mutable iterator over edges using a given node.".to_string()));
        }

        let it = self.graph.edge_weights_mut().filter(move |ed| {
            match (iter_spec.with_port.as_ref(), iter_spec.direction.as_ref()) {
                (Some(port), Some(DataDirection::Read)) => {
                    ed.src.port_name().to_string() == *port
                }
                (Some(port), Some(DataDirection::Write)) => {
                    ed.dest.port_name().to_string() == *port
                }
                (Some(port), _) => {
                    ed.dest.port_name().to_string() == *port
                        || ed.src.port_name().to_string() == *port
                }
                (None, _) => true,
            }
        });
        Ok(it)
    }

    /// Returns an iterator over all the nodes (components).
    pub fn component_iterator<'a>(
        &'a self,
    ) -> impl Iterator<Item = (NodeIndex, &'a Node)> + 'a {
        self.graph
            .node_indices()
            .map(move |idx| (idx, &self.graph[idx]))
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
                        let edge = &self.graph[*ed];
                        let src = ast::Guard {
                            guard: edge.guards.clone(),
                            expr: Atom::Port(edge.src.clone()),
                        };
                        Connection::Wire(Wire {
                            src,
                            dest: edge.dest.clone(),
                        })
                    })
                    .collect(),
                Some(name) => vec![Connection::Group(Group {
                    name: name.clone(),
                    wires: group_wires
                        .iter()
                        .map(|ed| {
                            let edge = &self.graph[*ed];
                            let src = ast::Guard {
                                guard: edge.guards.clone(),
                                expr: Atom::Port(edge.src.clone()),
                            };
                            Wire {
                                src,
                                dest: edge.dest.clone(),
                            }
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
