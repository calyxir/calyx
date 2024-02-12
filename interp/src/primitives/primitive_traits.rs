use crate::{
    errors::InterpreterResult, interpreter::ComponentInterpreter,
    serialization::Serializable, structures::state_views::StateView,
    utils::PrintCode, values::Value,
};

use calyx_ir as ir;

/// A trait indicating that the thing has a name
pub trait Named {
    fn get_full_name(&self) -> &ir::Id;
}

/// A primitive for the interpreter.
/// Roughly corresponds to the cells defined in the primitives library for the Calyx compiler.
/// Primitives can be either stateful or combinational.
pub trait Primitive: Named {
    /// Returns true if this primitive is combinational
    fn is_comb(&self) -> bool;

    /// Validate inputs to the component.
    fn validate(&self, inputs: &[(ir::Id, &Value)]);

    /// Execute the component.
    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>>;

    /// Does nothing for comb. prims; mutates internal state for stateful
    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>>;

    /// Execute the component.
    fn validate_and_execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        self.validate(inputs);
        self.execute(inputs)
    }

    /// Reset the component.
    fn reset(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>>;

    /// Serialize the state of this primitive, if any.
    fn serialize(&self, _code: Option<PrintCode>) -> Serializable {
        Serializable::Empty
    }

    // more efficient to override this with true in stateful cases
    fn has_serializeable_state(&self) -> bool {
        self.serialize(None).has_state()
    }

    fn get_state(&self) -> Option<StateView<'_>> {
        None
    }

    fn get_comp_interpreter(&self) -> Option<&ComponentInterpreter> {
        None
    }
}
