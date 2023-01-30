use calyx::ir as cir;

use crate::flatten::{
    flat_ir::{
        identifier::IdMap,
        prelude::Identifier,
        wires::core::{Group, GroupMap},
    },
    structures::context::InterpretationContext,
};

pub fn translate(orig_ctx: cir::Context) -> InterpretationContext {
    todo!()
}

fn translate_group(
    group: &cir::Group,
    interp_ctx: &mut InterpretationContext,
) -> Group {
    let identifier = interp_ctx.string_table.insert(group.name());

    todo!()
}
