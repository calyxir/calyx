use calyx::ir::{self, CellType};
use petgraph::algo;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

pub struct CompTraversal<'a> {
    order: Vec<NodeIndex>,
    comps: &'a [ir::Component],
}

impl<'a> CompTraversal<'a> {
    /// This is a pruned clone of the func in [CompTraversal](calyx::ir::traversal::CompTraversal)
    /// it really shouldn't exist
    pub fn new(comps: &'a [ir::Component]) -> Self {
        let mut graph: DiGraph<usize, ()> = DiGraph::new();
        // Reverse mapping from index to comps.
        let rev_map: HashMap<ir::Id, NodeIndex> = comps
            .iter()
            .enumerate()
            .map(|(idx, c)| (c.name, graph.add_node(idx)))
            .collect::<HashMap<_, _>>();

        // Construct a graph.
        for comp in comps {
            for cell in comp.cells.iter() {
                if let CellType::Component { name, .. } =
                    &cell.borrow().prototype
                {
                    graph.add_edge(rev_map[name], rev_map[&comp.name], ());
                }
            }
        }

        // Build a topologically sorted ordering of the graph.
        let topo = algo::toposort(&graph, None)
            .expect("There is a cycle in definition of component cells");

        Self { order: topo, comps }
    }

    pub fn iter(&self) -> impl Iterator<Item = &ir::Component> {
        self.order.iter().map(|idx| &self.comps[idx.index()])
    }
}
