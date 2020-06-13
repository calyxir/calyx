use petgraph::stable_graph::StableDiGraph;
use petgraph::graph::{EdgeIndex, NodeIndex};
use std::collections::HashMap;
use std::ops::{Index, IndexMut};

/// A GroupedDiGraph is a directed graph where edges weights have a label
/// which denote the "group" to which they belong. In addition to the
/// normal graph iteration patterns, GroupedDiGraphs allow iteration
/// using a group name.
///
/// All operations (addition and removal of edges) preserve the mapping
/// of groups.
pub struct GroupedDiGraph<N, E, G> {
    graph: StableDiGraph<N, E>,
    groups: HashMap<G, Vec<EdgeIndex>>
}

impl<N, E, G> GroupedDiGraph<N, E, G> {

    pub fn add_node(&mut self, node: N) -> NodeIndex {
        self.graph.add_node(node)
    }
}

impl<N, E, G> Index<NodeIndex> for GroupedDiGraph<N, E, G> {

    type Output = N;

    fn index(&self, idx: NodeIndex) -> &N {
        &self.graph[idx]
    }
}

impl<N, E, G> Index<EdgeIndex> for GroupedDiGraph<N, E, G> {

    type Output = E;

    fn index(&self, idx: EdgeIndex) -> &E {
        &self.graph[idx]
    }
}
