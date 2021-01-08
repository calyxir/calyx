use itertools::Itertools;
use petgraph::matrix_graph::{MatrixGraph, NodeIndex, UnMatrix, Zero};
use std::{collections::HashMap, hash::Hash};

/// Edge weight used for the graph nodes
struct NonZeroBool(bool);

impl From<bool> for NonZeroBool {
    fn from(b: bool) -> Self {
        NonZeroBool(b)
    }
}

impl Zero for NonZeroBool {
    fn zero() -> Self {
        NonZeroBool(false)
    }

    fn is_zero(&self) -> bool {
        !self.0
    }
}

/// Defines a greedy graph coloring algorithm over a generic conflict graph.
pub struct GraphColoring<T: Eq + Hash> {
    /// Graph representating using identifier.
    graph: UnMatrix<T, NonZeroBool>,
    /// Mapping from T to a unique identifier.
    index_map: HashMap<T, NodeIndex>,
}

impl<T, C> From<C> for GraphColoring<T>
where
    T: Eq + Hash + Clone,
    C: Iterator<Item = T>,
{
    fn from(nodes: C) -> Self {
        let mut graph = MatrixGraph::new_undirected();
        let index_map = nodes
            .map(|node| (node.clone(), graph.add_node(node.clone())))
            .collect();
        GraphColoring { graph, index_map }
    }
}

impl<'a, T: 'a + Eq + Hash + Clone + std::fmt::Debug> GraphColoring<T> {
    /// Add a conflict edge between `a` and `b`.
    #[inline(always)]
    pub fn insert_conflict(&mut self, a: &T, b: &T) {
        self.graph.update_edge(
            self.index_map[a],
            self.index_map[b],
            true.into(),
        );
    }

    /// Add conflict edges between all given items.
    pub fn insert_conflicts<C>(&mut self, items: C)
    where
        C: Iterator<Item = &'a T> + Clone,
    {
        items.tuple_combinations().for_each(|(src, dst)| {
            self.insert_conflict(src, dst);
        });
    }

    /// Given an `ordering` of `T`s, find a mapping from nodes to `T`s such
    /// that no node has a neighbor with the same `T`.
    pub fn color_greedy_with(
        &self,
        ordering: impl Iterator<Item = T>,
    ) -> HashMap<T, T> {
        let mut available_colors: Vec<(T, bool)> = Vec::new();
        let mut coloring: HashMap<T, T> = HashMap::new();

        // for every node in the ordering
        for node in ordering {
            // reset available colors
            available_colors.iter_mut().for_each(|(_, b)| *b = false);

            // search neighbors for used colors
            for nbr in self.graph.neighbors(self.index_map[&node]) {
                let item = &self.graph[nbr];
                // if the neighbor is already colored
                if coloring.contains_key(item) {
                    // set color to be in use
                    available_colors.iter_mut().for_each(|(x, b)| {
                        if x == &coloring[item] {
                            *b = true
                        }
                    });
                }
            }
            // find the first available color
            let color = available_colors.iter().find_map(|(x, b)| {
                if !*b {
                    Some(x)
                } else {
                    None
                }
            });
            match color {
                Some(c) => coloring.insert(node.clone(), c.clone()),
                None => {
                    // use self as color if nothing else
                    available_colors.push((node.clone(), true));
                    coloring.insert(node.clone(), node.clone())
                }
            };
        }

        coloring
    }
}

/*impl<T: Eq + Hash + ToString> ToString for GraphColoring<T> {
    fn to_string(&self) -> String {
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
            .edge_indices()
            .filter_map(|idx| self.graph.edge_endpoints(idx))
            .unique()
            .map(|(a_idx, b_idx)| {
                format!(
                    "  {} -- {};",
                    self.graph[a_idx].to_string(),
                    self.graph[b_idx].to_string()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("graph {{ \n{}\n{}\n }}", nodes, edges)
    }
}*/
