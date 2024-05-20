use std::num::NonZeroU32;

use ahash::{HashMap, HashMapExt};

use super::super::context::Context;
use crate::flatten::{
    flat_ir::prelude::{ControlIdx, ControlMap, ControlNode, GlobalCellIdx},
    structures::index_trait::{impl_index_nonzero, IndexRef},
};

use itertools::{FoldWhile, Itertools};

/// Simple struct containing both the component instance and the active leaf
/// node in the component. This is used to represent an active execution of some
/// portion of the control tree
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ControlPoint {
    pub comp: GlobalCellIdx,
    pub control_node_idx: ControlIdx,
}

impl ControlPoint {
    pub fn new(comp: GlobalCellIdx, control_leaf: ControlIdx) -> Self {
        Self {
            comp,
            control_node_idx: control_leaf,
        }
    }

    /// Constructs a new [ControlPoint] from an existing one by copying over the
    /// component identifier but changing the leaf node
    pub fn new_retain_comp(&self, target: ControlIdx) -> Self {
        Self {
            comp: self.comp,
            control_node_idx: target,
        }
    }

    pub fn get_next(node: &Self, ctx: &Context) -> Option<Self> {
        let path = SearchPath::find_path_from_root(node.control_node_idx, ctx);
        let next = path.next_node(&ctx.primary.control);
        next.map(|x| node.new_retain_comp(x))
    }

    /// Attempts to get the next node for the given control point, if found
    /// it replaces the given node. Returns true if the node was found and
    /// replaced, returns false otherwise
    pub fn mutate_into_next(&mut self, ctx: &Context) -> bool {
        if let Some(next) = Self::get_next(self, ctx) {
            *self = next;
            true
        } else {
            false
        }
    }
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

    fn _is_false_branch(&self) -> bool {
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

    pub fn _source_node(&self) -> Option<&SearchNode> {
        self.path.first()
    }

    pub fn len(&self) -> usize {
        self.path.len()
    }

    pub fn _is_empty(&self) -> bool {
        self.path.is_empty()
    }

    /// Assuming the current node (i.e. the end of this path) has finished
    /// executing, this ascends the path to the parent node and then proceeds to
    /// it's next child, if no such child exists, it ascends again and repeats
    /// the process. If no next node is found, it returns None, indicating that
    /// there is nothing new to evaluate on the path.
    pub fn next_node(&self, control_map: &ControlMap) -> Option<ControlIdx> {
        // Case A: Path is empty? Or has exactly 1 node, so there is no next
        if self.len() < 2 {
            None
        }
        // Case B: We have an actual search to do
        else {
            // minus 2 gets us the second to last node index
            for search_head in (0..=self.len() - 2).rev() {
                let SearchNode { node, search_index } = &self.path[search_head];
                match &control_map[*node] {
                    ControlNode::Seq(s) => {
                        let current_child = search_index.expect(
                            "search index should be present in active seq",
                        );
                        // We still have children to iterate through in this composition
                        if current_child.index() < (s.stms().len() - 1) {
                            let next_child =
                                s.stms()[current_child.index() + 1];
                            return Some(next_child);
                        }
                        // we finished this seq node and need to ascend further
                    }
                    ControlNode::Par(_) => {
                        // the challenge here is that we currently don't know if
                        // the par is done executing. probably this means we
                        // should return None and wait until the entire par is
                        // done? or return a third value indicating that the
                        // par's child count should be decremented. The latter
                        // seems more promising but I need to think on it more

                        return Some(*node);
                    }
                    ControlNode::If(_) => {
                        // there is nothing to do when ascending to an if as it
                        // is already done once the body is done
                        continue;
                    }
                    ControlNode::While(_) => {
                        // we need to re-check the conditional, so this is our
                        // next node
                        return Some(*node);
                    }

                    // none of these three should be possible as a non-leaf node
                    // which is what we are currently searching through on the
                    // path, so this is definitely an error
                    ControlNode::Invoke(_)
                    | ControlNode::Empty(_)
                    | ControlNode::Enable(_) => {
                        unreachable!("SearchPath is malformed. This is an error and should be reported")
                    }
                }
            }

            None
        }
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
                ControlNode::If(i) => {
                    if let Some(idx) = &mut node.search_index {
                        if idx.is_true_branch() {
                            *idx = SearchIndex::new(SearchIndex::FALSE_BRANCH);
                            current_path.path.push(SearchNode {
                                node: i.fbranch(),
                                search_index: None,
                            })
                        } else {
                            current_path.path.pop();
                        }
                    } else {
                        node.search_index =
                            Some(SearchIndex::new(SearchIndex::TRUE_BRANCH));
                        current_path.path.push(SearchNode {
                            node: i.tbranch(),
                            search_index: None,
                        })
                    }
                }
                ControlNode::While(w) => {
                    if node.search_index.is_some() {
                        current_path.path.pop();
                    } else {
                        node.search_index = Some(SearchIndex::new(0));
                        current_path.path.push(SearchNode {
                            node: w.body(),
                            search_index: None,
                        })
                    }
                }
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
        let root_cell = GlobalCellIdx::new(0);

        let mut vec = Vec::with_capacity(CONTROL_POINT_PREALLOCATE);

        if let Some(current) = ctx.primary[root].control {
            vec.push(ControlPoint {
                comp: root_cell,
                control_node_idx: current,
            })
        } else {
            todo!(
                "Flat interpreter does not support control-less components yet"
            )
        }

        Self {
            vec,
            par_map: HashMap::new(),
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, ControlPoint> {
        self.vec.iter()
    }

    pub fn is_done(&self) -> bool {
        self.vec.is_empty()
    }

    pub fn _iter_mut(&mut self) -> impl Iterator<Item = &mut ControlPoint> {
        self.vec.iter_mut()
    }

    pub fn vec_mut(&mut self) -> &mut Vec<ControlPoint> {
        &mut self.vec
    }

    pub fn par_map_mut(&mut self) -> &mut HashMap<ControlPoint, ChildCount> {
        &mut self.par_map
    }

    pub fn _par_map(&self) -> &HashMap<ControlPoint, ChildCount> {
        &self.par_map
    }
}

impl<'a> IntoIterator for &'a ProgramCounter {
    type Item = &'a ControlPoint;

    type IntoIter = std::slice::Iter<'a, ControlPoint>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
