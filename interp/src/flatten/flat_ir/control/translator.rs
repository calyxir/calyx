use std::collections::{HashMap, VecDeque};

use calyx::ir as cir;

use crate::{
    flatten::{
        flat_ir::{
            identifier::IdMap,
            prelude::{
                Assignment, GuardIdx, Identifier, LocalPortRef, PortRef,
            },
            wires::{
                core::{Group, GroupMap},
                guards::Guard,
            },
        },
        structures::context::InterpretationContext,
        utils::{flatten_tree, FlattenTree, SingleHandle},
    },
    utils::AsRaw,
};

type PortMapper = HashMap<*const cir::Port, PortRef>;

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

fn translate_assignment(
    assign: &cir::Assignment,
    interp_ctx: &mut InterpretationContext,
    map: &PortMapper,
) -> Assignment {
    Assignment {
        dst: map[&assign.dst.as_raw()],
        src: map[&assign.src.as_raw()],
        guard: translate_guard(&assign.guard, interp_ctx, map),
    }
}

fn translate_guard(
    guard: &cir::Guard,
    interp_ctx: &mut InterpretationContext,
    map: &PortMapper,
) -> GuardIdx {
    flatten_tree(guard, None, &mut interp_ctx.guards, map)
}

fn translate_component(
    comp: &cir::Component,
    interp_ctx: &mut InterpretationContext,
) {
    let mut portmap: PortMapper = HashMap::new();
    

    todo!()
}

impl FlattenTree for cir::Guard {
    type Output = Guard;
    type IdxType = GuardIdx;
    type AuxillaryData = PortMapper;

    fn process_element<'data>(
        &'data self,
        mut handle: SingleHandle<'_, 'data, Self, Self::Output, Self::IdxType>,
        aux: &Self::AuxillaryData,
    ) -> Self::Output {
        match self {
            cir::Guard::Or(a, b) => {
                Guard::Or(handle.enqueue(a), handle.enqueue(b))
            }
            cir::Guard::And(a, b) => {
                Guard::And(handle.enqueue(a), handle.enqueue(b))
            }
            cir::Guard::Not(n) => Guard::Not(handle.enqueue(n)),
            cir::Guard::True => Guard::True,
            cir::Guard::CompOp(op, a, b) => Guard::Comp(
                op.clone(),
                *aux.get(&a.as_raw()).unwrap(),
                *aux.get(&b.as_raw()).unwrap(),
            ),
            cir::Guard::Port(p) => Guard::Port(*aux.get(&p.as_raw()).unwrap()),
        }
    }
}
