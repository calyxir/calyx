pub(crate) mod flat_ir;
pub mod primitives;
pub(crate) mod structures;
pub(crate) mod text_utils;

use crate::errors::InterpreterResult;
use structures::environment::{Environment, Simulator};

pub fn flat_main(ctx: &calyx_ir::Context) -> InterpreterResult<()> {
    let i_ctx = flat_ir::control::translator::translate(ctx);

    let env = Environment::new(&i_ctx);
    let mut sim = Simulator::new(env);
    let this = &mut sim;
    this.run_program()
}
