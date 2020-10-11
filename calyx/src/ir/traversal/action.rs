//! Actions control the traversal of control programs.
use crate::errors::FutilResult;
use crate::ir::Control;

/// Result of performing a visit.
pub type VisResult = FutilResult<Action>;

/// A visit action.
pub enum Action {
    /// Continue traversal of control program.
    Continue,
    /// Globally abort traversal of control program.
    Stop,
    /// Skips the traversal of this node's children but continues traversing\
    /// the sibling nodes.
    SkipChildren,
    /// Replace the the current ast node with a new node.
    /// If performed using a start_* method, none of the newly created children
    /// will be visited.
    Change(Control),
}

impl Action {
    /// Run the traversal specified by `next` if this traversal succeeds.
    /// If the result of this traversal is not `Action::Continue`, do not
    /// run `next()`.
    pub(super) fn and_then<F>(self, mut next: F) -> VisResult
    where
        F: FnMut() -> VisResult,
    {
        match self {
            Action::Continue => next(),
            Action::Change(_) | Action::Stop | Action::SkipChildren => Ok(self),
        }
    }

    /// Applies the Change action if `self` is a Change action.
    /// Otherwise passes the action through unchanged
    pub(super) fn apply_change(self, con: &mut Control) -> VisResult {
        match self {
            Action::Change(c) => {
                *con = c;
                Ok(Action::Continue)
            }
            x => Ok(x),
        }
    }

    /// Changes a Action::SkipChildren to Action::Continue.
    /// Should be called to indicate the boundary of traversing the children
    /// of a node.
    pub(super) fn pop(self) -> Self {
        match self {
            Action::SkipChildren => Action::Continue,
            x => x,
        }
    }
}
