use ahash::{HashMap, HashMapExt};
use calyx_ir::{self as cir, RRC};

use crate::{
    flatten::{
        flat_ir::{
            cell_prototype::{CellPrototype, LiteralOrPrimitive},
            component::{AuxillaryComponentInfo, ComponentCore},
            flatten_trait::{flatten_tree, FlattenTree, SingleHandle},
            prelude::{
                Assignment, AssignmentIdx, CellRef, CombGroup, CombGroupIdx,
                ComponentIdx, GroupIdx, GuardIdx, PortRef,
            },
            wires::{core::Group, guards::Guard},
        },
        structures::{
            context::{Context, InterpretationContext, SecondaryContext},
            index_trait::IndexRange,
        },
    },
    utils::AsRaw,
};

use super::{structures::*, utils::CompTraversal};

type PortMapper = HashMap<*const cir::Port, PortRef>;
type CellMapper = HashMap<*const cir::Cell, CellRef>;
type ComponentMapper = HashMap<cir::Id, ComponentIdx>;

/// An ephemeral structure used during the translation of a component.
pub struct GroupMapper {
    comb_groups: HashMap<*const cir::CombGroup, CombGroupIdx>,
    groups: HashMap<*const cir::Group, GroupIdx>,
}

pub fn translate(orig_ctx: &cir::Context) -> Context {
    let mut ctx = Context::new();

    let mut component_id_map = ComponentMapper::new();

    // TODO (griffin)
    // the current component traversal is not well equipped for immutable
    // iteration over the components in a post-order so this is a hack instead

    for comp in CompTraversal::new(&orig_ctx.components).iter() {
        translate_component(comp, &mut ctx, &mut component_id_map);
    }

    ctx.entry_point = *component_id_map
        .get(&orig_ctx.entrypoint().name)
        .expect("Unable to find entrypoint");

    ctx
}

#[must_use]
fn translate_group(
    group: &cir::Group,
    ctx: &mut Context,
    map: &PortMapper,
) -> Group {
    let id = ctx.secondary.string_table.insert(group.name());
    let base = ctx.primary.assignments.peek_next_idx();

    for assign in group.assignments.iter() {
        let assign_new = translate_assignment(assign, &mut ctx.primary, map);
        ctx.primary.assignments.push(assign_new);
    }

    let range: IndexRange<AssignmentIdx> =
        IndexRange::new(base, ctx.primary.assignments.peek_next_idx());

    Group::new(
        id,
        range,
        *map[&group.get("go").as_raw()].unwrap_local(),
        *map[&group.get("done").as_raw()].unwrap_local(),
    )
}

#[must_use]
fn translate_comb_group(
    comb_group: &cir::CombGroup,
    ctx: &mut Context,
    map: &PortMapper,
) -> CombGroup {
    let identifier = ctx.secondary.string_table.insert(comb_group.name());
    let base = ctx.primary.assignments.peek_next_idx();

    for assign in comb_group.assignments.iter() {
        let assign_new = translate_assignment(assign, &mut ctx.primary, map);
        ctx.primary.assignments.push(assign_new);
    }

    let range: IndexRange<AssignmentIdx> =
        IndexRange::new(base, ctx.primary.assignments.peek_next_idx());

    CombGroup::new(identifier, range)
}

#[must_use]
fn translate_assignment(
    assign: &cir::Assignment<cir::Nothing>,
    interp_ctx: &mut InterpretationContext,
    map: &PortMapper,
) -> Assignment {
    Assignment {
        dst: map[&assign.dst.as_raw()],
        src: map[&assign.src.as_raw()],
        guard: translate_guard(&assign.guard, interp_ctx, map),
    }
}
#[must_use]
fn translate_guard(
    guard: &cir::Guard<cir::Nothing>,
    interp_ctx: &mut InterpretationContext,
    map: &PortMapper,
) -> GuardIdx {
    flatten_tree(guard, None, &mut interp_ctx.guards, map)
}

fn translate_component(
    comp: &cir::Component,
    ctx: &mut Context,
    component_id_map: &mut ComponentMapper,
) -> ComponentIdx {
    let mut auxillary_component_info = AuxillaryComponentInfo::new_with_name(
        ctx.secondary.string_table.insert(comp.name),
    );

    let layout = compute_local_layout(
        comp,
        ctx,
        &mut auxillary_component_info,
        component_id_map,
    );

    // Translate the groups
    let mut group_map = HashMap::with_capacity(comp.groups.len());

    let group_base = ctx.primary.groups.peek_next_idx();

    for group in comp.groups.iter() {
        let group_brw = group.borrow();
        let group_idx = translate_group(&group_brw, ctx, &layout.port_map);
        let k = ctx.primary.groups.push(group_idx);
        group_map.insert(group.as_raw(), k);
    }
    auxillary_component_info
        .set_group_range(group_base, ctx.primary.groups.peek_next_idx());

    let comb_group_base = ctx.primary.comb_groups.peek_next_idx();
    // Translate comb groups
    let mut comb_group_map = HashMap::with_capacity(comp.comb_groups.len());

    for comb_grp in comp.comb_groups.iter() {
        let comb_grp_brw = comb_grp.borrow();
        let comb_grp_idx =
            translate_comb_group(&comb_grp_brw, ctx, &layout.port_map);
        let k = ctx.primary.comb_groups.push(comb_grp_idx);
        comb_group_map.insert(comb_grp.as_raw(), k);
    }
    auxillary_component_info.set_comb_group_range(
        comb_group_base,
        ctx.primary.comb_groups.peek_next_idx(),
    );

    let group_mapper = GroupMapper {
        comb_groups: comb_group_map,
        groups: group_map,
    };

    // Continuous Assignments
    let cont_assignment_base = ctx.primary.assignments.peek_next_idx();
    for assign in &comp.continuous_assignments {
        let assign_new =
            translate_assignment(assign, &mut ctx.primary, &layout.port_map);
        ctx.primary.assignments.push(assign_new);
    }

    let continuous_assignments = IndexRange::new(
        cont_assignment_base,
        ctx.primary.assignments.peek_next_idx(),
    );

    let ctrl_ref = comp.control.borrow();

    // do some memory slight of hand to pass the owned version rather than a ref
    // to the tuple
    let mut taken_ctx = std::mem::take(ctx);
    // control also must be taken since the flatten needs mutable access to it
    // and this is not possible when it is inside the context
    let mut taken_control = std::mem::take(&mut taken_ctx.primary.control);

    let argument_tuple =
        (group_mapper, layout, taken_ctx, auxillary_component_info);

    let control: Option<ControlIdx> =
        if matches!(*ctrl_ref, cir::Control::Empty(_)) {
            None
        } else {
            let ctrl_node = flatten_tree(
                &*ctrl_ref,
                None,
                &mut taken_control,
                &argument_tuple,
            );
            Some(ctrl_node)
        };

    // unwrap all the stuff packed into the argument tuple
    let (_, _layout, mut taken_ctx, auxillary_component_info) = argument_tuple;

    // put stuff back
    taken_ctx.primary.control = taken_control;
    *ctx = taken_ctx;

    let comp_core = ComponentCore {
        control,
        continuous_assignments,
        is_comb: comp.is_comb,
    };

    let ctrl_ref = ctx.primary.components.push(comp_core);
    ctx.secondary
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
            let idx_definition =
                secondary_ctx.push_local_port(id, port.borrow().width as usize);
            let local_offset = aux.port_offset_map.insert(idx_definition);
            local_offset.into()
        }
    }
}

fn insert_cell(
    secondary_ctx: &mut SecondaryContext,
    aux: &mut AuxillaryComponentInfo,
    cell: &RRC<cir::Cell>,
    layout: &mut Layout,
    comp_id: ComponentIdx,
    comp_id_map: &ComponentMapper,
) {
    let cell_ref = cell.borrow();
    let id = secondary_ctx.string_table.insert(cell_ref.name());

    if !cell_ref.is_reference() {
        let base = aux.port_offset_map.peek_next_index();
        for port in cell_ref.ports() {
            layout.port_map.insert(
                port.as_raw(),
                insert_port(secondary_ctx, aux, port, ContainmentType::Local),
            );
        }
        let range =
            IndexRange::new(base, aux.port_offset_map.peek_next_index());
        let cell_def = secondary_ctx.push_local_cell(
            id,
            range,
            comp_id,
            create_cell_prototype(cell, comp_id_map),
        );
        let cell_offset = aux.cell_offset_map.insert(cell_def);
        layout.cell_map.insert(cell.as_raw(), cell_offset.into());
    }
    // CASE 2 - Reference Cell
    else {
        let base = aux.ref_port_offset_map.peek_next_index();
        for port in cell_ref.ports() {
            layout.port_map.insert(
                port.as_raw(),
                insert_port(secondary_ctx, aux, port, ContainmentType::Ref),
            );
        }

        let range =
            IndexRange::new(base, aux.ref_port_offset_map.peek_next_index());
        let ref_cell_def = secondary_ctx.push_ref_cell(
            id,
            range,
            comp_id,
            create_cell_prototype(cell, comp_id_map),
        );
        let cell_offset = aux.ref_cell_offset_map.insert(ref_cell_def);
        layout.cell_map.insert(cell.as_raw(), cell_offset.into());
    }
}

#[derive(Debug, Default)]
pub struct Layout {
    port_map: PortMapper,
    cell_map: CellMapper,
}

fn compute_local_layout(
    comp: &cir::Component,
    ctx: &mut Context,
    aux: &mut AuxillaryComponentInfo,
    component_id_map: &ComponentMapper,
) -> Layout {
    let comp_id = ctx.primary.components.peek_next_idx();

    let port_def_base = ctx.secondary.local_port_defs.peek_next_idx();
    let ref_port_def_base = ctx.secondary.ref_port_defs.peek_next_idx();
    let cell_def_base = ctx.secondary.local_cell_defs.peek_next_idx();
    let ref_cell_def_base = ctx.secondary.ref_cell_defs.peek_next_idx();

    let mut layout = Layout::default();

    // need this to set the appropriate signature range on the component
    let sig_base = aux.port_offset_map.peek_next_index();

    // first, handle the signature ports
    for port in comp.signature.borrow().ports() {
        let local_offset =
            insert_port(&mut ctx.secondary, aux, port, ContainmentType::Local);
        layout.port_map.insert(port.as_raw(), local_offset);
    }

    // update the aux info with the signature layout
    aux.signature =
        IndexRange::new(sig_base, aux.port_offset_map.peek_next_index());

    // second the group holes
    for group in &comp.groups {
        let group = group.borrow();
        for port in &group.holes {
            let local_offset = insert_port(
                &mut ctx.secondary,
                aux,
                port,
                ContainmentType::Local,
            );
            layout.port_map.insert(port.as_raw(), local_offset);
        }
    }

    let mut sub_component_queue = vec![];

    // third, the primitive cells
    for cell in comp.cells.iter() {
        // this is silly
        // CASE 1 & 2 - local/ref cells
        if is_primitive(&cell.borrow()) {
            insert_cell(
                &mut ctx.secondary,
                aux,
                cell,
                &mut layout,
                comp_id,
                component_id_map,
            )
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
        insert_cell(
            &mut ctx.secondary,
            aux,
            cell,
            &mut layout,
            comp_id,
            component_id_map,
        );

        // Advance the offsets to appropriately layout the next comp-cell
        let cell_ref = cell.borrow();
        if let cir::CellType::Component { name } = &cell_ref.prototype {
            let aux_info = &ctx.secondary.comp_aux_info[component_id_map[name]];
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
        ctx.secondary.local_port_defs.peek_next_idx(),
    );
    aux.set_ref_port_range(
        ref_port_def_base,
        ctx.secondary.ref_port_defs.peek_next_idx(),
    );
    aux.set_cell_range(
        cell_def_base,
        ctx.secondary.local_cell_defs.peek_next_idx(),
    );
    aux.set_ref_cell_range(
        ref_cell_def_base,
        ctx.secondary.ref_cell_defs.peek_next_idx(),
    );

    layout
}

fn create_cell_prototype(
    cell: &RRC<cir::Cell>,
    comp_id_map: &ComponentMapper,
) -> CellPrototype {
    let borrow = cell.borrow();
    match &borrow.prototype {
        prim @ cir::CellType::Primitive { .. } => {
            CellPrototype::construct_primitive(prim)
        }
        cir::CellType::Component { name } => {
            CellPrototype::Component(comp_id_map[name])
        }

        cir::CellType::Constant { val, width } => CellPrototype::Constant {
            value: *val,
            width: (*width).try_into().unwrap(),
            c_type: LiteralOrPrimitive::Literal,
        },
        cir::CellType::ThisComponent => unreachable!(
            "the flattening should not have this cell type, this is an error"
        ),
    }
}

fn is_primitive(cell_ref: &std::cell::Ref<cir::Cell>) -> bool {
    cell_ref.is_primitive::<&str>(None)
        || matches!(&cell_ref.prototype, cir::CellType::Constant { .. })
}

impl FlattenTree for cir::Guard<cir::Nothing> {
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
            cir::Guard::Info(_) => panic!("Guard::Info(_) not handled yet"),
        }
    }
}

impl FlattenTree for cir::Control {
    type Output = ControlNode;

    type IdxType = ControlIdx;

    type AuxillaryData = (GroupMapper, Layout, Context, AuxillaryComponentInfo);

    fn process_element<'data>(
        &'data self,
        mut handle: SingleHandle<'_, 'data, Self, Self::IdxType, Self::Output>,
        aux: &Self::AuxillaryData,
    ) -> Self::Output {
        let (group_map, layout, ctx, comp_info) = aux;
        match self {
            cir::Control::Seq(s) => ControlNode::Seq(Seq::new(
                s.stmts.iter().map(|s| handle.enqueue(s)),
            )),
            cir::Control::Par(p) => ControlNode::Par(Par::new(
                p.stmts.iter().map(|s| handle.enqueue(s)),
            )),
            cir::Control::If(i) => ControlNode::If(If::new(
                layout.port_map[&i.port.as_raw()],
                i.cond.as_ref().map(|c| group_map.comb_groups[&c.as_raw()]),
                handle.enqueue(&i.tbranch),
                handle.enqueue(&i.fbranch),
            )),
            cir::Control::While(w) => ControlNode::While(While::new(
                layout.port_map[&w.port.as_raw()],
                w.cond.as_ref().map(|c| group_map.comb_groups[&c.as_raw()]),
                handle.enqueue(&w.body),
            )),
            cir::Control::Invoke(inv) => {
                let invoked_cell = layout.cell_map[&inv.comp.as_raw()];

                let invoked_comp = match invoked_cell {
                    CellRef::Local(local_off) => {
                        let def_idx = comp_info.cell_offset_map[local_off];

                        &ctx.secondary[def_idx].prototype
                    }
                    CellRef::Ref(ref_off) => {
                        let def_idx = comp_info.ref_cell_offset_map[ref_off];

                        &ctx.secondary[def_idx].prototype
                    }
                };

                let resolve_id = |id: &cir::Id| {
                    *ctx.secondary.string_table.lookup_id(id).unwrap()
                };

                let resolve_invoked_cell_port = |id: &cir::Id| -> PortRef {
                    let id = resolve_id(id);

                    match invoked_cell {
                        CellRef::Local(l) => {
                            let def_idx = comp_info.cell_offset_map[l];
                            let cell_def = &ctx.secondary[def_idx];

                            cell_def
                                .ports
                                .into_iter()
                                .find(|&candidate_offset| {
                                    let candidate_def = comp_info
                                        .port_offset_map[candidate_offset];
                                    ctx.secondary[candidate_def].name == id
                                })
                                .unwrap()
                                .into()
                        }
                        CellRef::Ref(r) => {
                            let def_idx = comp_info.ref_cell_offset_map[r];
                            let cell_def = &ctx.secondary[def_idx];

                            cell_def
                                .ports
                                .into_iter()
                                .find(|&candidate_offset| {
                                    let candidate_def = comp_info
                                        .ref_port_offset_map[candidate_offset];
                                    ctx.secondary[candidate_def] == id
                                })
                                .unwrap()
                                .into()
                        }
                    }
                };

                let ref_cells = inv.ref_cells.iter().map(|(ref_cell_id, realizing_cell)| {
                        let invoked_comp = invoked_comp.as_component().expect("cannot invoke a non-component with ref cells");
                        let target = &ctx.secondary[*invoked_comp].ref_cell_offset_map.iter().find(|(_idx, &def_idx)| {
                            let def = &ctx.secondary[def_idx];
                            def.name == resolve_id(ref_cell_id)
                        }).map(|(t, _)| t).expect("Unable to find the given ref cell in the invoked component");
                        (*target, layout.cell_map[&realizing_cell.as_raw()])
                    });

                let inputs = inv.inputs.iter().map(|(id, port)| {
                    (
                        resolve_invoked_cell_port(id),
                        layout.port_map[&port.as_raw()],
                    )
                });

                let outputs = inv.outputs.iter().map(|(id, port)| {
                    (
                        resolve_invoked_cell_port(id),
                        layout.port_map[&port.as_raw()],
                    )
                });

                ControlNode::Invoke(Invoke::new(
                    invoked_cell,
                    inv.comb_group
                        .as_ref()
                        .map(|x| group_map.comb_groups[&x.as_raw()]),
                    ref_cells,
                    inputs,
                    outputs,
                ))
            }
            cir::Control::Enable(e) => ControlNode::Enable(Enable::new(
                group_map.groups[&e.group.as_raw()],
            )),
            cir::Control::Empty(_) => ControlNode::Empty(Empty),
            cir::Control::Static(_) => {
                todo!("The interpreter does not support static control yet")
            }
            cir::Control::Repeat(_) => {
                todo!("The interpreter does not support repeat yet")
            }
        }
    }
}
