use std::collections::HashSet;

use crate::{
    debugger::name_tree::ActiveTreeNode,
    environment::InterpreterState,
    errors::InterpreterResult,
    structures::names::GroupQIN,
    structures::state_views::{MutStateView, StateView},
};

/// The core interpreter trait which defines the methods each interpretation
/// construct must support. This trait is implemented by each of the control
/// interpreters and the component interpreter but not the
/// [AssignmentInterpreter](interp::interpreter::group_interpreter::AssignmentInterpreter])
pub trait Interpreter {
    /// Advance the interpreter by a "clock" cycle. This should advance stateful
    /// components and potentially change the state machines for control
    /// structures.
    fn step(&mut self) -> InterpreterResult<()>;

    /// Performs combinational convergence for the underlying interpreter
    /// subtree. This updates the combinational paths of all cells in the active
    /// subtree. This is primarially used for evaluating combinational groups in
    /// conditions.
    fn converge(&mut self) -> InterpreterResult<()>;

    /// Advance the interpreter until [Interpreter::is_done] is true. This comes with a
    /// default implementation which calls step in a while loop, however it is
    /// usually more efficient to override this with run calls to the
    /// appropriate subcomponents.
    fn run(&mut self) -> InterpreterResult<()> {
        while !self.is_done() {
            self.step()?;
        }
        Ok(())
    }

    /// Consumes the interpreter and returns the concluding [InterpreterState].
    ///
    /// # Panics
    /// If [Interpreter::is_done] is false
    fn deconstruct(self) -> InterpreterResult<InterpreterState>;

    /// Return whether the interpreter has finished executing
    fn is_done(&self) -> bool;

    /// Returns an immutable handle to the environment underneath the given
    /// interpreter.
    fn get_env(&self) -> StateView<'_>;

    /// Returns the currently executing non-combinational groups
    fn currently_executing_group(&self) -> HashSet<GroupQIN>;

    /// Returns a mutable handle to the environment underneath the given
    /// interpreter
    fn get_env_mut(&mut self) -> MutStateView<'_>;

    /// Returns the active sub-tree of interpreter nodes. Used in the debugging
    /// flow.
    fn get_active_tree(&self) -> Vec<ActiveTreeNode>;

    /// Utility method which runs the interpreter and deconstructs it all in one
    /// step.
    fn run_and_deconstruct(mut self) -> InterpreterResult<InterpreterState>
    where
        Self: Sized,
    {
        self.run()?;
        self.deconstruct()
    }
}
