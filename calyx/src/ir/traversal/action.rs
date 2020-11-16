//! Actions control the traversal of control programs.
use crate::errors::FutilResult;
use crate::ir::{self, Control};

/// Contains both a data packet of type `T` and an `Action`.
/// The main purpose of this struct is to give names to
/// the data and action respectively for clearer code.
pub struct ActionTuple<T> {
    /// The data packet of this tuple.
    pub data: T,
    /// The action of this tuple.
    pub action: Action,
}

impl<T> From<(T, Action)> for ActionTuple<T> {
    fn from((data, action): (T, Action)) -> Self {
        ActionTuple { data, action }
    }
}

/// Result of performing a visit.
pub type VisResult<T> = FutilResult<ActionTuple<T>>;

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

impl<T> ActionTuple<T> {
    /// Run the traversal specified by `next` if this traversal succeeds.
    /// If the result of this traversal is not `Action::Continue`, do not
    /// run `next()`.
    pub(super) fn and_then<F>(self, mut next: F) -> VisResult<T>
    where
        F: FnMut(T) -> VisResult<T>,
    {
        match self.action {
            Action::Continue => next(self.data),
            Action::Change(_) | Action::Stop | Action::SkipChildren => Ok(self),
        }
    }

    /// Applies the Change action if `self` is a Change action.
    /// Otherwise passes the action through unchanged
    pub(super) fn apply_change(self, con: &mut Control) -> VisResult<T> {
        match self.action {
            Action::Change(c) => {
                *con = c;
                Ok((self.data, Action::Continue).into())
            }
            action => Ok((self.data, action).into()),
        }
    }

    /// Changes a Action::SkipChildren to Action::Continue.
    /// Should be called to indicate the boundary of traversing the children
    /// of a node.
    pub(super) fn pop(self) -> Self {
        match self.action {
            Action::SkipChildren => (self.data, Action::Continue).into(),
            x => (self.data, x).into(),
        }
    }
}

impl Action {
    /// Construct Action::Continue using a default data value
    pub fn continue_default<T: Default>() -> ActionTuple<T> {
        (T::default(), Action::Continue).into()
    }

    /// Construct Action::Continue using a custom data value
    pub fn continue_with<T>(t: T) -> ActionTuple<T> {
        (t, Action::Continue).into()
    }

    /// Construct Action::Stop using a default data value
    pub fn stop_default<T: Default>() -> ActionTuple<T> {
        (T::default(), Action::Stop).into()
    }

    /// Construct Action::Stop using a custom data value
    pub fn stop_with<T>(t: T) -> ActionTuple<T> {
        (t, Action::Stop).into()
    }

    /// Construct Action::Change using a default data value
    pub fn change_default<T: Default>(ctrl: ir::Control) -> ActionTuple<T> {
        (T::default(), Action::Change(ctrl)).into()
    }

    /// Construct Action::Change using a custom data value
    pub fn change_with<T>(t: T, ctrl: ir::Control) -> ActionTuple<T> {
        (t, Action::Change(ctrl)).into()
    }

    /// Construct Action::SkipChildren using a default data value
    pub fn skipchildren_default<T: Default>() -> ActionTuple<T> {
        (T::default(), Action::SkipChildren).into()
    }

    /// Construct Action::SkipChildren using a custom data value
    pub fn skipchildren_with<T>(t: T) -> ActionTuple<T> {
        (t, Action::SkipChildren).into()
    }
}
