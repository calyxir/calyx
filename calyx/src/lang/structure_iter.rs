use super::ast;
use super::structure::{
    DataDirection, EdgeData, Node, NodeData, StructG, StructureGraph,
};
use petgraph::graph::{EdgeIndex, NodeIndex};

pub struct EdgeIndexIterator<'a> {
    graph: &'a StructG,
    indices: Box<dyn Iterator<Item = EdgeIndex> + 'a>,
    direction: Option<DataDirection>,
    node: Option<NodeIndex>,
    port: Option<String>,
    node_type: Option<NodeType>,
}

impl<'a> EdgeIndexIterator<'a> {
    pub fn new(
        graph: &'a StructG,
        indices: impl Iterator<Item = EdgeIndex> + 'a,
    ) -> Self {
        Self {
            graph,
            indices: Box::new(indices),
            direction: None,
            node: None,
            port: None,
            node_type: None,
        }
    }

    pub fn with_direction(mut self, direction: DataDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    pub fn with_node(mut self, node: NodeIndex) -> Self {
        self.node = Some(node);
        self
    }

    pub fn with_port(mut self, port: String) -> Self {
        self.port = Some(port);
        self
    }

    pub fn with_node_type(mut self, node_type: NodeType) -> Self {
        self.node_type = Some(node_type);
        self
    }

    pub fn detach(self) -> impl Iterator<Item = EdgeIndex> {
        self.collect::<Vec<EdgeIndex>>().into_iter()
    }
}

macro_rules! dir_match {
    ($dir:expr, $src:expr, $dest:expr, $weight:expr, $fun:expr) => {
        match $dir {
            Some(DataDirection::Read) => $fun($src, &$weight.src),
            Some(DataDirection::Write) => $fun($dest, &$weight.dest),
            None => $fun($src, &$weight.src) || $fun($dest, &$weight.dest),
        }
    };
}

impl Iterator for EdgeIndexIterator<'_> {
    type Item = EdgeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        // get next index or return none
        let idx = match self.indices.next() {
            Some(idx) => idx,
            None => return None,
        };

        let (src, dest) = &self
            .graph
            .edge_endpoints(idx)
            .expect("Edges should always have endpoints");

        // get endpoints
        let weight = &self.graph[idx];
        let dir = &self.direction;

        // // create direction matcher
        // let dir_match: Box<
        //     dyn Fn(Box<dyn Fn(NodeIndex, ast::Port) -> bool>) -> bool,
        // > = match self.direction {
        //     Some(DataDirection::Read) => {
        //         Box::new(|f: Box<dyn Fn(NodeIndex, ast::Port) -> bool>| {
        //             f(*src, weight.src.clone())
        //         })
        //     }
        //     Some(DataDirection::Write) => {
        //         Box::new(|f: Box<dyn Fn(NodeIndex, ast::Port) -> bool>| {
        //             f(*dest, weight.dest.clone())
        //         })
        //     }
        //     None => Box::new(|f: Box<dyn Fn(NodeIndex, ast::Port) -> bool>| {
        //         f(*src, weight.src.clone()) || f(*dest, weight.dest.clone())
        //     }),
        // };

        // compute conditions
        let node_cond = self
            .node
            .clone()
            .map(|check| {
                dir_match!(dir, *src, *dest, weight, |node, _| check == node)
            })
            .unwrap_or(true);

        let port_cond = self
            .port
            .clone()
            .map(|check| {
                dir_match!(dir, *src, *dest, weight, |_, port: &ast::Port| {
                    port.port_name() == check.as_str()
                })
            })
            .unwrap_or(true);

        let node_type_cond = self
            .node_type
            .clone()
            .map(|check| match check {
                NodeType::Cell => {
                    dir_match!(dir, *src, *dest, weight, |node, _| {
                        matches!(
                            self.graph.node_weight(node).unwrap().data,
                            NodeData::Cell(..)
                        )
                    })
                }
                NodeType::Constant => {
                    dir_match!(dir, *src, *dest, weight, |node, _| {
                        matches!(
                            self.graph.node_weight(node).unwrap().data,
                            NodeData::Constant(..)
                        )
                    })
                }
                NodeType::Hole => {
                    dir_match!(dir, *src, *dest, weight, |node, _| {
                        matches!(
                            self.graph.node_weight(node).unwrap().data,
                            NodeData::Hole(..)
                        )
                    })
                }
                NodeType::Port => {
                    dir_match!(dir, *src, *dest, weight, |node, _| {
                        matches!(
                            self.graph.node_weight(node).unwrap().data,
                            NodeData::Port
                        )
                    })
                }
            })
            .unwrap_or(true);

        if node_cond && port_cond && node_type_cond {
            Some(idx)
        } else {
            self.next()
        }
    }
}

/// Represents the type of a node for the purposes of filtering.
/// Unfortunately this can't be the same as `ast::Port`
/// because we can't ask the user to construct fake data just for
/// the purposes of filtering.
#[derive(Clone, Debug)]
pub enum NodeType {
    /// Filters for `Port::Comp`
    Cell,
    /// Filters for `Port::This`
    Constant,
    /// Filters for `Port::Hole`
    Hole,
    Port,
}

/// TODO(rachit): Implement filtering with group name.
#[derive(Clone, Debug)]
pub struct ConnectionIteration {
    /// Throw errors if trying to overwrite a field.
    guarded: bool,
    /// Only iterate over edges that have this particular node.
    pub from_node: Option<NodeIndex>,
    /// Only iterate over edges that have this particular port.
    pub with_port: Option<String>,
    /// Only iterate over edges where data flows from a port in the given
    /// direction.
    pub direction: Option<DataDirection>,
    /// Only iterate over edges incident to a cell.
    pub from_node_type: Option<NodeType>,
}

impl Default for ConnectionIteration {
    fn default() -> Self {
        ConnectionIteration {
            guarded: true,
            from_node: None,
            with_port: None,
            direction: None,
            from_node_type: None,
        }
    }
}

impl ConnectionIteration {
    /// Iterate over set of edges that contain this edge.
    pub fn with_component(mut self, component: NodeIndex) -> Self {
        if self.guarded && self.from_node.is_some() {
            panic!("Tried to overwrite with_component field in EdgeIterationBuilder")
        }
        self.from_node = Some(component);
        self
    }

    pub fn with_port(mut self, port: String) -> Self {
        if self.guarded && self.with_port.is_some() {
            panic!(
                "Tried to overwrite with_port field in EdgeIterationBuilder"
            );
        }
        self.with_port = Some(port);
        self
    }

    pub fn in_direction(mut self, direction: DataDirection) -> Self {
        if self.guarded && self.with_port.is_some() {
            panic!("Tried to overwrite direction field in EdgeIterationBuilder")
        }
        self.direction = Some(direction);
        self
    }

    pub fn with_node_type(mut self, node_type: NodeType) -> Self {
        if self.guarded && self.from_node_type.is_some() {
            panic!("Tried to overwrite node type field in EdgeIterationBuilder")
        }
        self.from_node_type = Some(node_type);
        self
    }

    /// Disable the guard checking for this iteration builder.
    pub fn disable_guard(mut self) -> Self {
        self.guarded = false;
        self
    }
}
