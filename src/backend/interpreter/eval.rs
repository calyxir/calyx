use crate::backend::interpreter::state::State;
use crate::lang::ast;
use crate::lang::context::Context;
use crate::lang::structure::StructureGraph;
use std::collections::HashMap;

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
            let st_1 = eval_c(&comp.control, &comp.structure, st);
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
///   * `c` - is the context for the file
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
///   * `control` - is the control of the component
///   * `structure` - is the graph of the structure
///   * `st` - is the state of the component
///   * `comp_name` - the name of the type of component to run
///   * `inputs` - is a map of input port names to values for passing
///                inputs to the component during evaluation
/// # Returns
///   Returns the new component state
pub fn eval_c(
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
                let st_1 = eval_c(&head[0], structure, st);
                let seq_1 = ast::Seq {
                    stmts: tail.to_vec(),
                };
                let control_1 = Control::Seq { data: seq_1 };
                return eval_c(&control_1, structure, &st_1);
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
            return eval_s(structure, st, data.comps.clone())
        }
        Control::Empty { data: _ } => return st.clone(),
    }
}

/// Simulates the structure of a component for `enable` statements
/// # Arguments
///   * `structure` - is the graph of the structure
///   * `st` - is the context for the file
///   * `enabled` - is a list of enabled components to simulate
/// # Returns
///   Returns the new component state
pub fn eval_s(
    _structure: &StructureGraph,
    _st: &State,
    _enabled: Vec<ast::Id>,
) -> State {
    unimplemented!("Interpreter is not implemented.");
}
