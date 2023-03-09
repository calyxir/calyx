use std::{cell::RefCell, rc::Rc};

use ahash::{HashMap, HashMapExt};
use calyx::ir::{self as cir};

use crate::{
    flatten::{
        flat_ir::{
            component::AuxillaryComponentInfo,
            prelude::{
                Assignment, AssignmentIdx, CombGroup, CombGroupIdx, GroupIdx,
                GuardIdx, PortRef,
            },
            wires::{core::Group, guards::Guard},
        },
        structures::{
            context::{InterpretationContext, SecondaryContext},
            index_trait::IndexRange,
        },
        utils::{flatten_tree, FlattenTree, SingleHandle},
    },
    utils::AsRaw,
};

use super::{structures::*, utils::CompTraversal};

type PortMapper = HashMap<*const cir::Port, PortRef>;

/// An ephemeral structure used during the translation of a component.
pub struct GroupMapper {
    comb_groups: HashMap<*const cir::CombGroup, CombGroupIdx>,
    groups: HashMap<*const cir::Group, GroupIdx>,
}

pub fn translate(
    orig_ctx: &cir::Context,
) -> (InterpretationContext, SecondaryContext) {
    let mut primary_ctx = InterpretationContext::new();
    let mut secondary_ctx = SecondaryContext::new();

    // TODO (griffin)
    // the current component traversal is not well equipped for immutable
    // iteration over the components in a post-order so this is a hack instead

    for comp in CompTraversal::new(&orig_ctx.components).iter() {
        translate_component(comp, &mut primary_ctx, &mut secondary_ctx);
    }

    (primary_ctx, secondary_ctx)
}

fn translate_group(
    group: &cir::Group,
    interp_ctx: &mut InterpretationContext,
    secondary_ctx: &mut SecondaryContext,
    map: &PortMapper,
) -> Group {
    let id = secondary_ctx.string_table.insert(group.name());
    let base = interp_ctx.assignments.peek_next_idx();

    for assign in group.assignments.iter() {
        let assign_new =
            translate_assignment(assign, interp_ctx, secondary_ctx, map);
        interp_ctx.assignments.push(assign_new);
    }

    let range: IndexRange<AssignmentIdx> =
        IndexRange::new(base, interp_ctx.assignments.peek_next_idx());

    Group::new(
        id,
        range,
        *map[&group.get("go").as_raw()].unwrap_local(),
        *map[&group.get("done").as_raw()].unwrap_local(),
    )
}

fn translate_comb_group(
    comb_group: &cir::CombGroup,
    interp_ctx: &mut InterpretationContext,
    secondary_ctx: &mut SecondaryContext,
    map: &PortMapper,
) -> CombGroup {
    let identifier = secondary_ctx.string_table.insert(comb_group.name());
    let base = interp_ctx.assignments.peek_next_idx();

    for assign in comb_group.assignments.iter() {
        let assign_new =
            translate_assignment(assign, interp_ctx, secondary_ctx, map);
        interp_ctx.assignments.push(assign_new);
    }

    let range: IndexRange<AssignmentIdx> =
        IndexRange::new(base, interp_ctx.assignments.peek_next_idx());

    CombGroup::new(identifier, range)
}

fn translate_assignment(
    assign: &cir::Assignment,
    interp_ctx: &mut InterpretationContext,
    secondary_ctx: &mut SecondaryContext,
    map: &PortMapper,
) -> Assignment {
    Assignment {
        dst: map[&assign.dst.as_raw()],
        src: map[&assign.src.as_raw()],
        guard: translate_guard(&assign.guard, interp_ctx, secondary_ctx, map),
    }
}

fn translate_guard(
    guard: &cir::Guard,
    interp_ctx: &mut InterpretationContext,
    _secondary_ctx: &mut SecondaryContext,
    map: &PortMapper,
) -> GuardIdx {
    flatten_tree(guard, None, &mut interp_ctx.guards, map)
}

fn translate_component(
    comp: &cir::Component,
    interp_ctx: &mut InterpretationContext,
    secondary_ctx: &mut SecondaryContext,
) {
    let mut aux_info = AuxillaryComponentInfo::new_with_name(
        secondary_ctx.string_table.insert(comp.name),
    );

    let map =
        compute_local_layout(comp, interp_ctx, secondary_ctx, &mut aux_info);

    let mut group_map = HashMap::with_capacity(comp.groups.len());

    for group in comp.groups.iter() {
        let group_brw = group.borrow();
        let group_idx =
            translate_group(&group_brw, interp_ctx, secondary_ctx, &map);
        let k = interp_ctx.groups.push(group_idx);
        group_map.insert(group.as_raw(), k);
    }

    let mut comb_group_map = HashMap::with_capacity(comp.comb_groups.len());

    for comb_grp in comp.comb_groups.iter() {
        let comb_grp_brw = comb_grp.borrow();
        let comb_grp_idx = translate_comb_group(
            &comb_grp_brw,
            interp_ctx,
            secondary_ctx,
            &map,
        );
        let k = interp_ctx.comb_groups.push(comb_grp_idx);
        comb_group_map.insert(comb_grp.as_raw(), k);
    }

    todo!()
}

enum PortType {
    Ref,
    Local,
}

fn insert_port(
    secondary_ctx: &mut SecondaryContext,
    aux: &mut AuxillaryComponentInfo,
    port: &Rc<RefCell<cir::Port>>,
    port_type: PortType,
) -> PortRef {
    let id = secondary_ctx.string_table.insert(port.borrow().name);
    match port_type {
        PortType::Ref => {
            let idx_definition = secondary_ctx.push_ref_port(id);
            let local_offset = aux.ref_port_offset_map.insert(idx_definition);
            local_offset.into()
        }
        PortType::Local => {
            let idx_definition = secondary_ctx.push_local_port(id);
            let local_offset = aux.port_offset_map.insert(idx_definition);
            local_offset.into()
        }
    }
}

fn compute_local_layout(
    comp: &cir::Component,
    ctx: &mut InterpretationContext,
    secondary_ctx: &mut SecondaryContext,
    aux: &mut AuxillaryComponentInfo,
) -> PortMapper {
    let comp_id = ctx.components.peek_next_idx();

    let port_def_base = secondary_ctx.local_port_defs.peek_next_idx();
    let ref_port_def_base = secondary_ctx.ref_port_defs.peek_next_idx();
    let cell_def_base = secondary_ctx.local_cell_defs.peek_next_idx();
    let ref_cell_def_base = secondary_ctx.ref_cell_defs.peek_next_idx();

    let mut portmap = PortMapper::new();

    // need this to set the appropriate signature range on the component
    let sig_base = aux.port_offset_map.peek_next_index();

    // first, handle the signature ports
    for port in comp.signature.borrow().ports() {
        let local_offset =
            insert_port(secondary_ctx, aux, port, PortType::Local);

        portmap.insert(port.as_raw(), local_offset);
    }

    // update the aux info with the signature layout
    aux.signature =
        IndexRange::new(sig_base, aux.port_offset_map.peek_next_index());

    // second the group holes
    for group in &comp.groups {
        let group = group.borrow();
        for port in &group.holes {
            let local_offset =
                insert_port(secondary_ctx, aux, port, PortType::Local);
            portmap.insert(port.as_raw(), local_offset);
        }
    }

    let mut sub_component_queue = vec![];

    // third, the primitive cells
    for cell in comp.cells.iter() {
        let cell_ref = cell.borrow();
        let id = secondary_ctx.string_table.insert(cell_ref.name());

        // this is silly
        if cell_ref.is_primitive::<&str>(None)
            || matches!(&cell_ref.prototype, cir::CellType::Constant { .. })
        {
            // CASE 1 - Normal Cell
            if !cell_ref.is_reference() {
                let base = aux.port_offset_map.peek_next_index();

                for port in cell_ref.ports() {
                    let local_offset =
                        insert_port(secondary_ctx, aux, port, PortType::Local);
                    portmap.insert(port.as_raw(), local_offset);
                }
                let range = IndexRange::new(
                    base,
                    aux.port_offset_map.peek_next_index(),
                );
                let cell_def =
                    secondary_ctx.push_local_cell(id, range, comp_id);
                aux.cell_offset_map.insert(cell_def);
            }
            // CASE 2 - Reference Cell
            else {
                let base = aux.ref_port_offset_map.peek_next_index();
                for port in cell_ref.ports() {
                    let local_offset =
                        insert_port(secondary_ctx, aux, port, PortType::Ref);

                    portmap.insert(port.as_raw(), local_offset);
                }
                let range = IndexRange::new(
                    base,
                    aux.ref_port_offset_map.peek_next_index(),
                );
                let ref_cell_def =
                    secondary_ctx.push_ref_cell(id, range, comp_id);
                aux.ref_cell_offset_map.insert(ref_cell_def);
            }
        }
        // CASE 3 - Subcomponent
        else {
            // put in the queue to handle after
            sub_component_queue.push(cell);
        }
    }

    // fourth, the sub-components
    for _cell in sub_component_queue {
        todo!("non-primitive cells are not yet supported")
    }

    aux.set_port_range(
        port_def_base,
        secondary_ctx.local_port_defs.peek_next_idx(),
    );
    aux.set_ref_port_range(
        ref_port_def_base,
        secondary_ctx.ref_port_defs.peek_next_idx(),
    );
    aux.set_cell_range(
        cell_def_base,
        secondary_ctx.local_cell_defs.peek_next_idx(),
    );
    aux.set_ref_cell_range(
        ref_cell_def_base,
        secondary_ctx.ref_cell_defs.peek_next_idx(),
    );

    portmap
}

impl FlattenTree for cir::Guard {
    type Output = Guard;
    type IdxType = GuardIdx;
    type AuxillaryData = PortMapper;

    fn process_element<'data>(
        &'data self,
        mut handle: SingleHandle<'_, 'data, Self, Self::IdxType, Self::Output>,
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

impl FlattenTree for cir::Control {
    type Output = ControlNode;

    type IdxType = ControlIdx;

    type AuxillaryData = (GroupMapper, PortMapper);

    fn process_element<'data>(
        &'data self,
        mut handle: SingleHandle<'_, 'data, Self, Self::IdxType, Self::Output>,
        aux: &Self::AuxillaryData,
    ) -> Self::Output {
        let (group_map, port_map) = aux;
        match self {
            cir::Control::Seq(s) => ControlNode::Seq(Seq::new(
                s.stmts.iter().map(|s| handle.enqueue(s)),
            )),
            cir::Control::Par(p) => ControlNode::Par(Par::new(
                p.stmts.iter().map(|s| handle.enqueue(s)),
            )),
            cir::Control::If(i) => ControlNode::If(If::new(
                port_map[&i.port.as_raw()],
                i.cond.as_ref().map(|c| group_map.comb_groups[&c.as_raw()]),
                handle.enqueue(&i.tbranch),
                handle.enqueue(&i.fbranch),
            )),
            cir::Control::While(w) => ControlNode::While(While::new(
                port_map[&w.port.as_raw()],
                w.cond.as_ref().map(|c| group_map.comb_groups[&c.as_raw()]),
                handle.enqueue(&w.body),
            )),
            cir::Control::Invoke(_inv) => todo!("Invoke not yet supported"),
            cir::Control::Enable(e) => ControlNode::Enable(Enable::new(
                group_map.groups[&e.group.as_raw()],
            )),
            cir::Control::Empty(_) => ControlNode::Empty(Empty),
        }
    }
}
