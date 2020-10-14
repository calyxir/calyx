use crate::ir::{Assignment, Component, Direction, Id, Port, RRC};
use petgraph::{
    algo,
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
    Direction::{Incoming, Outgoing},
};
use std::{collections::HashMap, rc::Rc};

type Node = RRC<Port>;
type Edge = ();

pub type CellGraph = DiGraph<Node, Edge>;

#[derive(Clone)]
pub struct Analysis {
    nodes: HashMap<(Id, Id), NodeIndex>,
    graph: CellGraph,
}

impl Analysis {
    pub fn from(component: &Component) -> Self {
        let mut graph = CellGraph::new();
        let mut nodes = HashMap::new();

        let mut insert_asgn = |asgn: &Assignment| {
            let src_key = asgn.src.borrow().key();
            let dst_key = asgn.dst.borrow().key();
            nodes
                .entry(src_key.clone())
                .or_insert(graph.add_node(Rc::clone(&asgn.src)));
            nodes
                .entry(dst_key.clone())
                .or_insert(graph.add_node(Rc::clone(&asgn.dst)));
            let src_node = nodes[&src_key];
            let dst_node = nodes[&dst_key];
            graph.add_edge(src_node, dst_node, ());

            // add edges for ports
            if let Some(guard) = &asgn.guard {
                for port in guard.all_ports() {
                    let guard_key = port.borrow().key();
                    nodes
                        .entry(guard_key.clone())
                        .or_insert(graph.add_node(Rc::clone(&port)));
                    graph.add_edge(nodes[&guard_key], dst_node, ());
                }
            };
        };

        for asgn in &component.continuous_assignments {
            insert_asgn(asgn);
        }

        for group in &component.groups {
            for asgn in &group.borrow().assignments {
                insert_asgn(asgn);
            }
        }

        Analysis { nodes, graph }
    }

    pub fn reads_from(
        &self,
        port: &Port,
    ) -> impl Iterator<Item = RRC<Port>> + '_ {
        let idx = self.nodes[&port.key()];
        match port.direction {
            Direction::Input => unimplemented!(),
            Direction::Output | Direction::Inout => {
                self.graph.edges_directed(idx, Outgoing).map(move |edge| {
                    let node_idx =
                        self.graph.edge_endpoints(edge.id()).unwrap().1;
                    Rc::clone(&self.graph[node_idx])
                })
            }
        }
    }

    pub fn writes_to(
        &self,
        port: &Port,
    ) -> impl Iterator<Item = RRC<Port>> + '_ {
        let idx = self.nodes[&port.key()];
        match port.direction {
            Direction::Input | Direction::Inout => {
                self.graph.edges_directed(idx, Incoming).map(move |edge| {
                    let node_idx =
                        self.graph.edge_endpoints(edge.id()).unwrap().0;
                    Rc::clone(&self.graph[node_idx])
                })
            }
            Direction::Output => unimplemented!(),
        }
    }

    pub fn edge_induced_subgraph<F>(self, mut f: F) -> Self
    where
        F: FnMut(&Port, &Port) -> bool,
    {
        let Self { graph, nodes } = self;
        let graph = graph.filter_map(
            |_idx, node| Some(Rc::clone(node)),
            |idx, edge| {
                let (src_idx, dst_idx) = graph.edge_endpoints(idx).unwrap();
                if f(&graph[src_idx].borrow(), &graph[dst_idx].borrow()) {
                    Some(*edge)
                } else {
                    None
                }
            },
        );
        Self { graph, nodes }
    }

    pub fn has_cycles(&self) -> bool {
        algo::is_cyclic_directed(&self.graph)
    }

    // pub fn visualize(&self) -> String {
    //     let config = &[Config::EdgeNoLabel];
    //     format!(
    //         "{}",
    //         Dot::with_attr_getters(
    //             &self.graph,
    //             config,
    //             &|_g, _edgeref| { "".to_string() },
    //             &|_g, (_idx, cell)| {
    //                 "".to_string()
    //                 // match cell.borrow().prototype {
    //                 //     // NodeData::Hole(..) => "shape=diamond".to_string(),
    //                 //     // NodeData::Cell(..) => "shape=box".to_string(),
    //                 // }
    //             }
    //         )
    //     )
    // }
}
