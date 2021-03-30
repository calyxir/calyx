use crate::utils::{Idx, WeightGraph};
use std::{collections::HashMap, hash::Hash};

/// Defines a greedy graph coloring algorithm over a generic conflict graph.
pub struct GraphColoring<T> {
    graph: WeightGraph<T>,
}

impl<T, C> From<C> for GraphColoring<T>
where
    T: Hash + Eq,
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
    pub fn color_greedy_with<F>(
        &self,
        ordering: impl Iterator<Item = T>,
        color_equiv: F,
    ) -> HashMap<T, T>
    where
        F: Fn(&T, &T) -> bool,
    {
        // false = available, true = in use
        let mut colors_in_use: Vec<(Idx, bool)> =
            Vec::with_capacity(self.graph.index_map.len());
        let mut coloring: HashMap<Idx, Idx> = HashMap::new();
        let rev_map = self.graph.reverse_index();

        // for every node in the ordering
        for node in ordering {
            let nidx = self.graph.index_map[&node];
            // reset available colors: if colors not equivalent, set the color as being in use
            colors_in_use.iter_mut().for_each(|(n, b)| {
                *b = !color_equiv(&node, &rev_map[n]);
            });

            // search neighbors for used colors
            for item in self.graph.graph.neighbors(self.graph.index_map[&node])
            {
                // if the neighbor is already colored
                if coloring.contains_key(&item) {
                    // set color to be in use
                    colors_in_use.iter_mut().for_each(|(x, b)| {
                        if x == &coloring[&item] {
                            *b = true
                        }
                    });
                }
            }
            // find the first available color
            let color =
                colors_in_use.iter().find_map(
                    |(x, b)| {
                        if !*b {
                            Some(x)
                        } else {
                            None
                        }
                    },
                );
            match color {
                Some(c) => coloring.insert(nidx, *c),
                None => {
                    // use self as color if nothing else
                    colors_in_use.push((nidx, true));
                    coloring.insert(nidx, nidx)
                }
            };
        }

        coloring
            .into_iter()
            .map(|(n1, n2)| (rev_map[&n1].clone(), rev_map[&n2].clone()))
            .collect()
    }
}

impl<T: Eq + Hash + ToString + Clone> ToString for GraphColoring<T> {
    fn to_string(&self) -> String {
        // self.graph.to_string()
        format!(
            "nodes: {:?}",
            self.graph
                .index_map
                .keys()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
        )
    }
}
