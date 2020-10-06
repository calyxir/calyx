use super::ast;
use super::structure::{DataDirection, NodeData, StructG};
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

    /// Modifies the target of other filters.
    ///  * `DataDirection::Write` makes other filters reference the `dest` of an edge
    ///  * `DataDirection::Read` makes other filters reference the `src` of an edge
    pub fn with_direction(mut self, direction: DataDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Filters edges that have an `node` as an endpoint.
    pub fn with_node(mut self, node: NodeIndex) -> Self {
        self.node = Some(node);
        self
    }

    /// Filters edges that have `port` as an endpoint.
    pub fn with_port(mut self, port: String) -> Self {
        self.port = Some(port);
        self
    }

    /// Filters for nodes of `node_type: NodeType`.
    pub fn with_node_type(mut self, node_type: NodeType) -> Self {
        self.node_type = Some(node_type);
        self
    }

    /// Detaches the iterator from it's reference to `StructG`
    /// so that it is possible to get mutable references to `EdgeData`
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
                            NodeData::ThisPort
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
/// Unfortunately this can't be the same as `NodeData`
/// because we can't ask the user to construct fake data just for
/// the purposes of filtering.
#[derive(Clone, Debug)]
pub enum NodeType {
    /// Filters for `NodeData::Comp`
    Cell,
    /// Filters for `NodeData::Constant`
    Constant,
    /// Filters for `NodeData::Hole`
    Hole,
    /// Filters for `NodeData::ThisPort
    Port,
}
