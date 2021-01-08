use crate::ir::{self, RRC};
use itertools::Itertools;
use petgraph::{graph::NodeIndex, Graph};
use std::collections::HashMap;
use std::rc::Rc;

type GroupNode = RRC<ir::Group>;

#[derive(Default)]
/// A conflict graph that describes which groups are being run in parallel
/// to each other.
pub struct ScheduleConflicts {
    /// The conflict graph. Two groups have an edge between them if they
    /// may run in parallel.
    conflicts: Graph<GroupNode, ()>,
    /// Map from the name of the group to its NodeIndex in the conflicts
    /// graph.
    index_map: HashMap<ir::Id, NodeIndex>,
}

/// Wrapper to iterate over all the conflict edges.
pub struct ConflictIterator<'a> {
    iter: Box<dyn Iterator<Item = (GroupNode, GroupNode)> + 'a>,
}

impl Iterator for ConflictIterator<'_> {
    type Item = (GroupNode, GroupNode);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl ScheduleConflicts {
    /// Return a vector of all groups that conflict with this group.
    pub fn conflicts_with(&self, group: &GroupNode) -> Vec<GroupNode> {
        self.conflicts
            .neighbors_undirected(self.get_index(group))
            .into_iter()
            .map(|idx| Rc::clone(&self.conflicts[idx]))
            .collect()
    }

    /// Returns a vector containing all conflict edges in this graph.
    pub fn all_conflicts(&self) -> ConflictIterator<'_> {
        let iter = self.conflicts.edge_indices().map(move |edge| {
            let (src, dst) = self.conflicts.edge_endpoints(edge).unwrap();
            (
                Rc::clone(&self.conflicts[src]),
                Rc::clone(&self.conflicts[dst]),
            )
        });

        ConflictIterator {
            iter: Box::new(iter),
        }
    }

    /////////////// Internal Methods //////////////////
    /// Returns the NodeIndex corresponding to this Group.
    /// Panics if the Group is not in the CurrentConflict.
    fn get_index(&self, group: &GroupNode) -> NodeIndex {
        *self.index_map.get(&group.borrow().name).unwrap_or_else(|| panic!("No index for group `{}' in conflict graph. Is the group used in the control program?", group.borrow().name))
    }

    /// Adds a node to the CurrentConflict set.
    fn add_node(&mut self, group: &GroupNode) {
        if self.index_map.get(&group.borrow().name).is_none() {
            let idx = self.conflicts.add_node(Rc::clone(group));
            self.index_map.insert(group.borrow().name.clone(), idx);
        }
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
    groups: &[Vec<GroupNode>],
    current_conflicts: &mut ScheduleConflicts,
) {
    for group1 in 0..groups.len() {
        for group2 in group1 + 1..groups.len() {
            for node1 in &groups[group1] {
                for node2 in &groups[group2] {
                    current_conflicts.conflicts.add_edge(
                        current_conflicts.get_index(node1),
                        current_conflicts.get_index(node2),
                        (),
                    );
                }
            }
        }
    }
}

/// Construct a conflict graph by traversing the Control program.
fn build_conflict_graph(
    c: &ir::Control,
    confs: &mut ScheduleConflicts,
    all_enables: &mut Vec<GroupNode>,
) {
    match c {
        ir::Control::Empty(_) => (),
        ir::Control::Invoke(_) => unimplemented!(),
        ir::Control::Enable(ir::Enable { group, .. }) => {
            confs.add_node(group);
            all_enables.push(Rc::clone(group));
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
            all_enables.push(Rc::clone(cond));
            confs.add_node(cond);
            build_conflict_graph(tbranch, confs, all_enables);
            build_conflict_graph(fbranch, confs, all_enables);
        }
        ir::Control::While(ir::While { cond, body, .. }) => {
            all_enables.push(Rc::clone(cond));
            confs.add_node(cond);
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
        confs
    }
}

impl ToString for ScheduleConflicts {
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
            .conflicts
            .edge_indices()
            .filter_map(|idx| self.conflicts.edge_endpoints(idx))
            .unique()
            .map(|(a_idx, b_idx)| {
                format!(
                    "  {} -- {};",
                    self.conflicts[a_idx].borrow().name.to_string(),
                    self.conflicts[b_idx].borrow().name.to_string()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("graph {{ \n{}\n{}\n }}", nodes, edges)
    }
}
