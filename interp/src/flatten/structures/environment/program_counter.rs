use std::num::NonZeroU32;

use ahash::{HashMap, HashMapExt};

use super::super::context::Context;
use crate::flatten::{
    flat_ir::prelude::{ControlIdx, ControlNode, GlobalCellId},
    structures::index_trait::{impl_index_nonzero, IndexRef},
};

use itertools::{FoldWhile, Itertools};

/// Simple struct containing both the component instance and the active leaf
/// node in the component
#[derive(Debug, Hash, Eq, PartialEq)]
pub struct ControlPoint {
    pub comp: GlobalCellId,
    pub control_leaf: ControlIdx,
}

impl ControlPoint {
    pub fn new(comp: GlobalCellId, control_leaf: ControlIdx) -> Self {
        Self { comp, control_leaf }
    }
}

#[derive(Debug)]
enum NextControlPoint {
    /// no
    None,
    /// This is the node to run next. The nice friendly singular case
    Next(ControlPoint),
    /// We just finished the child of this par block and need to decrement its
    /// count
    FinishedParChild(ControlPoint),
    /// We passed through one or more par nodes to reach this leaf (or leaves)
    StartedParChild(Vec<ControlPoint>, Vec<(ControlPoint, u32)>),
}

/// An index for searching up and down a tree. This is used to index into
/// various  control nodes. For If blocks the true branch is denoted by 0 and
/// the false by 1. The same is true for while blocks. For seq and par blocks,
/// it represents the current index into their statement vector. It is not
/// meaningful for other control types.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SearchIndex(NonZeroU32);
impl_index_nonzero!(SearchIndex);

pub struct SearchNode {
    pub node: ControlIdx,
    pub search_index: Option<SearchIndex>,
}

impl SearchIndex {
    const TRUE_BRANCH: usize = 0;
    const FALSE_BRANCH: usize = 1;

    /// Returns the next index, i.e. the current index incremented by 1
    fn next(&self) -> Self {
        Self::new(self.index() + 1)
    }

    fn is_true_branch(&self) -> bool {
        self.index() == Self::TRUE_BRANCH
    }

    fn is_false_branch(&self) -> bool {
        self.index() == Self::FALSE_BRANCH
    }
}

/// A path from a control node (usually root) to some descendent node/leaf in the control tree
pub struct SearchPath {
    path: Vec<SearchNode>,
}

impl SearchPath {
    fn new() -> Self {
        SearchPath { path: vec![] }
    }

    pub fn source_node(&self) -> Option<&SearchNode> {
        self.path.get(0)
    }

    pub fn len(&self) -> usize {
        self.path.len()
    }

    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }

    pub fn find_path_to_node(
        start: ControlIdx,
        target: ControlIdx,
        context: &Context,
    ) -> Self {
        let mut current_path = Self::new();
        current_path.path.push(SearchNode {
            node: start,
            search_index: None,
        });

        while let Some(node) = current_path.path.last_mut() {
            if node.node == target {
                break;
            }

            match &context.primary.control[node.node] {
                ControlNode::Empty(_)
                | ControlNode::Enable(_)
                | ControlNode::Invoke(_) => {
                    // in this case we reached a terminal node which was not the
                    // target since we did not break in the above case. So we
                    // simply remove the current lowest node and ascend the
                    // stack to continue the DFS.
                    current_path.path.pop();
                }
                ControlNode::Seq(s) => {
                    if let Some(idx) = &mut node.search_index {
                        if idx.index() < (s.stms().len() - 1) {
                            *idx = idx.next();
                        } else {
                            current_path.path.pop();
                            continue;
                        }
                    } else if !s.stms().is_empty() {
                        let new_idx = SearchIndex::new(0);
                        node.search_index = Some(new_idx);
                    } else {
                        current_path.path.pop();
                        continue;
                    }

                    // unwrap is safe since by this point it has been forced to
                    // be a Some variant
                    let new_node = s.stms()[node.search_index.unwrap().index()];
                    current_path.path.push(SearchNode {
                        node: new_node,
                        search_index: None,
                    })
                }
                // TODO Griffin: figure out how to deduplicate these arms
                ControlNode::Par(p) => {
                    if let Some(idx) = &mut node.search_index {
                        if idx.index() < (p.stms().len() - 1) {
                            *idx = idx.next();
                        } else {
                            current_path.path.pop();
                            continue;
                        }
                    } else if !p.stms().is_empty() {
                        let new_idx = SearchIndex::new(0);
                        node.search_index = Some(new_idx);
                    } else {
                        current_path.path.pop();
                        continue;
                    }

                    // unwrap is safe since by this point it has been forced to
                    // be a Some variant
                    let new_node = p.stms()[node.search_index.unwrap().index()];
                    current_path.path.push(SearchNode {
                        node: new_node,
                        search_index: None,
                    })
                }
                ControlNode::If(_) => todo!(),
                ControlNode::While(_) => todo!(),
            }
        }

        current_path
    }

    /// find a path to the target node from the root of it's control tree. This
    /// automatically finds the root node and invokes [find_path_to_node].
    pub fn find_path_from_root(target: ControlIdx, context: &Context) -> Self {
        let root = context
            .primary
            .components
            .iter()
            .fold_while(ControlIdx::new(0), |current_root, (_, comp_info)| {
                if let Some(index) = comp_info.control {
                    if index >= current_root && index < target {
                        FoldWhile::Continue(index)
                    } else {
                        FoldWhile::Done(current_root)
                    }
                } else {
                    FoldWhile::Continue(current_root)
                }
            })
            .into_inner();

        Self::find_path_to_node(root, target, context)
    }
}

/// The number of control points to preallocate for the program counter.
const CONTROL_POINT_PREALLOCATE: usize = 16;

/// The number of children that have yet to finish for a given par arm. I have
/// this a u16 at the moment which is hopefully fine? More than 65,535 parallel
/// children would be a lot.
pub type ChildCount = u16;

/// The program counter for the whole program execution. Wraps over a vector of
/// the active leaf statements for each component instance.
#[derive(Debug, Default)]
pub(crate) struct ProgramCounter {
    vec: Vec<ControlPoint>,
    par_map: HashMap<ControlPoint, ChildCount>,
}

// we need a few things from the program counter

impl ProgramCounter {
    pub fn new(ctx: &Context) -> Self {
        let root = ctx.entry_point;
        // this relies on the fact that we construct the root cell-ledger
        // as the first possible cell in the program. If that changes this will break.
        let root_cell = GlobalCellId::new(0);
        let mut par_map: HashMap<ControlPoint, ChildCount> = HashMap::new();

        let mut vec = Vec::with_capacity(CONTROL_POINT_PREALLOCATE);
        if let Some(current) = ctx.primary[root].control {
            let mut work_queue: Vec<ControlIdx> = Vec::from([current]);
            let mut backtrack_map = HashMap::new();

            while let Some(current) = work_queue.pop() {
                match &ctx.primary[current] {
                    ControlNode::Empty(_) => {
                        vec.push(ControlPoint::new(root_cell, current))
                    }
                    ControlNode::Enable(_) => {
                        vec.push(ControlPoint::new(root_cell, current))
                    }
                    ControlNode::Seq(s) => match s
                        .stms()
                        .iter()
                        .find(|&x| !backtrack_map.contains_key(x))
                    {
                        Some(n) => {
                            backtrack_map.insert(*n, current);
                            work_queue.push(*n);
                        }
                        None => {
                            if let Some(b) = backtrack_map.get(&current) {
                                work_queue.push(*b)
                            }
                        }
                    },
                    ControlNode::Par(p) => {
                        par_map.insert(
                            ControlPoint::new(root_cell, current),
                            p.stms().len().try_into().expect(
                                "number of par arms does not fit into the default size value. Please let us know so that we can adjust the datatype accordingly",
                            ),
                        );
                        for node in p.stms() {
                            work_queue.push(*node);
                        }
                    }
                    ControlNode::If(_) => {
                        vec.push(ControlPoint::new(root_cell, current))
                    }
                    ControlNode::While(_) => {
                        vec.push(ControlPoint::new(root_cell, current))
                    }
                    ControlNode::Invoke(_) => {
                        vec.push(ControlPoint::new(root_cell, current))
                    }
                }
            }
        } else {
            todo!(
                "Flat interpreter does not support control-less components yet"
            )
        }

        Self { vec, par_map }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, ControlPoint> {
        self.vec.iter()
    }

    pub fn is_done(&self) -> bool {
        self.vec.is_empty()
    }
}

impl<'a> IntoIterator for &'a ProgramCounter {
    type Item = &'a ControlPoint;

    type IntoIter = std::slice::Iter<'a, ControlPoint>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
