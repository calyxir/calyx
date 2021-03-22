use crate::ir;
use crate::utils::{Idx, WeightGraph};
use petgraph::visit::IntoEdgeReferences;
use std::collections::HashMap;

#[derive(Default)]
/// A conflict graph that describes which groups are being run in parallel
/// to each other.
pub struct ScheduleConflicts {
    graph: WeightGraph<ir::Id>,
    /// Reverse mapping from node indices to Group names.
    /// We can store this because we don't expect nodes or edges to be added
    /// once a conflict graph is constructed.
    rev_map: HashMap<Idx, ir::Id>,
}

/// Wrapper to iterate over all the conflict edges.
pub struct ConflictIterator<'a> {
    iter: Box<dyn Iterator<Item = (ir::Id, ir::Id)> + 'a>,
}

impl Iterator for ConflictIterator<'_> {
    type Item = (ir::Id, ir::Id);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl ScheduleConflicts {
    /// Return a vector of all groups that conflict with this group.
    pub fn conflicts_with(&self, group: &ir::Id) -> Vec<&ir::Id> {
        self.graph
            .graph
            .neighbors(self.graph.index_map[group])
            .into_iter()
            .map(|idx| &self.rev_map[&idx])
            .collect()
    }

    /// Returns an iterator containing all conflict edges,
    /// `(src group: ir::Id, dst group: ir::Id)`, in this graph.
    pub fn all_conflicts(&self) -> ConflictIterator<'_> {
        let iter =
            self.graph
                .graph
                .edge_references()
                .map(move |(src, dst, _)| {
                    (self.rev_map[&src].clone(), self.rev_map[&dst].clone())
                });

        ConflictIterator {
            iter: Box::new(iter),
        }
    }

    /////////////// Internal Methods //////////////////
    /// Adds a node to the CurrentConflict set.
    fn add_node(&mut self, group: &ir::Id) {
        if !self.graph.contains_node(group) {
            self.graph.add_node(group.clone())
        }
    }

    fn add_edge(&mut self, g1: &ir::Id, g2: &ir::Id) {
        self.graph.add_edge(g1, g2)
    }
}

/// Given a set of vectors of nodes, adds edges between all nodes in one
/// vector to all nodes in every other vector.
///
/// For example:
/// ```
/// vec![
///     vec!["a", "b"],
///     vec!["c", "d"]
/// ]
/// ```
/// will create the edges:
/// ```
/// a --- c
/// b --- c
/// a --- d
/// b --- d
/// ```
fn all_conflicting(
    groups: &[Vec<ir::Id>],
    current_conflicts: &mut ScheduleConflicts,
) {
    for group1 in 0..groups.len() {
        for group2 in group1 + 1..groups.len() {
            for node1 in &groups[group1] {
                for node2 in &groups[group2] {
                    current_conflicts.add_edge(node1, node2);
                }
            }
        }
    }
}

/// Construct a conflict graph by traversing the Control program.
fn build_conflict_graph(
    c: &ir::Control,
    confs: &mut ScheduleConflicts,
    all_enables: &mut Vec<ir::Id>,
) {
    match c {
        ir::Control::Empty(_) => (),
        ir::Control::Invoke(_) => unimplemented!(),
        ir::Control::Enable(ir::Enable { group, .. }) => {
            confs.add_node(&group.borrow().name);
            all_enables.push(group.borrow().name.clone());
        }
        ir::Control::Seq(ir::Seq { stmts, .. }) => stmts
            .iter()
            .for_each(|c| build_conflict_graph(c, confs, all_enables)),
        ir::Control::If(ir::If {
            cond,
            tbranch,
            fbranch,
            ..
        }) => {
            all_enables.push(cond.borrow().name.clone());
            confs.add_node(&cond.borrow().name);
            build_conflict_graph(tbranch, confs, all_enables);
            build_conflict_graph(fbranch, confs, all_enables);
        }
        ir::Control::While(ir::While { cond, body, .. }) => {
            all_enables.push(cond.borrow().name.clone());
            confs.add_node(&cond.borrow().name);
            build_conflict_graph(body, confs, all_enables);
        }
        ir::Control::Par(ir::Par { stmts, .. }) => {
            let enables = stmts
                .iter()
                .map(|c| {
                    // Visit this child and add conflict edges.
                    // Collect the enables in this into a new vector.
                    let mut enables = Vec::new();
                    build_conflict_graph(c, confs, &mut enables);
                    enables
                })
                .collect::<Vec<_>>();

            // Add conflict edges between all children.
            all_conflicting(&enables, confs);

            // Add the enables from visiting the children to the current
            // set of enables.
            all_enables.append(&mut enables.into_iter().flatten().collect());
        }
    }
}

/// Construct ScheduleConflicts from a ir::Control.
impl From<&ir::Control> for ScheduleConflicts {
    fn from(control: &ir::Control) -> Self {
        let mut confs = ScheduleConflicts::default();
        build_conflict_graph(control, &mut confs, &mut vec![]);
        // Build the reverse index
        confs.rev_map = confs.graph.reverse_index();
        confs
    }
}
