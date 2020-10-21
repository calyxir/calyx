use crate::{
    ir::{Assignment, Component, Direction, Id, Port, RRC},
    utils::Keyable,
};
use petgraph::{
    algo,
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
    Direction::{Incoming, Outgoing},
};
use std::{collections::HashMap, rc::Rc};

type Node = RRC<Port>;
type Edge = ();

/// A petgraph::DiGraph where ports are the nodes and edges contain no
/// information.
pub type CellGraph = DiGraph<Node, Edge>;

/// Implement keyable for port
impl Keyable for Port {
    type Key = (Id, Id);
    fn key(&self) -> Self::Key {
        (self.get_parent_name(), self.name.clone())
    }
}

/// Constructs a graph based representation of a component. Each port is
/// represented as a node, and each edge represents a read/write between ports.
///
/// For example:
///  ```
///  c.in = G[done] & b.done ? add.out
///  ```
/// creates the edges:
///   ```
///   add.out -> c.in
///   G[done] -> c.in
///   b.done -> c.in
///   ```
///
/// This representation is useful for asking graph based queries
/// such as all the reads from a port or all the write to a port.
#[derive(Clone)]
pub struct GraphAnalysis {
    nodes: HashMap<(Id, Id), NodeIndex>,
    graph: CellGraph,
}

impl GraphAnalysis {
    /// Construct a graph from a component. Ports are nodes
    /// and assignments are edges.
    pub fn from(component: &Component) -> Self {
        let mut graph = CellGraph::new();
        let mut nodes = HashMap::new();

        // helper for inserting ports and edges from an assignment
        let mut insert_asgn = |asgn: &Assignment| {
            // insert nodes for src and dst ports
            let src_key = asgn.src.borrow().key();
            let dst_key = asgn.dst.borrow().key();
            nodes
                .entry(src_key.clone())
                .or_insert_with(|| graph.add_node(Rc::clone(&asgn.src)));
            nodes
                .entry(dst_key.clone())
                .or_insert_with(|| graph.add_node(Rc::clone(&asgn.dst)));
            // add edge for the assignment
            let src_node = nodes[&src_key];
            let dst_node = nodes[&dst_key];
            graph.add_edge(src_node, dst_node, ());
            // add edges for guards that read from the port in the guard
            // and write to the dst of the assignment
            for port in &asgn.guard.all_ports() {
                let guard_key = port.borrow().key();
                nodes
                    .entry(guard_key.clone())
                    .or_insert_with(|| graph.add_node(Rc::clone(&port)));
                graph.add_edge(nodes[&guard_key], dst_node, ());
            }
        };

        // add edges and nodes for continuous assignments
        for asgn in &component.continuous_assignments {
            insert_asgn(asgn);
        }
        // add edges and nodes for all group assignments
        for group in &component.groups {
            for asgn in &group.borrow().assignments {
                insert_asgn(asgn);
            }
        }

        GraphAnalysis { nodes, graph }
    }

    /// Returns an iterator over all the reads from a port.
    /// Returns an empty iterator if this is an Input port.
    pub fn reads_from(&self, port: &Port) -> PortIterator<'_> {
        let idx = self.nodes[&port.key()];
        match port.direction {
            Direction::Input => PortIterator::empty(),
            Direction::Output | Direction::Inout => PortIterator {
                port_iter: Box::new(
                    self.graph.edges_directed(idx, Outgoing).map(move |edge| {
                        let node_idx =
                            self.graph.edge_endpoints(edge.id()).unwrap().1;
                        Rc::clone(&self.graph[node_idx])
                    }),
                ),
            },
        }
    }

    /// Returns an iterator over all the writes to this port.
    /// Returns an empty iterator if this is an Output port.
    pub fn writes_to(&self, port: &Port) -> PortIterator<'_> {
        let idx = self.nodes[&port.key()];
        match port.direction {
            Direction::Input | Direction::Inout => PortIterator {
                port_iter: Box::new(
                    self.graph.edges_directed(idx, Incoming).map(move |edge| {
                        let node_idx =
                            self.graph.edge_endpoints(edge.id()).unwrap().0;
                        Rc::clone(&self.graph[node_idx])
                    }),
                ),
            },
            Direction::Output => PortIterator::empty(),
        }
    }

    /// Restricts the analysis graph to only include edges and nodes
    /// that are specified by the `filter`.
    ///
    /// `filter` is passed references to the `src` and `dst` of each
    /// edge. When `filter(src, dst)` is `true`, then the edge between
    /// `src` and `dst` is kept. Otherwise, it is removed.
    pub fn edge_induced_subgraph<F>(self, mut filter: F) -> Self
    where
        F: FnMut(&Port, &Port) -> bool,
    {
        let Self { graph, nodes } = self;
        let graph = graph.filter_map(
            |_idx, node| Some(Rc::clone(node)),
            |idx, edge| {
                let (src_idx, dst_idx) = graph.edge_endpoints(idx).unwrap();
                if filter(&graph[src_idx].borrow(), &graph[dst_idx].borrow()) {
                    Some(*edge)
                } else {
                    None
                }
            },
        );
        Self { graph, nodes }
    }

    /// Checks if there are cycles in the analysis graph.
    pub fn has_cycles(&self) -> bool {
        algo::is_cyclic_directed(&self.graph)
    }
}

/// An iterator over ports. Wraps generic iterators
/// over ports to allow functions to build and return
/// port iterators in different ways.
pub struct PortIterator<'a> {
    port_iter: Box<dyn Iterator<Item = RRC<Port>> + 'a>,
}

impl PortIterator<'_> {
    /// Returns an empty iterator over ports.
    fn empty() -> Self {
        PortIterator {
            port_iter: Box::new(vec![].into_iter()),
        }
    }
}

impl Iterator for PortIterator<'_> {
    type Item = RRC<Port>;

    fn next(&mut self) -> Option<Self::Item> {
        self.port_iter.next()
    }
}
