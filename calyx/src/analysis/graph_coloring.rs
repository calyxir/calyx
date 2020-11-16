use petgraph::{graph::NodeIndex, Graph};
use std::{collections::HashMap, hash::Hash};

pub struct GraphColoring<T: Eq + Hash> {
    graph: Graph<T, ()>,
    index_map: HashMap<T, NodeIndex>,
}

impl<T: Eq + Hash> Default for GraphColoring<T> {
    fn default() -> Self {
        GraphColoring {
            graph: Graph::new(),
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
            None => self.graph.add_node(a),
        };
        let b_node: NodeIndex = match self.index_map.get(&b) {
            Some(node) => *node,
            None => self.graph.add_node(b),
        };
        self.graph.add_edge(a_node, b_node, ());
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
