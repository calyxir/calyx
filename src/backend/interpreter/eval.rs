use crate::lang::context::Context;
use std::collections::HashMap;
use std::path::Path;

/// Evaluates the structure of a component
/// # Arguments
///   * `c` - is the context for the file
///   * `comp_name` - is a path that contains the Id's of all of the component's
///                   parent components, and lastly its own Id.
///   * `inputs` - is a map of input port names to values for passing
///                inputs to the component during evaluation
/// # Returns
///   Returns a map of output port names to values
pub fn _eval_s(
    _c: &Context,
    _st: &State,
    _comp_name: &Path,
    _inputs: HashMap<ast::Id, Option<i64>>,
) -> (HashMap<ast::Id, Option<i64>>, &State) {
    unimplemented!("Interpreter is not implemented.");
}

pub fn _eval_c(_c: &Context) {
    unimplemented!("Interpreter is not implemented.");
}
