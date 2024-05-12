pub(crate) mod flat_ir;
pub mod primitives;
pub(crate) mod structures;
pub(crate) mod text_utils;

use crate::{errors::InterpreterResult, serialization::data_dump::DataDump};
use structures::environment::{Environment, Simulator};

pub fn flat_main(ctx: &calyx_ir::Context) -> InterpreterResult<DataDump> {
    let i_ctx = flat_ir::control::translator::translate(ctx);

    let mut sim = Simulator::new(Environment::new(&i_ctx));
    sim.run_program()?;
    Ok(sim.dump_memories())
}
