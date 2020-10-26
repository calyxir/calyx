use crate::ir::{self, RRC};
use petgraph::{graph::NodeIndex, Graph};
use std::collections::HashMap;
use std::fmt::Write;
use std::rc::Rc;

type GroupNode = RRC<ir::Group>;

#[derive(Default)]
/// A conflict graph that describes which groups are being run in parallel
/// to each other.
pub struct ScheduleConflicts {
    /// The conflict graph. Two groups have an edge between them if they
    /// may run in parallel.
    pub(self) conflicts: Graph<GroupNode, ()>,
    /// Map from the name of the group to its NodeIndex in the conflicts
    /// graph.
    pub(self) index_map: HashMap<ir::Id, NodeIndex>,
}

impl ScheduleConflicts {
    /// Returns the NodeIndex corresponding to this Group if present.
    pub fn find_index(&self, group: &GroupNode) -> Option<NodeIndex> {
        self.index_map.get(&group.borrow().name).cloned()
    }

    /// Returns the NodeIndex corresponding to this Group.
    /// Panics if the Group is not in the CurrentConflict.
    pub fn get_index(&self, group: &GroupNode) -> NodeIndex {
        self.index_map[&group.borrow().name]
    }

    /// Return a vector of all groups that conflict with this group.
    pub fn all_conflicts(&self, group: &GroupNode) -> Vec<GroupNode> {
        self.conflicts
            .neighbors_undirected(self.get_index(group))
            .into_iter()
            .map(|idx| Rc::clone(&self.conflicts[idx]))
            .collect()
    }

    /////////////// Internal Methods //////////////////
    /// Adds a node to the CurrentConflict set.
    pub(self) fn add_node(&mut self, group: &GroupNode) {
        if self.find_index(group).is_none() {
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
    groups: &Vec<Vec<GroupNode>>,
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
        ir::Control::Enable(ir::Enable { group }) => {
            confs.add_node(group);
            all_enables.push(Rc::clone(group));
        }
        ir::Control::Seq(ir::Seq { stmts }) => stmts
            .into_iter()
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
        ir::Control::Par(ir::Par { stmts }) => {
            let enables = stmts
                .into_iter()
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
        let mut out = String::new();
        for idx in self.conflicts.node_indices() {
            write!(
                &mut out,
                "{} -> {}",
                self.conflicts[idx].borrow().name,
                self.conflicts
                    .neighbors_undirected(idx)
                    .into_iter()
                    .map(|idx| self.conflicts[idx].borrow().name.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            )
            .expect("Failed to write to ScheduleConflicts string");
        }
        out
    }
}
