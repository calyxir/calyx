pub(crate) mod flat_ir;
pub mod primitives;
mod structures;

pub fn flat_main(ctx: &calyx_ir::Context) {
    let i_ctx = flat_ir::control::translator::translate(ctx);

    for (idx, _comp) in i_ctx.primary.components.iter() {
        println!("Component: {}", i_ctx.resolve_id(i_ctx.secondary[idx].name));
        for x in i_ctx.secondary.comp_aux_info[idx].definitions.groups() {
            println!(
                "Group: {}",
                i_ctx.resolve_id(i_ctx.primary.groups[x].name())
            );
            for assign in i_ctx.primary.groups[x].assignments.iter() {
                println!("\t{}", i_ctx.print_assignment(idx, assign));
            }
        }
        println!()
    }
}
