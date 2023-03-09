pub(crate) mod flat_ir;
pub mod primitives;
mod structures;
pub(crate) mod utils;

pub fn flat_main(ctx: &calyx::ir::Context) {
    let (prim, sec) = flat_ir::control::translator::translate(ctx);

    dbg!(prim);
    dbg!(sec);
}
