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

/// store the structure ast node so that we can reconstruct the ast
#[derive(Clone, Debug)]
pub enum NodeData {
    Cell(ast::Cell),
    Group(ast::Group),
    Constant(u64),
    /// Represents a go/done hole
    Hole,
    Port,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: ast::Id,
    pub data: NodeData,
    pub signature: ast::Signature,
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

impl Node {
    pub fn get_component_type(&self) -> Result<&ast::Id> {
        match &self.data {
            NodeData::Port
            | NodeData::Group { .. }
            | NodeData::Constant(_)
            | NodeData::Hole => Err(errors::Error::NotSubcomponent),
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

    fn new_constant(namegen: &mut NameGenerator, val: u64) -> Self {
        Node {
            name: namegen.gen_name("$const").into(),
            data: NodeData::Constant(val),
            signature: ast::Signature {
                inputs: vec![],
                outputs: vec![("out", 32).into()],
            },
        }
    }

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
#[derive(Clone, Debug)]
pub struct EdgeData {
    pub src: Port,
    pub dest: Port,
    pub width: u64,
    pub group: Option<ast::Id>,
    pub guard: ast::Guard,
}

/// private graph type. the data in the node is the identifier
/// for the corresponding component, and the data on the edge
/// is (src port, dest port). Use stable graph so that NodeIndexes
/// remain valid after removals. the graph is directed
type StructG = StableDiGraph<Node, EdgeData>;

// I want to keep the fields of this struct private so that it is easy to swap
// out implementations / add new ways of manipulating this
/// Structure holds information about the structure of the current component
#[derive(Clone, Debug)]
pub struct StructureGraph {
    io: NodeIndex,
    nodes: HashMap<ast::Id, NodeIndex>,
    groups: HashMap<Option<ast::Id>, Vec<EdgeIndex>>,
    graph: StructG,
    namegen: NameGenerator,
}

impl Default for StructureGraph {
    fn default() -> Self {
        let mut graph = StructG::new();
        let io = graph.add_node(Node {
            name: "this".into(),
            data: NodeData::Port,
            signature: ast::Signature::default(),
        });
        StructureGraph {
            io,
            nodes: HashMap::new(),
            groups: HashMap::new(),
            graph,
            namegen: NameGenerator::default(),
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
                        // create sub-graph for this group
                        structure
                            .groups
                            .insert(Some(group.name.clone()), vec![]);

                        // add go/done holes
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
                        Node::new_constant(&mut structure.namegen, *n);
                    let idx = structure.graph.add_node(constant_node);
                    (idx, "out".into())
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
                &group,
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
    ///   * `structure` - the AST structure of the subcomponent
    pub fn add_subcomponent(
        &mut self,
        id: &ast::Id,
        comp: &component::Component,
        cell: ast::Cell,
    ) -> NodeIndex {
        let idx = self.graph.add_node(Node {
            name: id.clone(),
            data: NodeData::Cell(cell),
            signature: comp.signature.clone(),
        });
        self.nodes.insert(id.clone(), idx);
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
        let structure =
            Cell::prim(id.clone(), name.as_ref().into(), params.to_vec());
        self.add_subcomponent(id, comp, structure)
    }

    // pub fn add_group(
    //     &mut self,
    //     comps: &[ast::Id],
    // ) -> Result<(ast::Id, NodeIndex)> {
    //     let name: &str = &self.namegen.gen_name("gen");

    //     // check to make sure that all the comps are well defined
    //     for id in comps {
    //         if !self.nodes.contains_key(id) {
    //             return Err(Error::UndefinedComponent(id.clone()));
    //         }
    //     }

    //     // generate node for group
    //     let data = Node {
    //         name: name.into(),
    //         data: NodeData::Group(comps.to_vec()),
    //         signature: group_signature(),
    //     };
    //     let idx = self.graph.add_node(data);
    //     self.nodes.insert(name.into(), idx);

    //     Ok((name.into(), idx))
    // }
    /* ============= Helper Methods ============= */

    /// Returns an iterator over all the nodes in the structure graph
    // pub fn nodes(&self) -> impl Iterator<Item = (NodeIndex, Node)> + '_ {
    //     self.graph
    //         .node_indices()
    //         .map(move |ni| (ni, self.graph[ni].clone()))
    // }

    // pub fn group_nodes(
    //     &self,
    //     group_id: &ast::Id,
    // ) -> impl Iterator<Item = (NodeIndex, Node)> + '_ {
    //     let group_comps =
    //         self.nodes.get(group_id).map_or(vec![], move |gr_idx| {
    //             if let NodeData::Group(data) = &self.graph[*gr_idx].data {
    //                 data.clone()
    //             } else {
    //                 vec![]
    //             }
    //         });

    //     self.graph
    //         .node_indices()
    //         .filter(move |nidx| group_comps.contains(&self.graph[*nidx].name))
    //         .map(move |nidx| (nidx, self.graph[nidx].clone()))
    // }

    fn connected_direction<'a>(
        &'a self,
        node: NodeIndex,
        direction: Direction,
    ) -> impl Iterator<Item = (&'a Node, &'a EdgeData)> + 'a {
        let edge_iter = self
            .graph
            .edges_directed(node, direction)
            .map(|e| e.weight());
        let node_iter = self
            .graph
            .neighbors_directed(node, direction)
            .map(move |idx| &self.graph[idx]);
        node_iter.zip(edge_iter)
    }

    /// Returns a (Node, EdgeData) iterator for edges leaving `node`
    /// i.e. edges that have `node` as a source. This iterator ignores ports.
    pub fn outgoing_from_node<'a>(
        &'a self,
        node: NodeIndex,
    ) -> impl Iterator<Item = (&'a Node, &'a EdgeData)> + 'a {
        self.connected_direction(node, Direction::Outgoing)
    }

    /// Returns a (Node, EdgeData) iterator for edges coming into `node`
    /// i.e. edges that have `node` as a destination. This iterator ignores ports.
    pub fn incoming_to_node<'a>(
        &'a self,
        node: NodeIndex,
    ) -> impl Iterator<Item = (&'a Node, &'a EdgeData)> + 'a {
        self.connected_direction(node, Direction::Incoming)
    }

    /// Returns a (Node, EdgeData) iterator for edges leaving `node` at `port`
    /// i.e. edges that have `node.port` as a source.
    pub fn outgoing_from_port<'a, S: 'a + PartialEq<String>>(
        &'a self,
        node: NodeIndex,
        port: S,
    ) -> impl Iterator<Item = (&'a Node, &'a EdgeData)> + 'a {
        self.outgoing_from_node(node)
            .filter(move |(_nd, ed)| port == ed.src.port_name().to_string())
    }

    /// Returns a (Node, EdgeData) iterator for edges coming into `node` at `port`
    /// i.e. edges that have `node.port` as a destination.
    pub fn incoming_to_port<'a, S: 'a + PartialEq<String>>(
        &'a self,
        node: NodeIndex,
        port: S,
    ) -> impl Iterator<Item = (&'a Node, &'a EdgeData)> + 'a {
        self.incoming_to_node(node)
            .filter(move |(_nd, ed)| port == ed.dest.port_name().to_string())
    }

    pub fn insert_input_port(&mut self, port: &ast::Portdef) {
        let sig = &mut self.graph[self.io].signature;
        // add to outputs because was want to use input ports as sources for
        // wires in self
        sig.outputs.push(port.clone())
    }

    pub fn insert_output_port(&mut self, port: &ast::Portdef) {
        let sig = &mut self.graph[self.io].signature;
        // add to inputs because was want to use input ports as sources for
        // wires in self
        sig.inputs.push(port.clone())
    }

    /// Construct and insert an edge given two node indices with a group and a guard
    pub fn insert_edge(
        &mut self,
        src_node: NodeIndex,
        src_port: &ast::Id,
        dest_node: NodeIndex,
        dest_port: &ast::Id,
        group: &Option<ast::Id>,
        guard: ast::Guard,
    ) -> Result<EdgeIndex> {
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

        // if widths match, add edge to the graph
        if src_width == dest_width {
            let edge_data = EdgeData {
                src: self.construct_port(src_node, src_port),
                dest: self.construct_port(dest_node, dest_port),
                width: src_width,
                group: group.clone(),
                guard,
            };
            Ok(self.graph.add_edge(src_node, dest_node, edge_data))
        } else {
            Err(Error::MismatchedPortWidths(
                self.construct_port(src_node, src_port),
                src_width,
                self.construct_port(dest_node, dest_port),
                dest_width,
            ))
        }
    }

    // pub fn remove_edge<S: AsRef<str>, U: AsRef<str>>(
    //     &mut self,
    //     src_node: NodeIndex,
    //     src_port: S,
    //     dest_node: NodeIndex,
    //     dest_port: U,
    // ) -> Result<()> {
    //     let edge_idx = self.graph.edge_indices()
    //         .filter_map(|eidx| self.graph.edge_endpoints(eidx))
    //         .filter(|(s, t)| s == &src_node && t == &dest_node)
    //         .find(|(s, t)| );
    //     Ok(())
    //         // (src_node, dest_node).find()
    // }

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
            NodeData::Cell(..)
            | NodeData::Group(..)
            | NodeData::Constant(..)
            | NodeData::Hole => Port::Comp {
                component: node.name.clone(),
                port: port.clone(),
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
