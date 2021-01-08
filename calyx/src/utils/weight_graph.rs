use itertools::Itertools;
use petgraph::matrix_graph::{MatrixGraph, NodeIndex, UnMatrix, Zero};
use std::{collections::HashMap, hash::Hash};

/// Edge weight used for the graph nodes
pub struct BoolIdx(bool);

impl From<bool> for BoolIdx {
    fn from(b: bool) -> Self {
        BoolIdx(b)
    }
}

impl Zero for BoolIdx {
    fn zero() -> Self {
        BoolIdx(false)
    }

    fn is_zero(&self) -> bool {
        !self.0
    }
}

/// Weight graph provides a wrapper over a Graph that allows adding edges using
/// the NodeWeight type `T` (petgraph only allows adding edges using `NodeIndex`).
/// Additionally, the edges are not allowed to have any weights.
///
/// The internal representation stores a mapping from NodeWeight `T` to a
/// `NodeIndex` in the graph.
/// The underlying `petgraph::MatrixGraph` stores `()` for node weights and
/// a boolean to represent the edges.
pub struct WeightGraph<T: Eq + Hash> {
    /// Mapping from T to a unique identifier.
    pub index_map: HashMap<T, NodeIndex>,
    /// Graph representating using identifier.
    pub graph: UnMatrix<(), BoolIdx>,
}

impl<T, C> From<C> for WeightGraph<T>
where
    T: Eq + Hash + Clone,
    C: Iterator<Item = T>,
{
    fn from(nodes: C) -> Self {
        let mut graph = MatrixGraph::new_undirected();
        let index_map = nodes
            .map(|node| (node, graph.add_node(())))
            .collect();
        WeightGraph { graph, index_map }
    }
}

impl<'a, T> WeightGraph<T>
where
    T: 'a + Eq + Hash,
{
    /// Add an edge between `a` and `b`.
    #[inline(always)]
    pub fn add_edge(&mut self, a: &T, b: &T) {
        self.graph.update_edge(
            self.index_map[a],
            self.index_map[b],
            true.into(),
        );
    }

    /// Add edges between all given items.
    pub fn add_all_edges<C>(&mut self, items: C)
    where
        C: Iterator<Item = &'a T> + Clone,
    {
        items.tuple_combinations().for_each(|(src, dst)| {
            self.add_edge(src, dst);
        });
    }

    /// Add a new node to the graph. Client code should ensure that duplicate
    /// edges are never added to graph.
    /// Instead of using this method, consider constructing the graph using
    /// `From<Iterator<T>>`.
    ///
    /// # Panics
    /// (Debug build only) Panics if node is already present in the graph
    pub fn add_node(&mut self, node: T) {
        debug_assert!(
            !self.index_map.contains_key(&node),
            "Attempted to add pre-existing node to WeightGraph. Client code should ensure that this never happens.");
        let idx = self.graph.add_node(());
        self.index_map.insert(node, idx);
    }
}
