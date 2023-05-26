pub(crate) mod flat_ir;
pub mod primitives;
mod structures;
pub(crate) mod utils;

pub fn flat_main(ctx: &calyx_ir::Context) {
    let i_ctx = flat_ir::control::translator::translate(ctx);

    dbg!(i_ctx);
}
