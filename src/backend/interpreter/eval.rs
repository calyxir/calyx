use crate::backend::interpreter::state::State;
use crate::backend::traits::{Backend, Emitable};
use crate::lang::ast;
use crate::lang::context::Context;
use crate::lang::structure::StructureGraph;
use std::collections::HashMap;

pub struct InterpreterBackend {}

// impl Backend for InterpreterBackend {
//     fn name() -> &'static str {
//         "interpreter"
//     }

//     fn validate(prog: &Context) -> Result<(), errors::Error> {}
// }

/// Evaluates a component
/// # Arguments
///   * `c` - is the context for the file
///   * `st` - is the default starting state of the component
///   * `comp_name` - the name of the type of component to run
///   * `inputs` - is a map of input port names to values for passing
///                inputs to the component during evaluation
/// # Returns
///   Returns a map of output port names to values
pub fn _eval(
    c: &Context,
    st: &State,
    comp_name: &ast::Id,
    inputs: HashMap<ast::Id, Option<i64>>,
) -> (HashMap<ast::Id, Option<i64>>, State) {
    if c.is_lib(comp_name) {
        // Handle library components in a special case
        return eval_lib(c, st, comp_name, inputs);
    }

    // User-defined components
    match c.get_component(comp_name) {
        Ok(comp) => {
            //  structure graph
            let st_1 = eval_c(&c, &inputs, &comp.control, &comp.structure, st);
        }
        Err(_e) => {
            //XXX(ken) errors
            unimplemented!("Implement errors");
        }
    }

    unimplemented!("Interpreter is not implemented.");
}

/// Evaluates a library component
/// # Arguments
///   * `c` - is the context for the compilation unit
///   * `st` - is the state of the component
///   * `comp_name` - the name of the type of component to run
///   * `inputs` - is a map of input port names to values for passing
///                inputs to the component during evaluation
/// # Returns
///   Returns a map of output port names to values
pub fn eval_lib(
    _c: &Context,
    _st: &State,
    _comp_name: &ast::Id,
    _inputs: HashMap<ast::Id, Option<i64>>,
) -> (HashMap<ast::Id, Option<i64>>, State) {
    unimplemented!("Interpreter is not implemented.");
}

/// Simulates the control of a component
/// # Arguments
///   * `c` - is the context for the compilation unit
///   * `inputs` - is a map of input port names to values for passing
///                inputs to the component during evaluation
///   * `control` - is the control statement to evaluate
///   * `structure` - is the graph of the structure
///   * `st` - is the state of the component
///   * `comp_name` - the name of the type of component to run
/// # Returns
///   Returns the new component state
pub fn eval_c(
    c: &Context,
    inputs: &HashMap<ast::Id, Option<i64>>,
    control: &ast::Control,
    structure: &StructureGraph,
    st: &State,
) -> State {
    use ast::Control;
    match control {
        Control::Seq { data } => {
            if data.stmts.is_empty() {
                return st.clone();
            } else {
                let (head, tail) = data.stmts.split_at(1);
                let st_1 = eval_c(&c, inputs, &head[0], structure, st);
                let seq_1 = ast::Seq {
                    stmts: tail.to_vec(),
                };
                let control_1 = Control::Seq { data: seq_1 };
                return eval_c(&c, inputs, &control_1, structure, &st_1);
            }
        }
        Control::Par { data } => {
            unimplemented!("Parallel");
        }
        Control::If { data } => {
            unimplemented!("If");
        }
        Control::While { data } => {
            unimplemented!("While");
        }
        Control::Print { data } => {
            unimplemented!("Print");
        }
        Control::Enable { data } => {
            // Create a fresh graph for evaluation so we don't impact the original structure
            // It should have no values on the wires
            let mut graph = structure.clone();
            graph.split_seq_prims(); // Split sequential primitives to remove valid cycles
            graph.drive_inputs(inputs); // Initialize with input values
            return eval_s(&c, inputs, &mut graph, st, data.comps.clone());
        }
        Control::Empty { data: _ } => return st.clone(),
    }
}

/// Simulates the structure of a component for `enable` statements
/// # Arguments
///   * `c` - is the context for the compilation unit
///   * `inputs` - is a map of input port names to values for passing
///                inputs to the component during evaluation
///   * `structure` - is the graph of the structure
///   * `st` - is the context for the file
///   * `enabled` - is a list of enabled components to simulate
/// # Returns
///   Returns the new component state
pub fn eval_s(
    c: &Context,
    inputs: &HashMap<ast::Id, Option<i64>>,
    structure: &mut StructureGraph,
    st: &State,
    enabled: Vec<ast::Id>,
) -> State {
    unimplemented!("Interpreter is not implemented.");
}
