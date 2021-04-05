use crate::utils::{Idx, WeightGraph};
use itertools::Itertools;
use petgraph::algo;
use std::{
    collections::{BTreeSet, HashMap},
    hash::Hash,
};

/// Defines a greedy graph coloring algorithm over a generic conflict graph.
pub struct GraphColoring<T> {
    graph: WeightGraph<T>,
}

impl<T, C> From<C> for GraphColoring<T>
where
    T: Hash + Eq + Ord,
    C: Iterator<Item = T>,
{
    fn from(nodes: C) -> Self {
        let graph = WeightGraph::from(nodes);
        GraphColoring { graph }
    }
}

impl<'a, T> GraphColoring<T>
where
    T: 'a + Eq + Hash + Clone + Ord,
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

    pub fn has_nodes(&self) -> bool {
        self.graph.graph.node_count() > 0
    }

    /// Given an `ordering` of `T`s, find a mapping from nodes to `T`s such
    /// that no node has a neighbor with the same `T`.
    pub fn color_greedy(&self) -> HashMap<T, T> {
        let mut all_colors: BTreeSet<Idx> = BTreeSet::new();
        let mut coloring: HashMap<Idx, Idx> = HashMap::new();

        // get strongly get components of graph
        let sccs = algo::tarjan_scc(&self.graph.graph);
        // sort strongly components from largest to smallest
        for scc in sccs.into_iter().sorted_by(|a, b| b.len().cmp(&a.len())) {
            // check if graph component is complete
            let is_complete = scc.iter().all(|&idx| {
                self.graph.graph.neighbors(idx).count() == scc.len() - 1
            });
            // if graph is complete, then every node needs a new color. so there's no reason to
            // check neighbors
            if is_complete {
                let mut available_colors: Vec<_> =
                    all_colors.iter().cloned().collect_vec();

                // every node with need a different color
                for nidx in scc.into_iter().sorted() {
                    if available_colors.len() > 0 {
                        coloring.insert(nidx, available_colors.remove(0));
                    } else {
                        all_colors.insert(nidx);
                        coloring.insert(nidx, nidx);
                    }
                }
            } else {
                for nidx in scc.into_iter().sorted() {
                    let mut available_colors = all_colors.clone();
                    // search neighbors for used colors
                    for item in self.graph.graph.neighbors(nidx) {
                        // if the neighbor is already colored
                        if coloring.contains_key(&item) {
                            // remove it from the available colors
                            available_colors.remove(&coloring[&item]);
                        }
                    }

                    let color = available_colors.iter().next();
                    match color {
                        Some(c) => coloring.insert(nidx, *c),
                        None => {
                            // use self as color if nothing else
                            all_colors.insert(nidx);
                            coloring.insert(nidx, nidx)
                        }
                    };
                }
            }
        }

        let rev_map = self.graph.reverse_index();
        coloring
            .into_iter()
            .map(|(n1, n2)| (rev_map[&n1].clone(), rev_map[&n2].clone()))
            .filter(|(a, b)| a != b)
            .collect()
    }

    pub fn welsh_powell_coloring(&self) -> HashMap<T, T> {
        let mut coloring: HashMap<T, T> = HashMap::new();

        let mut degree_ordering: Vec<&T> = self
            .graph
            .nodes()
            .sorted()
            .sorted_by(|a, b| self.graph.degree(b).cmp(&self.graph.degree(a)))
            .collect();

        let rev_map = self.graph.reverse_index();
        while degree_ordering.len() > 0 {
            let head = degree_ordering.remove(0);
            // eprintln!("{}", self.graph.degree(head));
            if !coloring.contains_key(&head) {
                coloring.insert(head.clone(), head.clone());
                for &node in &degree_ordering {
                    if coloring.contains_key(node) {
                        continue;
                    }
                    if !self
                        .graph
                        .graph
                        .neighbors(self.graph.index_map[node])
                        .any(|x| coloring.get(&rev_map[&x]) == Some(head))
                    {
                        coloring.insert(node.clone(), head.clone());
                    }
                }
            }
        }

        coloring
    }
}

impl<T: Eq + Hash + ToString + Clone + Ord> ToString for GraphColoring<T> {
    fn to_string(&self) -> String {
        self.graph.to_string()
    }
}
