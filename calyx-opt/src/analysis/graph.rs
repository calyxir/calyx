use calyx_ir::{self as ir, Id, PortIterator, RRC};
use petgraph::{
    algo,
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
    Direction::{Incoming, Outgoing},
};
use std::fmt::Write;
use std::{collections::HashMap, rc::Rc};

type Node = RRC<ir::Port>;
type Edge = ();

/// A petgraph::DiGraph where ports are the nodes and edges contain no
/// information.
pub type CellGraph = DiGraph<Node, Edge>;

/// Constructs a graph based representation of a component. Each node represents
/// a [`ir::Port`](calyx_ir::Port) and each directed edge (`X -> Y`) means
/// that `X`'s value written to `Y`.
///
/// # Example
///  ```
///  c.in = G[done] & b.done ? add.out
///  ```
/// creates the edges:
///  ```
///  add.out -> c.in
///  G[done] -> c.in
///  b.done -> c.in
///  ```
///
/// This representation is useful for asking graph based queries
/// such as all the reads from a port or all the write to a port.
#[derive(Clone, Default, Debug)]
pub struct GraphAnalysis {
    nodes: HashMap<ir::Canonical, NodeIndex>,
    graph: CellGraph,
}

impl From<&ir::Group> for GraphAnalysis {
    fn from(group: &ir::Group) -> Self {
        let mut analysis = GraphAnalysis::default();

        for asgn in &group.assignments {
            analysis.insert_assignment(asgn);
        }

        analysis
    }
}

impl From<&ir::Component> for GraphAnalysis {
    fn from(component: &ir::Component) -> Self {
        let mut analysis = GraphAnalysis::default();
        component.iter_assignments(|asgn| {
            analysis.insert_assignment(asgn);
        });
        component.iter_static_assignments(|asgn| {
            analysis.insert_assignment(asgn);
        });
        analysis
    }
}

impl GraphAnalysis {
    fn insert_assignment<T>(&mut self, asgn: &ir::Assignment<T>) {
        let GraphAnalysis { nodes, graph } = self;
        // insert nodes for src and dst ports
        let src_key = asgn.src.borrow().canonical();
        let dst_key = asgn.dst.borrow().canonical();
        let src_node = *nodes
            .entry(src_key)
            .or_insert_with(|| graph.add_node(Rc::clone(&asgn.src)));
        let dst_node = *nodes
            .entry(dst_key)
            .or_insert_with(|| graph.add_node(Rc::clone(&asgn.dst)));
        graph.add_edge(src_node, dst_node, ());
        // add edges for guards that read from the port in the guard
        // and write to the dst of the assignment
        for port in &asgn.guard.all_ports() {
            let guard_key = port.borrow().canonical();
            let idx = *nodes
                .entry(guard_key)
                .or_insert_with(|| graph.add_node(Rc::clone(port)));
            graph.add_edge(idx, dst_node, ());
        }
    }

    /// Returns an iterator over all the reads from a port.
    /// Returns an empty iterator if this is an Input port.
    pub fn reads_from(&self, port: &ir::Port) -> PortIterator<'_> {
        if let Some(&idx) = self.nodes.get(&port.canonical()) {
            match port.direction {
                ir::Direction::Input => PortIterator::empty(),
                ir::Direction::Output | ir::Direction::Inout => {
                    PortIterator::new(Box::new(
                        self.graph.edges_directed(idx, Outgoing).map(
                            move |edge| {
                                let node_idx = self
                                    .graph
                                    .edge_endpoints(edge.id())
                                    .unwrap()
                                    .1;
                                Rc::clone(&self.graph[node_idx])
                            },
                        ),
                    ))
                }
            }
        } else {
            PortIterator::empty()
        }
    }

    /// Returns an iterator over all the writes to this port.
    /// Returns an empty iterator if this is an Output port.
    pub fn writes_to(&self, port: &ir::Port) -> PortIterator<'_> {
        if let Some(&idx) = self.nodes.get(&port.canonical()) {
            match port.direction {
                ir::Direction::Input | ir::Direction::Inout => {
                    return PortIterator::new(Box::new(
                        self.graph.edges_directed(idx, Incoming).map(
                            move |edge| {
                                let node_idx = self
                                    .graph
                                    .edge_endpoints(edge.id())
                                    .unwrap()
                                    .0;
                                Rc::clone(&self.graph[node_idx])
                            },
                        ),
                    ))
                }
                ir::Direction::Output => (),
            }
        }
        PortIterator::empty()
    }

    /// Add each edge in `edges` to the graph.
    pub fn add_edges(self, edges: &[(RRC<ir::Port>, RRC<ir::Port>)]) -> Self {
        let Self { graph, nodes } = self;
        let mut graph = graph;
        for (a_ref, b_ref) in edges {
            let a = a_ref.borrow();
            let b = b_ref.borrow();
            if let (Some(a_idx), Some(b_idx)) =
                (nodes.get(&a.canonical()), nodes.get(&b.canonical()))
            {
                graph.add_edge(*a_idx, *b_idx, ());
            }
        }

        Self { nodes, graph }
    }

    /// Return a topological sort of this graph.
    pub fn toposort(&self) -> PortIterator<'_> {
        PortIterator::new(Box::new(
            algo::toposort(&self.graph, None)
                .unwrap()
                .into_iter()
                .map(move |node_idx| Rc::clone(&self.graph[node_idx])),
        ))
    }

    /// Return a Vec of paths from `start` to `finish`, each path a Vec of ports.
    pub fn paths(
        &self,
        start: &ir::Port,
        finish: &ir::Port,
    ) -> Vec<Vec<RRC<ir::Port>>> {
        let start_idx = self.nodes.get(&start.canonical()).unwrap();
        let finish_idx = self.nodes.get(&finish.canonical()).unwrap();

        let paths: Vec<Vec<RRC<ir::Port>>> = algo::all_simple_paths(
            &self.graph,
            *start_idx,
            *finish_idx,
            0,
            None,
        )
        .map(|v: Vec<_>| {
            v.into_iter()
                .map(|i| Rc::clone(&self.graph[NodeIndex::new(i.index())]))
                .collect()
        })
        .collect();
        paths
    }

    /// Restricts the analysis graph to only include edges
    /// that are specified by the `filter`.
    ///
    /// `filter` is passed references to the `src` and `dst` of each
    /// edge. When `filter(src, dst)` is `true`, then the edge between
    /// `src` and `dst` is kept. Otherwise, it is removed.
    pub fn edge_induced_subgraph<F>(self, mut filter: F) -> Self
    where
        F: FnMut(&ir::Port, &ir::Port) -> bool,
    {
        let Self { graph, nodes } = self;
        let graph = graph.filter_map(
            |_, node| Some(Rc::clone(node)),
            |idx, _| {
                let (src_idx, dst_idx) = graph.edge_endpoints(idx).unwrap();
                if filter(&graph[src_idx].borrow(), &graph[dst_idx].borrow()) {
                    Some(())
                } else {
                    None
                }
            },
        );
        Self { nodes, graph }
    }

    /// Returns all the [`Port`](calyx_ir::Port) associated with this instance.
    pub fn ports(&self) -> Vec<RRC<ir::Port>> {
        self.graph
            .raw_nodes()
            .iter()
            .map(|node| Rc::clone(&node.weight))
            .collect()
    }

    /// Remove all vertices that have no undirected neighbors from the analysis graph.
    pub fn remove_isolated_vertices(self) -> Self {
        // Create a node -> neighbor count mapping, that's insensitive to `NodeIndex`s.
        // `retain_nodes`, called a few lines down, invalidates `NodeIndex`s.
        let mut num_neighbors: HashMap<(Id, Id), usize> = HashMap::new();

        let Self { graph, nodes } = self;
        for n_idx in graph.node_indices() {
            let node = graph[n_idx].borrow();
            num_neighbors.insert(
                (node.get_parent_name(), node.name),
                graph.neighbors_undirected(n_idx).count(),
            );
        }
        let mut graph_copy = graph.clone();
        let mut nodes_copy = nodes;

        graph_copy.retain_nodes(|_g, n_idx| {
            let node = graph[n_idx].borrow();
            return *num_neighbors
                .get(&(node.get_parent_name(), node.name))
                .unwrap()
                > 0;
        });

        // retain_nodes breaks existing `NodeIndex`s, so repopulate nodes.
        for node in graph_copy.raw_nodes() {
            let port = node.weight.borrow();
            let n_idx = graph_copy
                .node_indices()
                .find(|idx| *graph_copy[*idx].borrow() == *port)
                .unwrap();
            nodes_copy.insert(port.canonical(), n_idx);
        }

        Self {
            graph: graph_copy,
            nodes: nodes_copy,
        }
    }

    /// Checks if there are cycles in the analysis graph.
    pub fn has_cycles(&self) -> bool {
        algo::is_cyclic_directed(&self.graph)
    }
}

impl ToString for GraphAnalysis {
    fn to_string(&self) -> String {
        let mut out = String::new();
        for idx in self.graph.node_indices() {
            let src_port = self.graph[idx].borrow();
            let src =
                format!("{}.{}", src_port.get_parent_name(), src_port.name);
            writeln!(
                &mut out,
                "{} -> [{}]",
                src,
                self.graph
                    .neighbors_directed(idx, petgraph::Direction::Outgoing)
                    .map(|idx| {
                        let port = self.graph[idx].borrow();
                        format!("{}.{}", port.get_parent_name(), port.name)
                    })
                    .collect::<Vec<String>>()
                    .join(", ")
            )
            .expect("Failed to write to ScheduleConflicts string");
        }
        out
    }
}
