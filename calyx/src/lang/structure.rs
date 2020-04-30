use crate::errors;
use crate::lang::{ast, component};
use crate::utils::NameGenerator;
use ast::{Id, Port, Structure};
use component::Component;
use errors::Error;
use petgraph::dot::{Config, Dot};
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableDiGraph;
use petgraph::Direction;
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum NodeData {
    Instance(ast::Structure),
    Group(Vec<ast::Id>),
    Port,
}

/// store the structure ast node so that we can reconstruct the ast
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
    pub fn get_component_type(&self) -> Result<&ast::Id, errors::Error> {
        match &self.data {
            NodeData::Port | NodeData::Group { .. } => {
                Err(errors::Error::NotSubcomponent)
            }
            NodeData::Instance(structure) => match structure {
                Structure::Wire { .. } => Err(Error::Impossible(
                    "There should be no wires in nodes".to_string(),
                )),
                Structure::Std { data } => Ok(&data.instance.name),
                Structure::Decl { data } => Ok(&data.component),
                Structure::Group { .. } => Err(Error::Impossible(
                    "There should be no wires in nodes".to_string(),
                )),
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
type StructG = StableDiGraph<Node, EdgeData>;

// I want to keep the fields of this struct private so that it is easy to swap
// out implementations / add new ways of manipulating this
/// Structure holds information about the structure of the current component
#[derive(Clone, Debug)]
pub struct StructureGraph {
    ports: NodeIndex,
    nodes: HashMap<ast::Id, NodeIndex>,
    graph: StructG,
    namegen: NameGenerator,
}

impl Default for StructureGraph {
    fn default() -> Self {
        let mut graph = StructG::new();
        let ports = graph.add_node(Node {
            name: "this".into(),
            data: NodeData::Port,
            signature: ast::Signature::default(),
        });
        StructureGraph {
            ports,
            nodes: HashMap::new(),
            graph,
            namegen: NameGenerator::default(),
        }
    }
}

fn group_signature() -> ast::Signature {
    ast::Signature {
        inputs: vec![
            ("valid", 1).into(),
            ("ready", 1).into(),
            ("clk", 1).into(),
        ],
        outputs: vec![
            ("valid", 1).into(),
            ("ready", 1).into(),
            ("clk", 1).into(),
        ],
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
                    let instance = Node {
                        name: data.name.clone(),
                        data: NodeData::Instance(stmt.clone()),
                        signature: sig.clone(),
                    };
                    structure.nodes.insert(
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
                    let instance = Node {
                        name: data.name.clone(),
                        data: NodeData::Instance(stmt.clone()),
                        signature: sig.clone(),
                    };
                    structure.nodes.insert(
                        data.name.clone(),
                        structure.graph.add_node(instance),
                    );
                }
                ast::Structure::Group { data } => {
                    let data = Node {
                        name: data.name.clone(),
                        data: NodeData::Group(data.comps.clone()),
                        signature: group_signature(),
                    };
                    structure.nodes.insert(
                        data.name.clone(),
                        structure.graph.add_node(data),
                    );
                }
                ast::Structure::Wire { .. } => {}
            }
        }

        // then add edges
        for stmt in &compdef.structure {
            if let ast::Structure::Wire { data } = stmt {
                println!("1");
                // get src node in graph and src port
                let (src_node, src_port) = match &data.src {
                    Port::Comp { component, port } => (
                        structure.nodes.get(component).ok_or_else(|| {
                            Error::UndefinedComponent(component.clone())
                        })?,
                        port,
                    ),
                    Port::This { port } => (&structure.ports, port),
                };

                // get dest node in graph and dest port
                let (dest_node, dest_port) = match &data.dest {
                    Port::Comp { component, port } => (
                        structure.nodes.get(component).ok_or_else(|| {
                            Error::UndefinedComponent(component.clone())
                        })?,
                        port,
                    ),
                    Port::This { port } => (&structure.ports, port),
                };

                // add the edge
                let src = *src_node;
                let dest = *dest_node;
                structure.insert_edge(src, src_port, dest, dest_port)?;
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
        let mut data = &mut self.graph[self.ports];
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
        structure: ast::Structure,
    ) -> NodeIndex {
        let idx = self.graph.add_node(Node {
            name: id.clone(),
            data: NodeData::Instance(structure),
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
        let structure = ast::Structure::std(
            id.clone(),
            ast::Compinst {
                name: name.as_ref().into(),
                params: params.to_vec(),
            },
        );
        self.add_subcomponent(id, comp, structure)
    }

    pub fn add_group(
        &mut self,
        comps: &[ast::Id],
    ) -> Result<(ast::Id, NodeIndex), Error> {
        let name: &str = &self.namegen.gen_name("gen");

        // check to make sure that all the comps are well defined
        for id in comps {
            if !self.nodes.contains_key(id) {
                return Err(Error::UndefinedComponent(id.clone()));
            }
        }

        // generate node for group
        let data = Node {
            name: name.into(),
            data: NodeData::Group(comps.to_vec()),
            signature: group_signature(),
        };
        let idx = self.graph.add_node(data);
        self.nodes.insert(name.into(), idx);

        Ok((name.into(), idx))
    }
    /* ============= Helper Methods ============= */

    /// Returns an iterator over all the nodes in the structure graph
    pub fn nodes(&self) -> impl Iterator<Item = (NodeIndex, Node)> + '_ {
        self.graph
            .node_indices()
            .map(move |ni| (ni, self.graph[ni].clone()))
    }

    pub fn group_nodes(
        &self,
        group_id: &ast::Id,
    ) -> impl Iterator<Item = (NodeIndex, Node)> + '_ {
        let group_comps =
            self.nodes.get(group_id).map_or(vec![], move |gr_idx| {
                if let NodeData::Group(data) = &self.graph[*gr_idx].data {
                    data.clone()
                } else {
                    vec![]
                }
            });

        self.graph
            .node_indices()
            .filter(move |nidx| group_comps.contains(&self.graph[*nidx].name))
            .map(move |nidx| (nidx, self.graph[nidx].clone()))
    }

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
    /// i.e. edges that have `(@ node port)` as a source.
    pub fn outgoing_from_port<'a, S: 'a + PartialEq<String>>(
        &'a self,
        node: NodeIndex,
        port: S,
    ) -> impl Iterator<Item = (&'a Node, &'a EdgeData)> + 'a {
        self.outgoing_from_node(node)
            .filter(move |(_nd, ed)| port == ed.src)
    }

    /// Returns a (Node, EdgeData) iterator for edges coming into `node` at `port`
    /// i.e. edges that have `(@ node port)` as a destination.
    pub fn incoming_to_port<'a, S: 'a + PartialEq<String>>(
        &'a self,
        node: NodeIndex,
        port: S,
    ) -> impl Iterator<Item = (&'a Node, &'a EdgeData)> + 'a {
        self.incoming_to_node(node)
            .filter(move |(_nd, ed)| port == ed.dest)
    }

    pub fn insert_input_port(&mut self, port: &ast::Portdef) {
        let sig = &mut self.graph[self.ports].signature;
        // add to outputs because was want to use input ports as sources for
        // wires in self
        sig.outputs.push(port.clone())
    }

    pub fn insert_output_port(&mut self, port: &ast::Portdef) {
        let sig = &mut self.graph[self.ports].signature;
        // add to inputs because was want to use input ports as sources for
        // wires in self
        sig.inputs.push(port.clone())
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
        let find_width = |port_to_find: &str, portdefs: &[ast::Portdef]| {
            portdefs
                .iter()
                .find_map(|x| {
                    if &x.name == port_to_find {
                        Some(x.width)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| Error::UndefinedPort(port_to_find.to_string()))
        };
        let src_width =
            find_width(src_port, &self.graph[src_node].signature.outputs)?;
        let dest_width =
            find_width(dest_port, &self.graph[dest_node].signature.inputs)?;

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

    pub fn remove_edge<S: AsRef<str>, U: AsRef<str>>(
        &mut self,
        src_node: NodeIndex,
        src_port: S,
        dest_node: NodeIndex,
        dest_port: U,
    ) -> Result<(), Error> {
        let edge_idx = self.graph.edge_indices()
            .filter_map(|eidx| self.graph.edge_endpoints(eidx))
            .filter(|(s, t)| s == &src_node && t == &dest_node)
            .find(|(s, t)| );
        Ok(())
            // (src_node, dest_node).find()
    }

    pub fn get(&self, idx: NodeIndex) -> &Node {
        &self.graph[idx]
    }

    pub fn get_idx(&self, port: &ast::Id) -> Result<NodeIndex, Error> {
        match self.nodes.get(port) {
            Some(idx) => Ok(*idx),
            None => Err(Error::UndefinedPort(port.to_string())),
        }
    }

    pub fn get_this(&self) -> NodeIndex {
        self.ports
    }

    /// Constructs a ast::Port from a NodeIndex for error reporting
    fn construct_port(&self, idx: NodeIndex, port: &str) -> ast::Port {
        let node = &self.graph[idx];
        match node.data {
            NodeData::Port => Port::This { port: port.into() },
            NodeData::Instance(..) | NodeData::Group(..) => Port::Comp {
                component: node.name.clone(),
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
        for (name, idx) in &self.nodes {
            match &self.graph[*idx].data {
                NodeData::Instance(data) => {
                    ret.push(data.clone());
                }
                NodeData::Group(comps) => ret
                    .push(ast::Structure::group(name.clone(), comps.to_vec())),
                _ => (),
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
