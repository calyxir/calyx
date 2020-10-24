use crate::ir::{self, RRC};
use petgraph::{graph::NodeIndex, Graph};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
/// A conflict graph that describes which groups are being run in parallel
/// to each other.
pub struct ScheduleConflicts {
    /// The conflict graph. Two groups have an edge between them if they
    /// may run in parallel.
    pub(self) conflicts: Graph<RRC<ir::Group>, ()>,
    /// Map from the name of the group to its NodeIndex in the conflicts
    /// graph.
    pub(self) index_map: HashMap<ir::Id, NodeIndex>,
}

impl ScheduleConflicts {
    /// Returns the NodeIndex corresponding to this Group if present.
    pub fn find_index(&self, group: &RRC<ir::Group>) -> Option<NodeIndex> {
        self.index_map.get(&group.borrow().name).cloned()
    }

    /// Returns the NodeIndex corresponding to this Group.
    /// Panics if the Group is not in the CurrentConflict.
    pub fn get_index(&self, group: RRC<ir::Group>) -> NodeIndex {
        self.index_map[&group.borrow().name]
    }

    /////////////// Internal Methods //////////////////
    /// Adds a node to the CurrentConflict set.
    pub(self) fn add_node(&mut self, group: &RRC<ir::Group>) {
        if self.find_index(group).is_none() {
            let idx = self.conflicts.add_node(Rc::clone(group));
            self.index_map.insert(group.borrow().name.clone(), idx);
        }
    }
}

/// Adds conflict edges between every node in the vector.
fn all_conflicting(
    mut nodes: Vec<RRC<ir::Group>>,
    current_conflicts: &mut ScheduleConflicts,
) {
    while let Some(node1) = nodes.pop() {
        for node2 in &nodes {
            current_conflicts.conflicts.add_edge(
                current_conflicts.get_index(Rc::clone(&node1)),
                current_conflicts.get_index(Rc::clone(node2)),
                (),
            );
        }
    }
}

/// Construct a conflict graph by traversing the Control program.
fn build_conflict_graph(
    c: &ir::Control,
    confs: &mut ScheduleConflicts,
    all_enables: &mut Vec<RRC<ir::Group>>,
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
            let mut enables = stmts
                .into_iter()
                .flat_map(|c| {
                    let mut enables = Vec::new();
                    // Visit this child and add conflict edges.
                    // Collect the enables in this into a new vector.
                    build_conflict_graph(c, confs, &mut enables);
                    // Add conflict edges between all children.
                    all_conflicting(enables.clone(), confs);
                    enables
                })
                .collect::<Vec<_>>();

            // Add the enables from visiting the children to the current
            // set of enables.
            all_enables.append(&mut enables);
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
