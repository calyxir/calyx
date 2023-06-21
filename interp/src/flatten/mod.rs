pub(crate) mod flat_ir;
pub mod primitives;
mod structures;
pub(crate) mod text_utils;

use structures::environment::Environment;

pub fn flat_main(ctx: &calyx_ir::Context) {
    let i_ctx = flat_ir::control::translator::translate(ctx);

    i_ctx.printer().print_program();

    let env = Environment::new(&i_ctx);
}
