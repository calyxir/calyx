use crate::errors::FutilResult;
use crate::ir::{self, CellType};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// Enable post-order traversal of components.
/// If a component `B` creates a cell `A` then `A` is guaranteed to be visited
/// before `B`.
pub struct PostOrder {
    order: Vec<NodeIndex>,
    comps: Vec<ir::Component>,
}

impl PostOrder {
    /// Returns a new instance the PostOrder iterator
    pub fn new(comps: Vec<ir::Component>) -> Self {
        // Reverse mapping from index to comps.
        let rev_map: HashMap<ir::Id, u32> = comps
            .iter()
            .enumerate()
            .map(|(idx, c)| (c.name.clone(), idx as u32))
            .collect::<HashMap<_, _>>();

        let mut edges: Vec<(u32, u32)> = Vec::new();
        let mut graph: DiGraph<usize, ()> = DiGraph::new();
        for comp in &*comps {
            for cell in &comp.cells {
                if let CellType::Component { name } = &cell.borrow().prototype {
                    edges.push((rev_map[&name], rev_map[&comp.name]));
                }
            }
        }
        graph.extend_with_edges(edges);

        let order = toposort(&graph, None)
            .expect("There is a cycle in definition of component cells");

        PostOrder { order, comps }
    }

    /// Traverses components in post-order and applies `upd`.
    pub fn apply_update<F>(&mut self, mut upd: F) -> FutilResult<()>
    where
        F: FnMut(&mut ir::Component) -> FutilResult<()>,
    {
        self.order
            .clone()
            .iter()
            .map(|idx| upd(&mut self.comps[idx.index()]))
            .collect::<FutilResult<_>>()
    }

    /// Returns the underlying component vector in original order.
    pub fn take(self) -> Vec<ir::Component> {
        self.comps
    }
}
