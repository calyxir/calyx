use ahash::{HashMap, HashMapExt};
use calyx::ir::{self as cir};
use cir::RRC;

use crate::{
    flatten::{
        flat_ir::{
            component::{AuxillaryComponentInfo, ComponentCore},
            prelude::{
                Assignment, AssignmentIdx, CombGroup, CombGroupIdx,
                ComponentRef, GroupIdx, GuardIdx, PortRef,
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
type ComponentMapper = HashMap<cir::Id, ComponentRef>;

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
    let mut component_id_map = ComponentMapper::new();

    // TODO (griffin)
    // the current component traversal is not well equipped for immutable
    // iteration over the components in a post-order so this is a hack instead

    for comp in CompTraversal::new(&orig_ctx.components).iter() {
        translate_component(
            comp,
            &mut primary_ctx,
            &mut secondary_ctx,
            &mut component_id_map,
        );
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
        let assign_new = translate_assignment(assign, interp_ctx, map);
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
        let assign_new = translate_assignment(assign, interp_ctx, map);
        interp_ctx.assignments.push(assign_new);
    }

    let range: IndexRange<AssignmentIdx> =
        IndexRange::new(base, interp_ctx.assignments.peek_next_idx());

    CombGroup::new(identifier, range)
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
    secondary_ctx: &mut SecondaryContext,
    component_id_map: &mut ComponentMapper,
) -> ComponentRef {
    let mut auxillary_component_info = AuxillaryComponentInfo::new_with_name(
        secondary_ctx.string_table.insert(comp.name),
    );

    let port_map = compute_local_layout(
        comp,
        interp_ctx,
        secondary_ctx,
        &mut auxillary_component_info,
        component_id_map,
    );

    // Translate the groups
    let mut group_map = HashMap::with_capacity(comp.groups.len());

    for group in comp.groups.iter() {
        let group_brw = group.borrow();
        let group_idx =
            translate_group(&group_brw, interp_ctx, secondary_ctx, &port_map);
        let k = interp_ctx.groups.push(group_idx);
        group_map.insert(group.as_raw(), k);
    }

    // Translate comb groups
    let mut comb_group_map = HashMap::with_capacity(comp.comb_groups.len());

    for comb_grp in comp.comb_groups.iter() {
        let comb_grp_brw = comb_grp.borrow();
        let comb_grp_idx = translate_comb_group(
            &comb_grp_brw,
            interp_ctx,
            secondary_ctx,
            &port_map,
        );
        let k = interp_ctx.comb_groups.push(comb_grp_idx);
        comb_group_map.insert(comb_grp.as_raw(), k);
    }

    let group_mapper = GroupMapper {
        comb_groups: comb_group_map,
        groups: group_map,
    };

    // Continuous Assignments
    let cont_assignment_base = interp_ctx.assignments.peek_next_idx();
    for assign in &comp.continuous_assignments {
        translate_assignment(assign, interp_ctx, &port_map);
    }

    let continuous_assignments = IndexRange::new(
        cont_assignment_base,
        interp_ctx.assignments.peek_next_idx(),
    );

    let ctrl_ref = comp.control.borrow();

    let control: Option<ControlIdx> =
        if matches!(*ctrl_ref, cir::Control::Empty(_)) {
            None
        } else {
            let ctrl_node = flatten_tree(
                &*ctrl_ref,
                None,
                &mut interp_ctx.control,
                &(group_mapper, port_map),
            );
            Some(ctrl_node)
        };

    let comp_core = ComponentCore {
        control,
        continuous_assignments,
        is_comb: comp.is_comb,
    };

    let ctrl_ref = interp_ctx.components.push(comp_core);
    secondary_ctx
        .comp_aux_info
        .insert(ctrl_ref, auxillary_component_info);

    component_id_map.insert(comp.name, ctrl_ref);
    ctrl_ref
}

fn insert_port(
    secondary_ctx: &mut SecondaryContext,
    aux: &mut AuxillaryComponentInfo,
    port: &RRC<cir::Port>,
    port_type: ContainmentType,
) -> PortRef {
    let id = secondary_ctx.string_table.insert(port.borrow().name);
    match port_type {
        ContainmentType::Ref => {
            let idx_definition = secondary_ctx.push_ref_port(id);
            let local_offset = aux.ref_port_offset_map.insert(idx_definition);
            local_offset.into()
        }
        ContainmentType::Local => {
            let idx_definition = secondary_ctx.push_local_port(id);
            let local_offset = aux.port_offset_map.insert(idx_definition);
            local_offset.into()
        }
    }
}

fn insert_cell(
    secondary_ctx: &mut SecondaryContext,
    aux: &mut AuxillaryComponentInfo,
    cell: &RRC<cir::Cell>,
    port_map: &mut PortMapper,
    comp_id: ComponentRef,
) {
    let cell_ref = cell.borrow();
    let id = secondary_ctx.string_table.insert(cell_ref.name());

    if !cell_ref.is_reference() {
        let base = aux.port_offset_map.peek_next_index();
        for port in cell_ref.ports() {
            port_map.insert(
                port.as_raw(),
                insert_port(secondary_ctx, aux, port, ContainmentType::Local),
            );
        }
        let range =
            IndexRange::new(base, aux.port_offset_map.peek_next_index());
        let cell_def = secondary_ctx.push_local_cell(id, range, comp_id);
        aux.cell_offset_map.insert(cell_def);
    }
    // CASE 2 - Reference Cell
    else {
        let base = aux.ref_port_offset_map.peek_next_index();
        for port in cell_ref.ports() {
            port_map.insert(
                port.as_raw(),
                insert_port(secondary_ctx, aux, port, ContainmentType::Ref),
            );
        }
        let range =
            IndexRange::new(base, aux.ref_port_offset_map.peek_next_index());
        let ref_cell_def = secondary_ctx.push_ref_cell(id, range, comp_id);
        aux.ref_cell_offset_map.insert(ref_cell_def);
    }
}

fn compute_local_layout(
    comp: &cir::Component,
    ctx: &mut InterpretationContext,
    secondary_ctx: &mut SecondaryContext,
    aux: &mut AuxillaryComponentInfo,
    component_id_map: &ComponentMapper,
) -> PortMapper {
    let comp_id = ctx.components.peek_next_idx();

    let port_def_base = secondary_ctx.local_port_defs.peek_next_idx();
    let ref_port_def_base = secondary_ctx.ref_port_defs.peek_next_idx();
    let cell_def_base = secondary_ctx.local_cell_defs.peek_next_idx();
    let ref_cell_def_base = secondary_ctx.ref_cell_defs.peek_next_idx();

    let mut port_map = PortMapper::new();

    // need this to set the appropriate signature range on the component
    let sig_base = aux.port_offset_map.peek_next_index();

    // first, handle the signature ports
    for port in comp.signature.borrow().ports() {
        let local_offset =
            insert_port(secondary_ctx, aux, port, ContainmentType::Local);
        port_map.insert(port.as_raw(), local_offset);
    }

    // update the aux info with the signature layout
    aux.signature =
        IndexRange::new(sig_base, aux.port_offset_map.peek_next_index());

    // second the group holes
    for group in &comp.groups {
        let group = group.borrow();
        for port in &group.holes {
            let local_offset =
                insert_port(secondary_ctx, aux, port, ContainmentType::Local);
            port_map.insert(port.as_raw(), local_offset);
        }
    }

    let mut sub_component_queue = vec![];

    // third, the primitive cells
    for cell in comp.cells.iter() {
        // this is silly
        // CASE 1 & 2 - local/ref cells
        if is_primitive(&cell.borrow()) {
            insert_cell(secondary_ctx, aux, cell, &mut port_map, comp_id)
        }
        // CASE 3 - Subcomponent
        else {
            // put in the queue to handle after
            sub_component_queue.push(cell);
        }
    }

    // fourth, the sub-components
    for cell in sub_component_queue {
        // insert the cells and ports
        insert_cell(secondary_ctx, aux, cell, &mut port_map, comp_id);

        // Advance the offsets to appropriately layout the next comp-cell
        let cell_ref = cell.borrow();
        if let cir::CellType::Component { name } = &cell_ref.prototype {
            let aux_info = &secondary_ctx.comp_aux_info[component_id_map[name]];
            let skips = if cell_ref.is_reference() {
                aux_info.skip_sizes_for_ref()
            } else {
                aux_info.skip_sizes_for_local()
            };
            aux.skip_offsets(skips);
        } else {
            unreachable!("Component cell isn't a component?. This shouldn't be possible please report this.")
        }
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

    port_map
}

fn is_primitive(cell_ref: &std::cell::Ref<cir::Cell>) -> bool {
    cell_ref.is_primitive::<&str>(None)
        || matches!(&cell_ref.prototype, cir::CellType::Constant { .. })
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
            cir::Control::Invoke(_inv) => ControlNode::Invoke(Invoke {}),
            cir::Control::Enable(e) => ControlNode::Enable(Enable::new(
                group_map.groups[&e.group.as_raw()],
            )),
            cir::Control::Empty(_) => ControlNode::Empty(Empty),
        }
    }
}
