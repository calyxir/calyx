use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use calyx_frontend::{SetAttr, source_info::PositionId};
use calyx_ir::GetAttributes;
use calyx_ir::{self as cir, NumAttr, RRC};
use cider_idx::iter::IndexRange;
use itertools::Itertools;

use crate::{
    as_raw::AsRaw,
    flatten::{
        flat_ir::{
            base::{LocalPortOffset, SignatureRange},
            cell_prototype::{CellPrototype, ConstantType},
            component::{
                AuxiliaryComponentInfo, CombComponentCore, ComponentCore,
                PrimaryComponentInfo,
            },
            flatten_trait::{FlattenTree, SingleHandle, flatten_tree},
            prelude::{
                Assignment, AssignmentIdx, CellRef, CombGroup, CombGroupIdx,
                ComponentIdx, GroupIdx, GuardIdx, PortRef,
            },
            wires::{
                core::Group,
                guards::{Guard, PortComp},
            },
        },
        structures::context::{
            Context, InterpretationContext, SecondaryContext,
        },
    },
};

use super::{structures::*, utils::CompTraversal};

/// A transient version of guards that exists during translation to allow
/// hashing, and consequently Hash-consing the flattened guards.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
enum TranslationGuard {
    True,
    Or(Box<TranslationGuard>, Box<TranslationGuard>),
    And(Box<TranslationGuard>, Box<TranslationGuard>),
    Not(Box<TranslationGuard>),
    Comp(PortComp, PortRef, PortRef),
    Port(PortRef),
}

impl TranslationGuard {
    fn translate(
        guard: &cir::Guard<cir::Nothing>,
        mapper: &PortMapper,
    ) -> Self {
        // TODO griffin: make this not recursive? Probably not a huge issue
        // right now
        match guard {
            calyx_ir::Guard::Or(guard, guard1) => Self::Or(
                Box::new(Self::translate(guard, mapper)),
                Box::new(Self::translate(guard1, mapper)),
            ),
            calyx_ir::Guard::And(guard, guard1) => Self::And(
                Box::new(Self::translate(guard, mapper)),
                Box::new(Self::translate(guard1, mapper)),
            ),
            calyx_ir::Guard::Not(guard) => {
                Self::Not(Box::new(Self::translate(guard, mapper)))
            }
            calyx_ir::Guard::True => Self::True,
            calyx_ir::Guard::CompOp(port_comp, ref_cell, ref_cell1) => {
                Self::Comp(
                    port_comp.into(),
                    mapper[&ref_cell.as_raw()],
                    mapper[&ref_cell1.as_raw()],
                )
            }
            calyx_ir::Guard::Port(ref_cell) => {
                Self::Port(mapper[&ref_cell.as_raw()])
            }
            calyx_ir::Guard::Info(_) => {
                todo!("Guard::Info(_) is not currently supported")
            }
        }
    }
}

type PortMapper = HashMap<*const cir::Port, PortRef>;
type CellMapper = HashMap<*const cir::Cell, CellRef>;
type ComponentMapper = HashMap<cir::Id, ComponentIdx>;
type GuardHashConsMap = HashMap<TranslationGuard, GuardIdx>;

/// An ephemeral structure used during the translation of a component.
pub struct GroupMapper {
    comb_groups: HashMap<*const cir::CombGroup, CombGroupIdx>,
    groups: HashMap<*const cir::Group, GroupIdx>,
}

pub fn translate(orig_ctx: &cir::Context) -> Context {
    let mut ctx = Context::new(orig_ctx.source_info_table.clone());

    let mut component_id_map = ComponentMapper::new();
    let mut hash_cons_map = GuardHashConsMap::new();

    // TODO (griffin)
    // the current component traversal is not well equipped for immutable
    // iteration over the components in a post-order so this is a hack instead

    for comp in CompTraversal::new(&orig_ctx.components).iter() {
        translate_component(
            comp,
            &mut ctx,
            &mut component_id_map,
            &mut hash_cons_map,
        );
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
    hash_cons_map: &mut GuardHashConsMap,
) -> Group {
    let id = ctx.secondary.string_table.insert(group.name());
    let base = ctx.primary.assignments.peek_next_idx();

    for assign in group.assignments.iter() {
        let assign_new =
            translate_assignment(assign, &mut ctx.primary, map, hash_cons_map);
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
    hash_cons_map: &mut GuardHashConsMap,
) -> CombGroup {
    let identifier = ctx.secondary.string_table.insert(comb_group.name());
    let base = ctx.primary.assignments.peek_next_idx();

    for assign in comb_group.assignments.iter() {
        let assign_new =
            translate_assignment(assign, &mut ctx.primary, map, hash_cons_map);
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
    hash_cons_map: &mut GuardHashConsMap,
) -> Assignment {
    Assignment {
        dst: map[&assign.dst.as_raw()],
        src: map[&assign.src.as_raw()],
        guard: translate_guard(&assign.guard, interp_ctx, map, hash_cons_map),
    }
}
#[must_use]
fn translate_guard(
    guard: &cir::Guard<cir::Nothing>,
    interp_ctx: &mut InterpretationContext,
    map: &PortMapper,
    hash_cons_map: &mut GuardHashConsMap,
) -> GuardIdx {
    let guard = TranslationGuard::translate(guard, map);

    if let Some(idx) = hash_cons_map.get(&guard) {
        return *idx;
    }

    let idx =
        flatten_tree(&guard, None, &mut interp_ctx.guards, &(), hash_cons_map);

    // if we didn't exit early, the full guard needs to be added to hash-cons
    // map. The sub-guards will have already been added during flattening
    hash_cons_map.insert(guard, idx);

    // not worth trying to force this search traversal into the flatten trait so
    // I'm just gonna opt for this. It's a onetime cost, so I'm not particularly
    // worried about it
    let mut search_stack = vec![idx];
    let mut read_ports: HashSet<PortRef> = HashSet::new();

    while let Some(idx) = search_stack.pop() {
        match &interp_ctx.guards[idx] {
            Guard::True => {}
            Guard::Or(guard_idx, guard_idx1) => {
                search_stack.push(*guard_idx);
                search_stack.push(*guard_idx1);
            }
            Guard::And(guard_idx, guard_idx1) => {
                search_stack.push(*guard_idx);
                search_stack.push(*guard_idx1);
            }
            Guard::Not(guard_idx) => {
                search_stack.push(*guard_idx);
            }
            Guard::Comp(_port_comp, port_ref, port_ref1) => {
                read_ports.insert(*port_ref);
                read_ports.insert(*port_ref1);
            }
            Guard::Port(port_ref) => {
                read_ports.insert(*port_ref);
            }
        }
    }

    if !read_ports.is_empty() {
        interp_ctx
            .guard_read_map
            .insert_value(idx, read_ports.into_iter().collect());
    }

    idx
}

fn translate_component(
    comp: &cir::Component,
    ctx: &mut Context,
    component_id_map: &mut ComponentMapper,
    hash_cons_map: &mut GuardHashConsMap,
) -> ComponentIdx {
    let mut auxiliary_component_info = AuxiliaryComponentInfo::new_with_name(
        ctx.secondary.string_table.insert(comp.name),
    );

    let layout = compute_local_layout(
        comp,
        ctx,
        &mut auxiliary_component_info,
        component_id_map,
    );

    // Continuous Assignments
    let cont_assignment_base = ctx.primary.assignments.peek_next_idx();
    for assign in &comp.continuous_assignments {
        let assign_new = translate_assignment(
            assign,
            &mut ctx.primary,
            &layout.port_map,
            hash_cons_map,
        );
        ctx.primary.assignments.push(assign_new);
    }

    let continuous_assignments = IndexRange::new(
        cont_assignment_base,
        ctx.primary.assignments.peek_next_idx(),
    );

    // Translate the groups
    let mut group_map = HashMap::with_capacity(comp.groups.len());

    let group_base = ctx.primary.groups.peek_next_idx();

    let mut go_port_group_map: HashMap<LocalPortOffset, GroupIdx> =
        HashMap::new();

    for group in comp.groups.iter() {
        let group_brw = group.borrow();
        let translated_group =
            translate_group(&group_brw, ctx, &layout.port_map, hash_cons_map);
        let group_go = translated_group.go;
        let k = ctx.primary.groups.push(translated_group);
        go_port_group_map.insert(group_go, k);
        group_map.insert(group.as_raw(), k);
    }
    auxiliary_component_info
        .set_group_range(group_base, ctx.primary.groups.peek_next_idx());

    for group in auxiliary_component_info.definitions.groups() {
        for assignment in ctx.primary[group].assignments {
            let dst = ctx.primary[assignment].dst;
            if let Some(local) = dst.as_local() {
                if let Some(other_group) = go_port_group_map.get(local) {
                    ctx.primary
                        .groups
                        .get_mut(group)
                        .unwrap()
                        .structural_enables
                        .push(*other_group);
                }
            }
        }
    }

    let comb_group_base = ctx.primary.comb_groups.peek_next_idx();
    // Translate comb groups
    let mut comb_group_map = HashMap::with_capacity(comp.comb_groups.len());

    for comb_grp in comp.comb_groups.iter() {
        let comb_grp_brw = comb_grp.borrow();
        let comb_grp_idx = translate_comb_group(
            &comb_grp_brw,
            ctx,
            &layout.port_map,
            hash_cons_map,
        );
        let k = ctx.primary.comb_groups.push(comb_grp_idx);
        comb_group_map.insert(comb_grp.as_raw(), k);
    }
    auxiliary_component_info.set_comb_group_range(
        comb_group_base,
        ctx.primary.comb_groups.peek_next_idx(),
    );

    let group_mapper = GroupMapper {
        comb_groups: comb_group_map,
        groups: group_map,
    };

    let ctrl_ref = comp.control.borrow();

    // do some memory slight of hand to pass the owned version rather than a ref
    // to the tuple
    let mut taken_ctx = std::mem::take(ctx);
    // control also must be taken since the flatten needs mutable access to it
    // and this is not possible when it is inside the context
    let mut taken_control = std::mem::take(&mut taken_ctx.primary.control);

    let ctrl_idx_start = taken_control.peek_next_idx();

    let argument_tuple =
        (group_mapper, layout, taken_ctx, auxiliary_component_info);

    let control: Option<ControlIdx> =
        if matches!(*ctrl_ref, cir::Control::Empty(_)) {
            None
        } else {
            let ctrl_node = flatten_tree(
                &*ctrl_ref,
                None,
                &mut taken_control,
                &argument_tuple,
                &mut (),
            );
            Some(ctrl_node)
        };

    let ctrl_idx_end = taken_control.peek_next_idx();

    // unwrap all the stuff packed into the argument tuple
    let (_, layout, mut taken_ctx, mut auxiliary_component_info) =
        argument_tuple;

    // put stuff back
    taken_ctx.primary.control = taken_control;
    *ctx = taken_ctx;

    auxiliary_component_info.set_control_range(ctrl_idx_start, ctrl_idx_end);

    for node in IndexRange::new(ctrl_idx_start, ctrl_idx_end).iter() {
        if let Control::Invoke(i) = &mut ctx.primary.control[node].control {
            let assign_start_index = ctx.primary.assignments.peek_next_idx();

            for (dst, src) in i.signature.iter() {
                ctx.primary.assignments.push(Assignment {
                    dst: *dst,
                    src: *src,
                    guard: ctx.primary.guards.push(Guard::True),
                });
            }

            let assign_end_index = ctx.primary.assignments.peek_next_idx();
            i.assignments =
                IndexRange::new(assign_start_index, assign_end_index);
        }
    }

    let go_ports = comp
        .signature
        .borrow()
        .find_all_with_attr(NumAttr::Go)
        .collect_vec();
    let done_ports = comp
        .signature
        .borrow()
        .find_all_with_attr(NumAttr::Done)
        .collect_vec();

    let comp_info: PrimaryComponentInfo = if comp.is_comb {
        assert!(
            go_ports.is_empty() && done_ports.is_empty(),
            "malformed comb component: {}",
            comp.name
        );

        PrimaryComponentInfo::Comb(CombComponentCore {
            continuous_assignments,
        })
    } else {
        let go_port = &go_ports[0];
        let done_port = &done_ports[0];

        // Will need to rethink this at some point
        if go_ports.len() > 1 || done_ports.len() > 1 {
            todo!(
                "handle multiple go and done ports. On component: {}",
                comp.name
            );
        }

        let comp_core = ComponentCore {
            control,
            continuous_assignments,
            go: *layout.port_map[&go_port.as_raw()].unwrap_local(),
            done: *layout.port_map[&done_port.as_raw()].unwrap_local(),
        };
        comp_core.into()
    };

    let ctrl_ref = ctx.primary.components.push(comp_info);
    ctx.secondary
        .comp_aux_info
        .insert(ctrl_ref, auxiliary_component_info);

    component_id_map.insert(comp.name, ctrl_ref);
    ctrl_ref
}

fn insert_port(
    secondary_ctx: &mut SecondaryContext,
    aux: &mut AuxiliaryComponentInfo,
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
            let borrow = port.borrow();
            let is_data = borrow.has_attribute(calyx_ir::BoolAttr::Data);

            let idx_definition = secondary_ctx.push_local_port(
                id,
                port.borrow().width as usize,
                is_data,
                borrow.direction.clone(),
            );
            let local_offset = aux.port_offset_map.insert(idx_definition);
            local_offset.into()
        }
    }
}

fn insert_cell(
    secondary_ctx: &mut SecondaryContext,
    aux: &mut AuxiliaryComponentInfo,
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
            cell_ref.get_attribute(calyx_ir::BoolAttr::Data).is_some(),
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
            cell_ref.get_attribute(calyx_ir::BoolAttr::Data).is_some(),
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
    aux: &mut AuxiliaryComponentInfo,
    component_id_map: &ComponentMapper,
) -> Layout {
    let comp_id = ctx.primary.components.peek_next_idx();

    let port_def_base = ctx.secondary.local_port_defs.peek_next_idx();
    let ref_port_def_base = ctx.secondary.ref_port_defs.peek_next_idx();
    let cell_def_base = ctx.secondary.local_cell_defs.peek_next_idx();
    let ref_cell_def_base = ctx.secondary.ref_cell_defs.peek_next_idx();

    let mut layout = Layout::default();

    let mut sigs_input = SignatureRange::new();
    let mut sigs_output = SignatureRange::new();

    // first, handle the input signature ports
    for port in comp.signature.borrow().ports().into_iter() {
        let local_offset =
            insert_port(&mut ctx.secondary, aux, port, ContainmentType::Local);
        match &port.borrow().direction {
            cir::Direction::Input => {
                sigs_output.append_item(*local_offset.as_local().unwrap());
            }
            cir::Direction::Output => {
                sigs_input.append_item(*local_offset.as_local().unwrap());
            }
            _ => unreachable!("inout port in component signature"),
        }

        layout.port_map.insert(port.as_raw(), local_offset);
    }

    // update the aux info with the signature layout
    aux.signature_in = sigs_input;
    aux.signature_out = sigs_output;

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
            unreachable!(
                "Component cell isn't a component?. This shouldn't be possible please report this."
            )
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
        cir::CellType::Primitive { .. } => {
            CellPrototype::construct_prototype(&borrow)
        }
        cir::CellType::Component { name } => {
            CellPrototype::Component(comp_id_map[name])
        }

        cir::CellType::Constant { val, width } => CellPrototype::Constant {
            value: *val,
            width: (*width).try_into().unwrap(),
            c_type: ConstantType::Literal,
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

impl FlattenTree for TranslationGuard {
    type Output = Guard;
    type IdxType = GuardIdx;
    type AuxiliaryData = ();
    type MutAuxiliaryData = GuardHashConsMap;

    fn process_element<'data>(
        &'data self,
        mut handle: SingleHandle<'_, 'data, Self, Self::IdxType, Self::Output>,
        _: &Self::AuxiliaryData,
        cons_map: &mut Self::MutAuxiliaryData,
    ) -> Self::Output {
        match self {
            TranslationGuard::Or(a, b) => Guard::Or(
                *cons_map.entry(*a.clone()).or_insert(handle.enqueue(a)),
                *cons_map.entry(*b.clone()).or_insert(handle.enqueue(b)),
            ),
            TranslationGuard::And(a, b) => Guard::And(
                *cons_map.entry(*a.clone()).or_insert(handle.enqueue(a)),
                *cons_map.entry(*b.clone()).or_insert(handle.enqueue(b)),
            ),
            TranslationGuard::Not(n) => Guard::Not(
                *cons_map.entry(*n.clone()).or_insert(handle.enqueue(n)),
            ),
            TranslationGuard::True => Guard::True,
            TranslationGuard::Comp(op, a, b) => Guard::Comp(*op, *a, *b),
            TranslationGuard::Port(p) => Guard::Port(*p),
        }
    }
}

impl FlattenTree for cir::Control {
    type Output = ControlNode;
    type IdxType = ControlIdx;
    type AuxiliaryData = (GroupMapper, Layout, Context, AuxiliaryComponentInfo);
    type MutAuxiliaryData = ();

    fn process_element<'data>(
        &'data self,
        mut handle: SingleHandle<'_, 'data, Self, Self::IdxType, Self::Output>,
        aux: &Self::AuxiliaryData,
        _: &mut Self::MutAuxiliaryData,
    ) -> Self::Output {
        let (group_map, layout, ctx, comp_info) = aux;
        let ctrl = match self {
            cir::Control::FSMEnable(_) => todo!(),
            cir::Control::Seq(s) => Control::Seq(Seq::new(
                s.stmts.iter().map(|s| handle.enqueue(s)),
            )),
            cir::Control::Par(p) => Control::Par(Par::new(
                p.stmts.iter().map(|s| handle.enqueue(s)),
            )),
            cir::Control::If(i) => Control::If(If::new(
                layout.port_map[&i.port.as_raw()],
                i.cond.as_ref().map(|c| group_map.comb_groups[&c.as_raw()]),
                handle.enqueue(&i.tbranch),
                handle.enqueue(&i.fbranch),
            )),
            cir::Control::While(w) => Control::While(While::new(
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
                        let target = &ctx.secondary[*invoked_comp].ref_cell_offset_map.iter().find(|(_idx, def_idx)| {
                            let def = &ctx.secondary[**def_idx];
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

                let go = inv
                    .comp
                    .borrow()
                    .find_all_with_attr(NumAttr::Go)
                    .collect_vec();
                assert!(
                    go.len() == 1,
                    "cannot handle multiple go ports yet or the invoked cell has none"
                );
                let comp_go = layout.port_map[&go[0].as_raw()];
                let done = inv
                    .comp
                    .borrow()
                    .find_all_with_attr(NumAttr::Done)
                    .collect_vec();
                assert!(
                    done.len() == 1,
                    "cannot handle multiple done ports yet or the invoked cell has none"
                );
                let comp_done = layout.port_map[&done[0].as_raw()];

                Control::Invoke(Invoke::new(
                    invoked_cell,
                    inv.comb_group
                        .as_ref()
                        .map(|x| group_map.comb_groups[&x.as_raw()]),
                    ref_cells,
                    inputs,
                    outputs,
                    comp_go,
                    comp_done,
                ))
            }
            cir::Control::Enable(e) => Control::Enable(Enable::new(
                group_map.groups[&e.group.as_raw()],
            )),
            cir::Control::Empty(_) => Control::Empty(Empty),
            cir::Control::Static(_) => {
                todo!("The interpreter does not support static control yet")
            }
            cir::Control::Repeat(repeat) => {
                let body = handle.enqueue(&repeat.body);
                Control::Repeat(Repeat::new(body, repeat.num_repeats))
            }
        };

        ControlNode {
            control: ctrl,
            pos: self
                .get_attributes()
                .get_set(SetAttr::Pos)
                .map(|x| x.iter().map(|p| PositionId::new(*p)).collect()),
        }
    }
}
