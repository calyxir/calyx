use itertools::Itertools;
use petgraph::{
    graph::{NodeIndex, UnGraph},
    Graph,
};
use std::{collections::HashMap, hash::Hash};

pub struct GraphColoring<T: Eq + Hash> {
    graph: UnGraph<T, ()>,
    index_map: HashMap<T, NodeIndex>,
}

impl<T: Eq + Hash> Default for GraphColoring<T> {
    fn default() -> Self {
        GraphColoring {
            graph: Graph::new_undirected(),
            index_map: HashMap::new(),
        }
    }
}

impl<T: Eq + Hash + Clone + std::fmt::Debug> GraphColoring<T> {
    pub fn insert_conflict(&mut self, a: T, b: T) {
        // we don't need to add self edges, but we still want the node
        if a == b {
            if !self.index_map.contains_key(&a) {
                self.index_map.insert(a.clone(), self.graph.add_node(a));
            }
            return;
        }

        let a_node: NodeIndex = match self.index_map.get(&a) {
            Some(node) => *node,
            None => self.graph.add_node(a.clone()),
        };
        let b_node: NodeIndex = match self.index_map.get(&b) {
            Some(node) => *node,
            None => self.graph.add_node(b.clone()),
        };
        self.index_map.insert(a, a_node);
        self.index_map.insert(b, b_node);
        self.graph.update_edge(a_node, b_node, ());
    }

    pub fn insert_conflicts(&mut self, items: &[T]) {
        for a in items {
            for b in items {
                self.insert_conflict(a.clone(), b.clone());
            }
        }
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

impl<T: Eq + Hash + ToString> ToString for GraphColoring<T> {
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
}
