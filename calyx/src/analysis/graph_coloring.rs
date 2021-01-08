use crate::utils::{Idx, WeightGraph};
use std::{collections::HashMap, hash::Hash};

/// Defines a greedy graph coloring algorithm over a generic conflict graph.
pub struct GraphColoring<T: Eq + Hash> {
    graph: WeightGraph<T>,
}

impl<T, C> From<C> for GraphColoring<T>
where
    T: Eq + Hash,
    C: Iterator<Item = T>,
{
    fn from(nodes: C) -> Self {
        let graph = WeightGraph::from(nodes);
        GraphColoring { graph }
    }
}

impl<'a, T> GraphColoring<T>
where
    T: 'a + Eq + Hash + Clone,
{
    /// Add a conflict edge between `a` and `b`.
    #[inline(always)]
    pub fn insert_conflict(&mut self, a: &T, b: &T) {
        self.graph.add_edge(a, b);
    }

    /// Add conflict edges between all given items.
    pub fn insert_conflicts<C>(&mut self, items: C)
    where
        C: Iterator<Item = &'a T> + Clone,
    {
        self.graph.add_all_edges(items)
    }

    /// Given an `ordering` of `T`s, find a mapping from nodes to `T`s such
    /// that no node has a neighbor with the same `T`.
    pub fn color_greedy_with(
        &self,
        ordering: impl Iterator<Item = T>,
    ) -> HashMap<T, T> {
        let mut available_colors: Vec<(Idx, bool)> = Vec::new();
        let mut coloring: HashMap<Idx, Idx> = HashMap::new();

        // for every node in the ordering
        for node in ordering {
            let nidx = self.graph.index_map[&node];
            // reset available colors
            available_colors.iter_mut().for_each(|(_, b)| *b = false);

            // search neighbors for used colors
            for item in self.graph.graph.neighbors(self.graph.index_map[&node])
            {
                //let item = &self.graph[nbr];
                // if the neighbor is already colored
                if coloring.contains_key(&item) {
                    // set color to be in use
                    available_colors.iter_mut().for_each(|(x, b)| {
                        if x == &coloring[&item] {
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
                Some(c) => coloring.insert(nidx, *c),
                None => {
                    // use self as color if nothing else
                    available_colors.push((nidx, true));
                    coloring.insert(nidx, nidx)
                }
            };
        }

        let rev_map = self.graph.reverse_index();
        coloring
            .into_iter()
            .map(|(n1, n2)| (rev_map[&n1].clone(), rev_map[&n2].clone()))
            .collect()
    }
}
