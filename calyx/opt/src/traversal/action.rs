//! Actions control the traversal of control programs.
use calyx_ir::Control;
use calyx_ir::StaticControl;
use calyx_utils::CalyxResult;

/// Result of performing a visit.
pub type VisResult = CalyxResult<Action>;

/// Action performed at the end of visiting a control statement.
pub enum Action {
    /// Continue traversal of control program.
    Continue,
    /// Globally abort traversal of control program.
    Stop,
    /// Skips the traversal of this node's children but continues traversing\
    /// the sibling nodes.
    SkipChildren,
    /// Replace the current ast node with a new node.
    /// If performed using a start_* method, none of the newly created children
    /// will be visited.
    Change(Box<Control>),
    /// Replace the current StaticControl node with a new node
    /// If performed using a start_* method, none of the newly created children
    /// will be visited.
    StaticChange(Box<StaticControl>),
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
            Action::Change(_)
            | Action::Stop
            | Action::SkipChildren
            | Action::StaticChange(_) => Ok(self),
        }
    }

    pub fn change(control: Control) -> Self {
        Action::Change(Box::new(control))
    }

    pub fn static_change(control: StaticControl) -> Self {
        Action::StaticChange(Box::new(control))
    }

    /// Applies the Change action if `self` is a Change action.
    /// Otherwise passes the action through unchanged
    pub(super) fn apply_change(self, con: &mut Control) -> Action {
        match self {
            Action::Change(c) => {
                *con = *c;
                Action::Continue
            }
            action => action,
        }
    }

    /// Applies the StaticChange action if `self is a StaticChange action.
    /// Otherwise passes the action through unchanged
    pub(super) fn apply_static_change(self, con: &mut StaticControl) -> Action {
        match self {
            Action::StaticChange(c) => {
                *con = *c;
                Action::Continue
            }
            action => action,
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
