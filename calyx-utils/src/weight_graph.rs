use itertools::Itertools;
use petgraph::matrix_graph::{MatrixGraph, NodeIndex, UnMatrix, Zero};
use petgraph::visit::IntoEdgeReferences;
use std::{collections::HashMap, hash::Hash};

/// Index into a [WeightGraph]
pub type Idx = NodeIndex;

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
pub struct WeightGraph<T> {
    /// Mapping from T to a unique identifier.
    pub index_map: HashMap<T, NodeIndex>,
    /// Graph representating using identifier.
    pub graph: UnMatrix<(), BoolIdx>,
}

impl<T: Eq + Hash + Clone + Ord> Default for WeightGraph<T> {
    fn default() -> Self {
        WeightGraph {
            index_map: HashMap::new(),
            graph: MatrixGraph::new_undirected(),
        }
    }
}

impl<T, C> From<C> for WeightGraph<T>
where
    T: Eq + Hash + Ord,
    C: Iterator<Item = T>,
{
    fn from(nodes: C) -> Self {
        let mut graph = MatrixGraph::new_undirected();
        let index_map: HashMap<_, _> =
            nodes.map(|node| (node, graph.add_node(()))).collect();
        WeightGraph { index_map, graph }
    }
}

impl<'a, T> WeightGraph<T>
where
    T: 'a + Eq + Hash + Clone + Ord,
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

    /// Checks if the node has already been added to the graph.
    #[inline(always)]
    pub fn contains_node(&self, node: &T) -> bool {
        self.index_map.contains_key(node)
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

    /// Returns a Map from `NodeIndex` to `T` (the reverse of the index)
    pub fn reverse_index(&self) -> HashMap<NodeIndex, T> {
        self.index_map
            .iter()
            .map(|(k, v)| (*v, k.clone()))
            .collect()
    }

    /// Returns an iterator over references to nodes in the Graph.
    pub fn nodes(&self) -> impl Iterator<Item = &T> {
        self.index_map.keys()
    }

    /// Return the degree of a given node (number of edges connected).
    pub fn degree(&self, node: &T) -> usize {
        self.graph.neighbors(self.index_map[node]).count()
    }
}

impl<T: Eq + Hash + ToString + Clone + Ord> ToString for WeightGraph<T> {
    fn to_string(&self) -> String {
        let rev_map = self.reverse_index();
        let keys: Vec<_> = self.index_map.keys().collect();
        let nodes = keys
            .iter()
            .enumerate()
            .map(|(_idx, key)| {
                format!(
                    "  {} [label=\"{}\"];",
                    key.to_string(),
                    key.to_string()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let edges = self
            .graph
            .edge_references()
            .map(|(a_idx, b_idx, _)| {
                format!(
                    "  {} -- {};",
                    rev_map[&a_idx].to_string(),
                    rev_map[&b_idx].to_string()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("graph {{ \n{}\n{}\n }}", nodes, edges)
    }
}
