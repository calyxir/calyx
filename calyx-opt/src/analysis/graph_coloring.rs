use calyx_utils::{Idx, WeightGraph};
use itertools::Itertools;
use petgraph::algo;
use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
};

/// Defines a greedy graph coloring algorithm over a generic conflict graph.
pub struct GraphColoring<T> {
    graph: WeightGraph<T>,
    // color_freq_map records similar information as `all_colors` does in the
    // `color_greedy()` method, but `color_freq_map` stays alive after the
    // function call, and doesn't get rid of colors once they reach the bound
    color_freq_map: HashMap<Idx, i64>,
}

impl<T, C> From<C> for GraphColoring<T>
where
    T: Hash + Eq + Ord,
    C: Iterator<Item = T>,
{
    fn from(nodes: C) -> Self {
        let graph = WeightGraph::from(nodes);
        GraphColoring {
            graph,
            color_freq_map: HashMap::new(),
        }
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

    /// increases the frequency of `idx` in `color_freq_map` by one
    fn increase_freq(&mut self, idx: Idx) {
        self.color_freq_map
            .entry(idx)
            .and_modify(|v| *v += 1)
            .or_insert(1);
    }

    /// provides a hashmap that gives the sharing frequencies
    pub fn get_share_freqs(&mut self) -> HashMap<i64, i64> {
        let mut pdf: HashMap<i64, i64> = HashMap::new();
        // hold total value so we know how much to divide by at the end
        for value in self.color_freq_map.values() {
            // in`pdf`, each key represents a possible number of times a cell
            // is shared-- for example, x-- and the corresponding key represents
            // how many cells in the new program were shared x times
            pdf.entry(*value).and_modify(|v| *v += 1).or_insert(1);
        }
        pdf
    }

    /// Given an `ordering` of `T`s, find a mapping from nodes to `T`s such
    /// that no node has a neighbor with the same `T`.
    /// `keep_self_color` indicates whether to keep the mapping of the node to
    /// itself in the returned HashMap (since nodes are "colors")
    pub fn color_greedy(
        &mut self,
        bound: Option<i64>,
        keep_self_color: bool,
    ) -> HashMap<T, T> {
        let mut all_colors: BTreeMap<Idx, i64> = BTreeMap::new();
        let mut coloring: HashMap<Idx, Idx> = HashMap::new();
        let always_share = bound.is_none();
        // if we always_share is true, then we don't care about bound
        let bound_if_exists = if always_share { 0 } else { bound.unwrap() };

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
                    all_colors.keys().cloned().collect_vec();

                // every node will need a different color
                for nidx in scc.into_iter().sorted() {
                    if !available_colors.is_empty() {
                        let c = available_colors.remove(0);
                        coloring.insert(nidx, c);
                        self.increase_freq(c);
                        if let Some(num_used) = all_colors.get_mut(&c) {
                            *num_used += 1;
                            if !always_share && *num_used == bound_if_exists {
                                all_colors.remove(&c);
                            }
                        }
                    } else {
                        all_colors.insert(nidx, 1);
                        coloring.insert(nidx, nidx);
                        self.increase_freq(nidx);
                        if !always_share && bound_if_exists == 1 {
                            all_colors.remove(&nidx);
                        }
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
                        Some((c, _)) => {
                            coloring.insert(nidx, *c);
                            self.increase_freq(*c);
                            if let Some(num_used) = all_colors.get_mut(c) {
                                *num_used += 1;
                                if !always_share && *num_used == bound_if_exists
                                {
                                    all_colors.remove(c);
                                }
                            }
                        }
                        None => {
                            // use self as color if nothing else
                            all_colors.insert(nidx, 1);
                            coloring.insert(nidx, nidx);
                            self.increase_freq(nidx);
                            if !always_share && bound_if_exists == 1 {
                                all_colors.remove(&nidx);
                            }
                        }
                    };
                }
            }
        }

        let rev_map = self.graph.reverse_index();
        coloring
            .into_iter()
            .map(|(n1, n2)| (rev_map[&n1].clone(), rev_map[&n2].clone()))
            .filter(|(a, b)| (a != b) || keep_self_color)
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
        while !degree_ordering.is_empty() {
            let head = degree_ordering.remove(0);
            // eprintln!("{}", self.graph.degree(head));
            if !coloring.contains_key(head) {
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
