use calyx_ir::{self as ir, CellType};
use calyx_utils::CalyxResult;
use petgraph::algo;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// The order in which the components are traversed.
#[derive(Default, PartialEq, Eq)]
pub enum Order {
    /// Use an arbitrary order.
    #[default]
    No,
    /// Traverse components in pre-order.
    Pre,
    /// Traverse components in post-order.
    Post,
}

/// Define traversal order of components: pre-order, post-order, or none.
///
/// ## No order
/// Iterates over the components in any order
///
/// ## Post-order
/// If a component `B` creates a cell of type `A` then component `A` is
/// guaranteed to be visited before `B`.
/// This is done by finding a topological order over a graph where `A` will
/// have a directed edge to `B`.
///
/// Instead of constructing a new vector of components in a topological order,
/// the implementation builds an `order` vector which contains indices into the
/// original component vector.
/// This way, we can return the components in the input order once we're done
/// with the post order traversal.
///
/// ## Pre-order
/// Reverse of post-order
///
/// ## Example
/// ```rust
/// let comps: Vec<ir::Component>;
/// // Construct a post order.
/// let post = PostOrder::new(comps, Order::Post);
/// // Apply a mutable update to components.
/// let upd: FnMut(&mut ir::Component) -> CalyxResult<()>;
/// post.apply_update(upd);
/// // Recover the components in original order.
/// let new_comps = post.take();
/// ```
pub struct CompTraversal {
    /// A topological ordering of the components.
    order: Vec<NodeIndex>,
    /// Vector of components in the original ordering.
    comps: Vec<ir::Component>,
}

impl CompTraversal {
    /// Returns a new instance the PostOrder iterator given a Vector of components.
    ///
    /// # Panics
    /// Panics if there is no post-order traversal of the vectors possible.
    pub fn new(comps: Vec<ir::Component>, order: Order) -> Self {
        // If the order is not specified, return the components in the original order.
        if order == Order::No {
            return Self {
                order: (0..comps.len()).map(NodeIndex::new).collect(),
                comps,
            };
        }
        let mut graph: DiGraph<usize, ()> = DiGraph::new();
        // Reverse mapping from index to comps.
        let rev_map: HashMap<ir::Id, NodeIndex> = comps
            .iter()
            .enumerate()
            .map(|(idx, c)| (c.name, graph.add_node(idx)))
            .collect::<HashMap<_, _>>();

        // Construct a graph.
        for comp in &comps {
            for cell in comp.cells.iter() {
                if let CellType::Component { name, .. } =
                    &cell.borrow().prototype
                {
                    graph.add_edge(rev_map[name], rev_map[&comp.name], ());
                }
            }
        }

        // Build a topologically sorted ordering of the graph.
        let mut topo = algo::toposort(&graph, None)
            .expect("There is a cycle in definition of component cells");

        // Reverse the order if a pre-order traversal is requested
        if order == Order::Pre {
            topo.reverse();
        }
        Self { order: topo, comps }
    }

    /// Traverses components in post-order and applies `upd`.
    pub fn apply_update<F>(&mut self, mut upd: F) -> CalyxResult<()>
    where
        F: FnMut(&mut ir::Component, &Vec<ir::Component>) -> CalyxResult<()>,
    {
        for idx in self.order.iter() {
            let mut comp = self.comps.remove(idx.index());
            upd(&mut comp, &self.comps)?;
            self.comps.insert(idx.index(), comp)
        }

        Ok(())
    }

    /// Returns the underlying component vector in original order.
    pub fn take(self) -> Vec<ir::Component> {
        self.comps
    }
}
