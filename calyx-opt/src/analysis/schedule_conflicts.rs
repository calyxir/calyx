use calyx_ir as ir;
use calyx_utils::{Idx, WeightGraph};
use petgraph::visit::IntoEdgeReferences;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
/// A conflict graph that describes which nodes (i.e. groups/invokes) are being run in parallel
/// to each other.
pub struct ScheduleConflicts {
    graph: WeightGraph<ir::Id>,
    /// Reverse mapping from node indices to node (i.e. group/invoke) names.
    /// We can store this because we don't expect nodes or edges to be added
    /// once a conflict graph is constructed.
    rev_map: HashMap<Idx, ir::Id>,
}

/// A conflict between two nodes is specified using the name of the nodes
/// involved
type Conflict = (ir::Id, ir::Id);

impl ScheduleConflicts {
    /// Return a vector of all nodes that conflict with this nodes.
    pub fn conflicts_with(&self, node: &ir::Id) -> HashSet<ir::Id> {
        self.graph
            .graph
            .neighbors(self.graph.index_map[node])
            .map(|idx| self.rev_map[&idx])
            .collect()
    }

    /// Returns an iterator containing all conflict edges,
    /// `(src node: ir::Id, dst node: ir::Id)`, in this graph.
    pub fn all_conflicts(&self) -> impl Iterator<Item = Conflict> + '_ {
        self.graph
            .graph
            .edge_references()
            .map(move |(src, dst, _)| (self.rev_map[&src], self.rev_map[&dst]))
    }

    /////////////// Internal Methods //////////////////
    /// Adds a node to the CurrentConflict set.
    fn add_node(&mut self, node: ir::Id) {
        if !self.graph.contains_node(&node) {
            self.graph.add_node(node)
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

fn build_conflict_graph_static(
    sc: &ir::StaticControl,
    confs: &mut ScheduleConflicts,
    all_nodes: &mut Vec<ir::Id>,
) {
    match sc {
        ir::StaticControl::Enable(ir::StaticEnable { group, .. }) => {
            confs.add_node(group.borrow().name());
            all_nodes.push(group.borrow().name());
        }
        ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
            build_conflict_graph_static(body, confs, all_nodes);
        }
        ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => stmts
            .iter()
            .for_each(|c| build_conflict_graph_static(c, confs, all_nodes)),
        ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
            let par_nodes = stmts
                .iter()
                .map(|c| {
                    // Visit this child and add conflict edges.
                    // Collect the enables in this into a new vector.
                    let mut nodes = Vec::new();
                    build_conflict_graph_static(c, confs, &mut nodes);
                    nodes
                })
                .collect::<Vec<_>>();

            // Add conflict edges between all children.
            all_conflicting(&par_nodes, confs);

            // Add the enables from visiting the children to the current
            // set of enables.
            all_nodes.append(&mut par_nodes.into_iter().flatten().collect());
        }
        ir::StaticControl::If(ir::StaticIf {
            tbranch, fbranch, ..
        }) => {
            build_conflict_graph_static(tbranch, confs, all_nodes);
            build_conflict_graph_static(fbranch, confs, all_nodes);
        }
        ir::StaticControl::Invoke(ir::StaticInvoke { comp, .. }) => {
            confs.add_node(comp.borrow().name());
            all_nodes.push(comp.borrow().name());
        }
        ir::StaticControl::Empty(_) => (),
    }
}
/// Construct a conflict graph by traversing the Control program.
fn build_conflict_graph(
    c: &ir::Control,
    confs: &mut ScheduleConflicts,
    all_nodes: &mut Vec<ir::Id>,
) {
    match c {
        ir::Control::Empty(_) => (),
        ir::Control::Invoke(ir::Invoke { comp, .. }) => {
            confs.add_node(comp.borrow().name());
            all_nodes.push(comp.borrow().name());
        }
        ir::Control::Enable(ir::Enable { group, .. }) => {
            confs.add_node(group.borrow().name());
            all_nodes.push(group.borrow().name());
        }
        ir::Control::Seq(ir::Seq { stmts, .. }) => stmts
            .iter()
            .for_each(|c| build_conflict_graph(c, confs, all_nodes)),
        ir::Control::If(ir::If {
            cond,
            tbranch,
            fbranch,
            ..
        }) => {
            // XXX (rachit): This might be incorrect since cond is a combinational
            // group
            if let Some(c) = cond {
                all_nodes.push(c.borrow().name());
                confs.add_node(c.borrow().name());
            }
            build_conflict_graph(tbranch, confs, all_nodes);
            build_conflict_graph(fbranch, confs, all_nodes);
        }
        ir::Control::While(ir::While { cond, body, .. }) => {
            // XXX (rachit): This might be incorrect since cond is a combinational
            // group
            if let Some(c) = cond {
                all_nodes.push(c.borrow().name());
                confs.add_node(c.borrow().name());
            }
            build_conflict_graph(body, confs, all_nodes);
        }
        ir::Control::Repeat(ir::Repeat { body, .. }) => {
            build_conflict_graph(body, confs, all_nodes);
        }
        ir::Control::Par(ir::Par { stmts, .. }) => {
            let par_nodes = stmts
                .iter()
                .map(|c| {
                    // Visit this child and add conflict edges.
                    // Collect the enables in this into a new vector.
                    let mut nodes = Vec::new();
                    build_conflict_graph(c, confs, &mut nodes);
                    nodes
                })
                .collect::<Vec<_>>();

            // Add conflict edges between all children.
            all_conflicting(&par_nodes, confs);

            // Add the enables from visiting the children to the current
            // set of enables.
            all_nodes.append(&mut par_nodes.into_iter().flatten().collect());
        }
        ir::Control::Static(sc) => {
            build_conflict_graph_static(sc, confs, all_nodes)
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
