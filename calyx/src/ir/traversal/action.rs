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
    /// Monadic helper function that sequences actions
    /// that return a VisResult.
    /// If `self` is `Continue` or `Change`, return the result of running `f`.
    /// Pass `Stop` through
    fn and_then<F>(self, mut other: F) -> VisResult
    where
        F: FnMut() -> VisResult,
    {
        match self {
            Action::Continue => other(),
            x => Ok(x),
        }
    }

    /// Applies the Change action if `self` is a Change action.
    /// Otherwise passes the action through unchanged
    fn apply_change(self, con: &mut Control) -> VisResult {
        match self {
            Action::Change(c) => {
                *con = c;
                Ok(Action::Continue)
            }
            x => Ok(x),
        }
    }
}
