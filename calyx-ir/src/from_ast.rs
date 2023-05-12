use super::{
    Assignment, Attributes, BackendConf, Builder, Canonical, CellType,
    Component, Context, Control, Direction, GetAttributes, Guard, Id, Invoke,
    LibrarySignatures, Port, PortDef, StaticControl, StaticInvoke,
    RESERVED_NAMES, RRC,
};
use crate::{Nothing, PortComp, StaticTiming};
use calyx_frontend::ast::ComponentDef;
use calyx_frontend::{ast, ast::Atom, Attribute, BoolAttr, NumAttr, Workspace};
use calyx_utils::{CalyxResult, Error, GPosIdx, NameGenerator, WithPos};
use petgraph::algo;
use petgraph::graph::{DiGraph, NodeIndex};
use std::cell::RefCell;

use std::collections::{HashMap, HashSet};
use std::num::NonZeroU64;
use std::rc::Rc;

type InvokePortMap = Vec<(Id, Atom)>;

// given `comps`, a vec of `ast::ComponentDef`, builds a topological ordering Vec<NodeIndex>.
// In other words, if component 1 has index i1 in `comps` and
// component 2 has index i2 in `comps`, if component 1 instantiates component 2, then
// i2 will appear before i1 in the topological ordering returned by
// this function.
// Very similar to the `PostOrder` used by `Visitor` but here we are
// dealing with `ast::ComponentDef`s rather than `ir::Component`s.
fn get_post_order(comps: &Vec<ast::ComponentDef>) -> Vec<NodeIndex> {
    let mut graph: DiGraph<usize, ()> = DiGraph::new();

    let comp_names: HashSet<_> = comps.iter().map(|comp| comp.name).collect();

    // Reverse mapping from index to comps.
    let rev_map: HashMap<Id, NodeIndex> = comps
        .iter()
        .enumerate()
        .map(|(idx, c)| (c.name, graph.add_node(idx)))
        .collect::<HashMap<_, _>>();

    // Construct a graph.
    for comp in comps {
        for cell in comp.cells.iter() {
            let cell_type_name = cell.prototype.name;
            if comp_names.contains(&cell_type_name) {
                graph.add_edge(
                    rev_map[&cell_type_name],
                    rev_map[&comp.name],
                    (),
                );
            }
        }
    }

    // Build a topologically sorted ordering of the graph.
    algo::toposort(&graph, None)
        .expect("There is a cycle in definition of component cells")
}

// helper function `ast_to_ir`. Helps us iterate `ComponentDef` in topological
// order.
// When we remove a component at index `removed_value` from the Vec of `ComponentDef`
// we need to adjust the rest of the `values` that are greater than `removed_value` to
// decrease by 1, since they are now one spot earlier in the Vec of `ComponentDef`s.
fn update_sorted_indices(values: &mut [usize], removed_value: usize) {
    for val in values {
        if *val > removed_value {
            *val -= 1;
        }
    }
}

/// Context to store the signature information for all defined primitives and
/// components.
#[derive(Default)]
struct SigCtx {
    /// Mapping from component names to signatures
    comp_sigs: HashMap<Id, Vec<PortDef<u64>>>,

    /// Mapping from library functions to signatures
    lib: LibrarySignatures,
}

/// Validates a component signature to make sure there are not duplicate ports.
fn check_signature(pds: &[PortDef<u64>]) -> CalyxResult<()> {
    let mut ports: HashSet<&Id> = HashSet::new();
    for PortDef {
        name, direction, ..
    } in pds
    {
        // check for uniqueness
        match &direction {
            Direction::Input => {
                if !ports.contains(&name) {
                    ports.insert(name);
                } else {
                    return Err(Error::already_bound(
                        *name,
                        "port".to_string(),
                    ));
                }
            }
            Direction::Output => {
                if !ports.contains(&name) {
                    ports.insert(name);
                } else {
                    return Err(Error::already_bound(
                        *name,
                        "port".to_string(),
                    ));
                }
            }
            Direction::Inout => {
                panic!("Components shouldn't have inout ports.")
            }
        }
    }
    Ok(())
}

/// Definition of special interface ports.
const INTERFACE_PORTS: [(Attribute, u64, Direction); 4] = [
    (Attribute::Num(NumAttr::Go), 1, Direction::Input),
    (Attribute::Bool(BoolAttr::Clk), 1, Direction::Input),
    (Attribute::Bool(BoolAttr::Reset), 1, Direction::Input),
    (Attribute::Num(NumAttr::Done), 1, Direction::Output),
];

/// Extend the signature with magical ports.
fn extend_signature(sig: &mut Vec<PortDef<u64>>) {
    let port_names: HashSet<_> = sig.iter().map(|pd| pd.name).collect();
    let mut namegen = NameGenerator::with_prev_defined_names(port_names);
    for (attr, width, direction) in INTERFACE_PORTS.iter() {
        // Check if there is already another interface port defined for the
        // component
        if !sig.iter().any(|pd| pd.attributes.has(*attr)) {
            let mut attributes = Attributes::default();
            attributes.insert(*attr, 1);
            let name = Id::from(attr.to_string());
            sig.push(PortDef {
                name: namegen.gen_name(name.to_string()),
                width: *width,
                direction: direction.clone(),
                attributes,
            });
        }
    }
}

/// Construct an IR representation using a parsed AST and command line options.
pub fn ast_to_ir(mut workspace: Workspace) -> CalyxResult<Context> {
    let mut all_names: HashSet<&Id> = HashSet::with_capacity(
        workspace.components.len() + workspace.externs.len(),
    );

    let prim_names = workspace
        .externs
        .iter()
        .flat_map(|(_, prims)| prims.iter().map(|prim| &prim.name));

    let comp_names = workspace.components.iter().map(|comp| &comp.name);

    for bound in prim_names.chain(comp_names) {
        if all_names.contains(bound) {
            return Err(Error::already_bound(
                *bound,
                "component or primitive".to_string(),
            ));
        }
        all_names.insert(bound);
    }

    // Build the signature context
    let mut sig_ctx = SigCtx {
        lib: workspace.externs.into(),
        ..Default::default()
    };

    // Add declarations to context
    for comp in workspace
        .declarations
        .iter_mut()
        .chain(workspace.components.iter_mut())
    {
        let sig = &mut comp.signature;
        check_signature(&*sig)?;
        // extend the signature if the component does not have the @nointerface attribute.
        if !comp.attributes.has(BoolAttr::NoInterface) && !comp.is_comb {
            extend_signature(sig);
        }
        sig_ctx.comp_sigs.insert(comp.name, sig.clone());
    }

    let all_prims = sig_ctx.lib.all_prims();

    // maps primitives/components to optional latencies
    // starts with only primitives. Updates each time we call `build_component` to
    // include the component's (possibly inferred) latency (or None if the component
    // is not static)
    let mut comp_latency_map: HashMap<Id, Option<u64>> = all_prims
        .iter()
        .flat_map(|(_, prims)| {
            prims.iter().map(|(id, prim)| (*id, prim.latency))
        })
        .collect();

    // must iterate thru `ComponentDef`s in topological order to support `static_invoke` latency inference
    let mut topo_order: Vec<_> = get_post_order(&workspace.components)
        .into_iter()
        .map(|x| x.index())
        .collect();
    // reversing so we can use topo_order.pop()
    topo_order.reverse();
    // `ast_comps` should be a topological ordering of `ComponentDefs`
    let mut ast_comps: Vec<ComponentDef> = Vec::new();
    while !topo_order.is_empty() {
        // getting the earliest topological component (pop works since we called reverse()).
        let removed_idx = topo_order.pop().unwrap();
        ast_comps.push(workspace.components.remove(removed_idx));
        // must update topo_order since we removed a component at `removed_idx` from workspace.components
        update_sorted_indices(&mut topo_order, removed_idx);
    }

    // building components from `ast::ComponentDef`s to `ir::Component`
    let comps: Vec<Component> = ast_comps
        .into_iter()
        .map(|comp| build_component(comp, &mut comp_latency_map, &mut sig_ctx))
        .collect::<Result<_, _>>()?;

    // Find the entrypoint for the program.
    let entrypoint = comps
        .iter()
        .find(|c| c.attributes.has(BoolAttr::TopLevel))
        .or_else(|| comps.iter().find(|c| c.name == "main"))
        .map(|c| c.name)
        .ok_or_else(|| Error::misc("No entry point for the program. Program needs to be either mark a component with the \"toplevel\" attribute or define a component named `main`".to_string()))?;

    Ok(Context {
        components: comps,
        lib: sig_ctx.lib,
        bc: BackendConf::default(),
        entrypoint,
        extra_opts: vec![],
        metadata: workspace.metadata,
    })
}

fn validate_component(
    comp: &ast::ComponentDef,
    sig_ctx: &SigCtx,
) -> CalyxResult<()> {
    let mut cells: HashMap<Id, GPosIdx> = HashMap::new();
    let mut groups: HashMap<Id, GPosIdx> = HashMap::new();

    for cell in &comp.cells {
        let attrs = &cell.attributes;
        if let Some(pos) = cells.get(&cell.name) {
            let prev =
                pos.into_option().map(|s| s.format("Previous definition"));
            return Err(Error::already_bound(cell.name, "cell".to_string())
                .with_pos(attrs)
                .with_post_msg(prev));
        }
        cells.insert(cell.name, cell.attributes.copy_span());

        let proto_name = cell.prototype.name;

        if sig_ctx.lib.find_primitive(proto_name).is_none()
            && !sig_ctx.comp_sigs.contains_key(&proto_name)
        {
            return Err(Error::undefined(
                proto_name,
                "primitive or component".to_string(),
            )
            .with_pos(attrs));
        }
    }

    for group in &comp.groups {
        let name = &group.name;
        let attrs = &group.attributes;
        if let Some(pos) = groups.get(name) {
            let prev =
                pos.into_option().map(|s| s.format("Previous definition"));
            return Err(Error::already_bound(*name, "group".to_string())
                .with_pos(attrs)
                .with_post_msg(prev));
        }
        if let Some(pos) = cells.get(name) {
            let prev =
                pos.into_option().map(|s| s.format("Previous definition"));
            return Err(Error::already_bound(*name, "cell".to_string())
                .with_pos(attrs)
                .with_post_msg(prev));
        }
        if let Some(pos) = cells.get(name) {
            let prev =
                pos.into_option().map(|s| s.format("Previous definition"));
            return Err(Error::already_bound(*name, "cell".to_string())
                .with_pos(attrs)
                .with_post_msg(prev));
        }
        groups.insert(*name, group.attributes.copy_span());
    }

    Ok(())
}

/// Build an `ir::component::Component` using an `frontend::ast::ComponentDef`.
fn build_component(
    comp: ast::ComponentDef,
    comp_latency_map: &mut HashMap<Id, Option<u64>>,
    sig_ctx: &mut SigCtx,
) -> CalyxResult<Component> {
    // Validate the component before building it.
    validate_component(&comp, sig_ctx)?;

    let mut ir_component = Component::new(
        comp.name,
        comp.signature,
        comp.is_comb,
        // we may change latency from None to Some(inferred latency)
        // after we iterate thru the control of the Component
        comp.latency.map(|x| x.into()),
    );

    let mut builder =
        Builder::new(&mut ir_component, &sig_ctx.lib).not_generated();

    // For each ast::Cell, add a Cell that contains all the
    // required information.
    comp.cells
        .into_iter()
        .for_each(|cell| add_cell(cell, sig_ctx, &mut builder));

    comp.groups
        .into_iter()
        .try_for_each(|g| add_group(g, &mut builder))?;

    comp.static_groups
        .into_iter()
        .try_for_each(|g| add_static_group(g, &mut builder))?;

    let continuous_assignments =
        build_assignments(comp.continuous_assignments, &mut builder)?;
    builder.component.continuous_assignments = continuous_assignments;

    // Build the Control ast using ast::Control.
    let control = Rc::new(RefCell::new(build_control(
        comp.control,
        comp_latency_map,
        &mut builder,
    )?));

    // need to update the latency (if component is static) to be the inferred latency
    // of the control
    if comp.is_static {
        match &*control.borrow() {
            Control::Static(sc) => {
                assert_latencies_eq(comp.latency, sc.get_latency());
                // add inferred latency to comp_latency_map
                comp_latency_map.insert(comp.name, Some(sc.get_latency()));
                // must update the latency of the ir_component to be the inferred latency
                // (we needed to wait until we iterated thru the Control to get this information)
                builder.component.latency = Some(sc.get_latency());
            }
            _ => {
                return Err(Error::malformed_structure(format!(
                    "static component {} has non-static control",
                    comp.name
                )))
            }
        }
    } else {
        comp_latency_map.insert(comp.name, None);
    }

    builder.component.control = control;

    ir_component.attributes = comp.attributes;

    // Add reserved names to the component's namegenerator so future conflicts
    // don't occur
    ir_component
        .add_names(RESERVED_NAMES.iter().map(|s| Id::from(*s)).collect());

    Ok(ir_component)
}

///////////////// Cell Construction /////////////////////////

fn add_cell(cell: ast::Cell, sig_ctx: &SigCtx, builder: &mut Builder) {
    let proto_name = cell.prototype.name;

    let res = if sig_ctx.lib.find_primitive(proto_name).is_some() {
        let c = builder.add_primitive(
            cell.name,
            proto_name,
            &cell.prototype.params,
        );
        c.borrow_mut().set_reference(cell.reference);
        c
    } else {
        // Validator ensures that if the protoype is not a primitive, it
        // is a component.
        let name = builder.component.generate_name(cell.name);
        let sig = &sig_ctx.comp_sigs[&proto_name];
        let typ = CellType::Component { name: proto_name };
        let reference = cell.reference;
        // Components do not have any bindings for parameters
        let cell = Builder::cell_from_signature(name, typ, sig.clone());
        cell.borrow_mut().set_reference(reference);
        builder.component.cells.add(Rc::clone(&cell));
        cell
    };

    // Add attributes to the built cell
    res.borrow_mut().attributes = cell.attributes;
}

///////////////// Group Construction /////////////////////////

/// Build an [super::Group] from an [ast::Group] and attach it to the [Component]
/// associated with the [Builder]
fn add_group(group: ast::Group, builder: &mut Builder) -> CalyxResult<()> {
    if group.is_comb {
        let ir_group = builder.add_comb_group(group.name);
        let assigns = build_assignments(group.wires, builder)?;

        ir_group.borrow_mut().attributes = group.attributes;
        ir_group.borrow_mut().assignments = assigns;
    } else {
        let ir_group = builder.add_group(group.name);
        let assigns = build_assignments(group.wires, builder)?;

        ir_group.borrow_mut().attributes = group.attributes;
        ir_group.borrow_mut().assignments = assigns;
    };

    Ok(())
}

/// Build an [super::StaticGroup] from an [ast::StaticGroup] and attach it to the [Component]
/// associated with the [Builder]
fn add_static_group(
    group: ast::StaticGroup,
    builder: &mut Builder,
) -> CalyxResult<()> {
    if group.latency.get() == 0 {
        return Err(Error::malformed_structure(
            "static group with 0 latency".to_string(),
        ));
    }
    let ir_group = builder.add_static_group(group.name, group.latency.get());
    let assigns = build_static_assignments(group.wires, builder)?;

    ir_group.borrow_mut().attributes = group.attributes;
    ir_group.borrow_mut().assignments = assigns;

    Ok(())
}

///////////////// Assignment Construction /////////////////////////

/// Get the pointer to the Port represented by `port`.
fn get_port_ref(port: ast::Port, comp: &Component) -> CalyxResult<RRC<Port>> {
    match port {
        ast::Port::Comp { component, port } => comp
            .find_cell(component)
            .ok_or_else(|| Error::undefined(component, "cell".to_string()))?
            .borrow()
            .find(port)
            .ok_or_else(|| Error::undefined(port, "port".to_string())),
        ast::Port::This { port } => {
            comp.signature.borrow().find(&port).ok_or_else(|| {
                Error::undefined(port, "component port".to_string())
            })
        }
        ast::Port::Hole { group, name: port } => match comp.find_group(group) {
            Some(g) => g
                .borrow()
                .find(port)
                .ok_or_else(|| Error::undefined(port, "hole".to_string())),
            None => comp
                .find_static_group(group)
                .ok_or_else(|| Error::undefined(group, "group".to_string()))?
                .borrow()
                .find(port)
                .ok_or_else(|| Error::undefined(port, "hole".to_string())),
        },
    }
}

/// Get an port using an ast::Atom.
/// If the atom is a number and the context doesn't already contain a cell
/// for this constant, instantiate the constant node and get the "out" port
/// from it.
fn atom_to_port(
    atom: ast::Atom,
    builder: &mut Builder,
) -> CalyxResult<RRC<Port>> {
    match atom {
        ast::Atom::Num(n) => {
            let port = builder.add_constant(n.val, n.width).borrow().get("out");
            Ok(Rc::clone(&port))
        }
        ast::Atom::Port(p) => get_port_ref(p, builder.component),
    }
}

/// Ensures that the given port has the required direction.
fn ensure_direction(pr: RRC<Port>, dir: Direction) -> CalyxResult<RRC<Port>> {
    let port_dir = pr.borrow().direction.clone();
    match (dir, port_dir) {
        (Direction::Input, Direction::Output) => {
            let Canonical(c, p) = pr.borrow().canonical();
            Err(Error::malformed_structure(format!(
                "Port `{}.{}` occurs in write position but is an output port",
                c, p
            )))
        }
        (Direction::Output, Direction::Input) => {
            let Canonical(c, p) = pr.borrow().canonical();
            Err(Error::malformed_structure(format!(
                "Port `{}.{}` occurs in write position but is an output port",
                c, p
            )))
        }
        _ => Ok(pr),
    }
}

/// Build an ir::Assignment from ast::Wire.
/// The Assignment contains pointers to the relevant ports.
fn build_assignment(
    wire: ast::Wire,
    builder: &mut Builder,
) -> CalyxResult<Assignment<Nothing>> {
    let src_port: RRC<Port> = ensure_direction(
        atom_to_port(wire.src.expr, builder)?,
        Direction::Output,
    )?;
    let dst_port: RRC<Port> = ensure_direction(
        get_port_ref(wire.dest, builder.component)?,
        Direction::Input,
    )?;
    if src_port.borrow().width != dst_port.borrow().width {
        let msg = format!(
            "Mismatched port widths. Source has size {} while destination requires {}.",
            src_port.borrow().width,
            dst_port.borrow().width,
        );
        return Err(Error::malformed_structure(msg).with_pos(&wire.attributes));
    }
    let guard = match wire.src.guard {
        Some(g) => build_guard(g, builder)?,
        None => Guard::True,
    };

    let mut assign = builder.build_assignment(dst_port, src_port, guard);
    assign.attributes = wire.attributes;
    Ok(assign)
}

/// Build an ir::StaticAssignment from ast::StaticWire.
/// The Assignment contains pointers to the relevant ports.
fn build_static_assignment(
    wire: ast::StaticWire,
    builder: &mut Builder,
) -> CalyxResult<Assignment<StaticTiming>> {
    let src_port: RRC<Port> = ensure_direction(
        atom_to_port(wire.src.expr, builder)?,
        Direction::Output,
    )?;
    let dst_port: RRC<Port> = ensure_direction(
        get_port_ref(wire.dest, builder.component)?,
        Direction::Input,
    )?;
    if src_port.borrow().width != dst_port.borrow().width {
        let msg = format!(
            "Mismatched port widths. Source has size {} while destination requires {}.",
            src_port.borrow().width,
            dst_port.borrow().width,
        );
        return Err(Error::malformed_structure(msg).with_pos(&wire.attributes));
    }
    let guard = match wire.src.guard {
        Some(g) => build_static_guard(g, builder)?,
        None => Guard::True,
    };

    let mut assign = builder.build_assignment(dst_port, src_port, guard);
    assign.attributes = wire.attributes;
    Ok(assign)
}

fn build_assignments(
    assigns: Vec<ast::Wire>,
    builder: &mut Builder,
) -> CalyxResult<Vec<Assignment<Nothing>>> {
    assigns
        .into_iter()
        .map(|w| {
            let attrs = w.attributes.clone();
            build_assignment(w, builder).map_err(|err| err.with_pos(&attrs))
        })
        .collect::<CalyxResult<Vec<_>>>()
}

fn build_static_assignments(
    assigns: Vec<ast::StaticWire>,
    builder: &mut Builder,
) -> CalyxResult<Vec<Assignment<StaticTiming>>> {
    assigns
        .into_iter()
        .map(|w| {
            let attrs = w.attributes.clone();
            build_static_assignment(w, builder)
                .map_err(|err| err.with_pos(&attrs))
        })
        .collect::<CalyxResult<Vec<_>>>()
}

/// Transform an ast::GuardExpr to an ir::Guard.
fn build_guard(
    guard: ast::GuardExpr,
    bd: &mut Builder,
) -> CalyxResult<Guard<Nothing>> {
    use ast::GuardExpr as GE;

    let into_box_guard = |g: Box<GE>, bd: &mut Builder| -> CalyxResult<_> {
        Ok(Box::new(build_guard(*g, bd)?))
    };

    Ok(match guard {
        GE::Atom(atom) => Guard::port(ensure_direction(
            atom_to_port(atom, bd)?,
            Direction::Output,
        )?),
        GE::Or(l, r) => Guard::or(build_guard(*l, bd)?, build_guard(*r, bd)?),
        GE::And(l, r) => Guard::and(build_guard(*l, bd)?, build_guard(*r, bd)?),
        GE::Not(g) => Guard::Not(into_box_guard(g, bd)?),
        GE::CompOp((op, l, r)) => {
            let nl = ensure_direction(atom_to_port(l, bd)?, Direction::Output)?;
            let nr = ensure_direction(atom_to_port(r, bd)?, Direction::Output)?;
            let nop = match op {
                ast::GuardComp::Eq => PortComp::Eq,
                ast::GuardComp::Neq => PortComp::Neq,
                ast::GuardComp::Gt => PortComp::Gt,
                ast::GuardComp::Lt => PortComp::Lt,
                ast::GuardComp::Geq => PortComp::Geq,
                ast::GuardComp::Leq => PortComp::Leq,
            };
            Guard::CompOp(nop, nl, nr)
        }
    })
}

/// Transform an ast::GuardExpr to an ir::Guard.
fn build_static_guard(
    guard: ast::StaticGuardExpr,
    bd: &mut Builder,
) -> CalyxResult<Guard<StaticTiming>> {
    use ast::StaticGuardExpr as SGE;

    let into_box_guard = |g: Box<SGE>, bd: &mut Builder| -> CalyxResult<_> {
        Ok(Box::new(build_static_guard(*g, bd)?))
    };

    Ok(match guard {
        SGE::StaticInfo(interval) => Guard::Info(StaticTiming::new(interval)),
        SGE::Atom(atom) => Guard::port(ensure_direction(
            atom_to_port(atom, bd)?,
            Direction::Output,
        )?),
        SGE::Or(l, r) => {
            Guard::or(build_static_guard(*l, bd)?, build_static_guard(*r, bd)?)
        }
        SGE::And(l, r) => {
            Guard::and(build_static_guard(*l, bd)?, build_static_guard(*r, bd)?)
        }
        SGE::Not(g) => Guard::Not(into_box_guard(g, bd)?),
        SGE::CompOp((op, l, r)) => {
            let nl = ensure_direction(atom_to_port(l, bd)?, Direction::Output)?;
            let nr = ensure_direction(atom_to_port(r, bd)?, Direction::Output)?;
            let nop = match op {
                ast::GuardComp::Eq => PortComp::Eq,
                ast::GuardComp::Neq => PortComp::Neq,
                ast::GuardComp::Gt => PortComp::Gt,
                ast::GuardComp::Lt => PortComp::Lt,
                ast::GuardComp::Geq => PortComp::Geq,
                ast::GuardComp::Leq => PortComp::Leq,
            };
            Guard::CompOp(nop, nl, nr)
        }
    })
}

///////////////// Control Construction /////////////////////////

fn assert_latencies_eq(
    given_latency: Option<NonZeroU64>,
    inferred_latency: u64,
) {
    if let Some(v) = given_latency {
        assert_eq!(
            v.get(),
            inferred_latency,
            "inferred latency: {inferred_latency}, given latency: {v}"
        )
    };
}

// builds static_seq based on stmts, attributes, and latency
fn build_static_seq(
    stmts: Vec<ast::Control>,
    attributes: Attributes,
    latency: Option<NonZeroU64>,
    builder: &mut Builder,
    comp_latency_map: &mut HashMap<Id, Option<u64>>,
) -> CalyxResult<StaticControl> {
    let ir_stmts = stmts
        .into_iter()
        .map(|c| build_static_control(c, comp_latency_map, builder))
        .collect::<CalyxResult<Vec<_>>>()?;
    let inferred_latency =
        ir_stmts.iter().fold(0, |acc, s| acc + (s.get_latency()));
    assert_latencies_eq(latency, inferred_latency);
    let mut s = StaticControl::seq(ir_stmts, inferred_latency);
    *s.get_mut_attributes() = attributes;
    Ok(s)
}

fn build_static_par(
    stmts: Vec<ast::Control>,
    attributes: Attributes,
    latency: Option<NonZeroU64>,
    builder: &mut Builder,
    comp_latency_map: &mut HashMap<Id, Option<u64>>,
) -> CalyxResult<StaticControl> {
    let ir_stmts = stmts
        .into_iter()
        .map(|c| build_static_control(c, comp_latency_map, builder))
        .collect::<CalyxResult<Vec<_>>>()?;
    let inferred_latency = match ir_stmts.iter().max_by_key(|s| s.get_latency())
    {
        Some(s) => s.get_latency(),
        None => {
            return Err(Error::malformed_control("empty par block".to_string()))
        }
    };
    assert_latencies_eq(latency, inferred_latency);
    let mut p = StaticControl::par(ir_stmts, inferred_latency);
    *p.get_mut_attributes() = attributes;
    Ok(p)
}

fn build_static_if(
    port: ast::Port,
    tbranch: ast::Control,
    fbranch: ast::Control,
    attributes: Attributes,
    latency: Option<NonZeroU64>,
    builder: &mut Builder,
    comp_latency_map: &mut HashMap<Id, Option<u64>>,
) -> CalyxResult<StaticControl> {
    let ir_tbranch = build_static_control(tbranch, comp_latency_map, builder)?;
    let ir_fbranch = build_static_control(fbranch, comp_latency_map, builder)?;
    let inferred_latency =
        std::cmp::max(ir_tbranch.get_latency(), ir_fbranch.get_latency());
    assert_latencies_eq(latency, inferred_latency);
    let mut con = StaticControl::static_if(
        ensure_direction(
            get_port_ref(port, builder.component)?,
            Direction::Output,
        )?,
        Box::new(ir_tbranch),
        Box::new(ir_fbranch),
        inferred_latency,
    );
    *con.get_mut_attributes() = attributes;
    Ok(con)
}

// builds a static invoke from the given information
fn build_static_invoke(
    builder: &mut Builder,
    component: Id,
    (inputs, outputs): (InvokePortMap, InvokePortMap),
    attributes: Attributes,
    ref_cells: Vec<(Id, Id)>,
    given_latency: Option<std::num::NonZeroU64>,
    comp_latency_map: &mut HashMap<Id, Option<u64>>,
) -> CalyxResult<StaticControl> {
    let cell = Rc::clone(&builder.component.find_cell(component).ok_or_else(
        || {
            Error::undefined(component, "cell".to_string())
                .with_pos(&attributes)
        },
    )?);
    let comp_name = cell
        .borrow()
        .type_name()
        .unwrap_or_else(|| unreachable!("invoked component without a name"));
    // infers the latency of the invoke using comp_latency_map
    let comp_latency = match comp_latency_map
        .get(&comp_name)
        .unwrap_or_else(|| unreachable!("component {} not found", comp_name))
    {
        Some(v) => v,
        None => {
            return Err(Error::malformed_control(format!(
                "non-static component {} is statically invoked",
                comp_name
            ))
            .with_pos(&attributes))
        }
    };
    assert_latencies_eq(given_latency, *comp_latency);

    let inputs = inputs
        .into_iter()
        .map(|(id, port)| {
            atom_to_port(port, builder)
                .and_then(|pr| ensure_direction(pr, Direction::Output))
                .map(|p| (id, p))
        })
        .collect::<Result<_, _>>()?;
    let outputs = outputs
        .into_iter()
        .map(|(id, port)| {
            atom_to_port(port, builder)
                .and_then(|pr| ensure_direction(pr, Direction::Input))
                .map(|p| (id, p))
        })
        .collect::<Result<_, _>>()?;
    let mut inv = StaticInvoke {
        comp: cell,
        inputs,
        outputs,
        attributes: Attributes::default(),
        ref_cells: Vec::new(),
        latency: *comp_latency,
    };
    if !ref_cells.is_empty() {
        let mut ext_cell_tuples = Vec::new();
        for (outcell, incell) in ref_cells {
            let ext_cell_ref = builder
                .component
                .find_cell(incell)
                .ok_or_else(|| Error::undefined(incell, "cell".to_string()))?;
            ext_cell_tuples.push((outcell, ext_cell_ref));
        }
        inv.ref_cells = ext_cell_tuples;
    }
    let mut con = StaticControl::Invoke(inv);
    *con.get_mut_attributes() = attributes;
    Ok(con)
}

fn build_static_repeat(
    num_repeats: u64,
    body: ast::Control,
    builder: &mut Builder,
    attributes: Attributes,
    comp_latency_map: &mut HashMap<Id, Option<u64>>,
) -> CalyxResult<StaticControl> {
    let body = build_static_control(body, comp_latency_map, builder)?;
    let total_latency = body.get_latency() * num_repeats;
    let mut scon =
        StaticControl::repeat(num_repeats, total_latency, Box::new(body));
    *scon.get_mut_attributes() = attributes;
    Ok(scon)
}

// checks whether `control` is static
fn build_static_control(
    control: ast::Control,
    comp_latency_map: &mut HashMap<Id, Option<u64>>,
    builder: &mut Builder,
) -> CalyxResult<StaticControl> {
    let sc = match control {
        ast::Control::Enable {
            comp: group,
            attributes,
        } => {
            if builder.component.find_group(group).is_some() {
                // dynamic group called in build_static_control
                return Err(Error::malformed_control(
                    "found dynamic group in static context".to_string(),
                )
                .with_pos(&attributes));
            };
            let mut en = StaticControl::from(Rc::clone(
                &builder.component.find_static_group(group).ok_or_else(
                    || {
                        Error::undefined(group, "group".to_string())
                            .with_pos(&attributes)
                    },
                )?,
            ));
            *en.get_mut_attributes() = attributes;
            en
        }
        ast::Control::StaticInvoke {
            comp,
            inputs,
            outputs,
            attributes,
            ref_cells,
            latency,
        } => {
            return build_static_invoke(
                builder,
                comp,
                (inputs, outputs),
                attributes,
                ref_cells,
                latency,
                comp_latency_map,
            )
        }
        ast::Control::StaticSeq {
            stmts,
            attributes,
            latency,
        } => {
            return build_static_seq(
                stmts,
                attributes,
                latency,
                builder,
                comp_latency_map,
            )
        }
        ast::Control::StaticPar {
            stmts,
            attributes,
            latency,
        } => {
            return build_static_par(
                stmts,
                attributes,
                latency,
                builder,
                comp_latency_map,
            )
        }
        ast::Control::StaticIf {
            port,
            tbranch,
            fbranch,
            attributes,
            latency,
        } => {
            return build_static_if(
                port,
                *tbranch,
                *fbranch,
                attributes,
                latency,
                builder,
                comp_latency_map,
            )
        }
        ast::Control::StaticRepeat {
            attributes,
            num_repeats,
            body,
        } => {
            return build_static_repeat(
                num_repeats,
                *body,
                builder,
                attributes,
                comp_latency_map,
            )
        }
        ast::Control::Par { .. }
        | ast::Control::If { .. }
        | ast::Control::While { .. }
        | ast::Control::Seq { .. }
        | ast::Control::Invoke { .. } => {
            return Err(Error::malformed_control(
                "found dynamic control in static context".to_string(),
            )
            .with_pos(control.get_attributes()));
        }
        ast::Control::Empty { attributes } => {
            let mut emp = StaticControl::empty();
            *emp.get_mut_attributes() = attributes;
            emp
        } // ast::Control::Invoke { .. } => {
          //     todo!("implement frontend parsing for invoke")
          // }
    };
    Ok(sc)
}

/// Transform ast::Control to ir::Control.
fn build_control(
    control: ast::Control,
    comp_latency_map: &mut HashMap<Id, Option<u64>>,
    builder: &mut Builder,
) -> CalyxResult<Control> {
    let c = match control {
        ast::Control::Enable {
            comp: component,
            attributes,
        } => match builder.component.find_group(component) {
            Some(g) => {
                let mut en = Control::enable(Rc::clone(&g));
                *en.get_mut_attributes() = attributes;
                en
            }
            None => {
                let mut en = Control::Static(StaticControl::from(Rc::clone(
                    &builder
                        .component
                        .find_static_group(component)
                        .ok_or_else(|| {
                            Error::undefined(component, "group".to_string())
                                .with_pos(&attributes)
                        })?,
                )));
                *en.get_mut_attributes() = attributes;
                en
            }
        },
        ast::Control::StaticInvoke {
            comp,
            inputs,
            outputs,
            attributes,
            ref_cells,
            latency,
        } => {
            let i = build_static_invoke(
                builder,
                comp,
                (inputs, outputs),
                attributes,
                ref_cells,
                latency,
                comp_latency_map,
            );
            Control::Static(i?)
        }
        ast::Control::Invoke {
            comp: component,
            inputs,
            outputs,
            attributes,
            comb_group,
            ref_cells,
        } => {
            let cell = Rc::clone(
                &builder.component.find_cell(component).ok_or_else(|| {
                    Error::undefined(component, "cell".to_string())
                        .with_pos(&attributes)
                })?,
            );

            let comp_name = cell.borrow().type_name().unwrap_or_else(|| {
                unreachable!("invoked component without a name")
            });

            // Not sure if it should be ane error to dynamically invoke static
            // component
            // match comp_latency_map.get(&comp_name).unwrap_or_else(|| {
            //     unreachable!("component {} not found", comp_name)
            // }) {
            //     Some(_) => {
            //         return Err(Error::malformed_control(format!(
            //             "static component {} is dynamically invoked",
            //             comp_name
            //         ))
            //         .with_pos(&attributes))
            //     }
            //     None => (),
            // };

            let inputs = inputs
                .into_iter()
                .map(|(id, port)| {
                    atom_to_port(port, builder)
                        .and_then(|pr| ensure_direction(pr, Direction::Output))
                        .map(|p| (id, p))
                })
                .collect::<Result<_, _>>()?;
            let outputs = outputs
                .into_iter()
                .map(|(id, port)| {
                    atom_to_port(port, builder)
                        .and_then(|pr| ensure_direction(pr, Direction::Input))
                        .map(|p| (id, p))
                })
                .collect::<Result<_, _>>()?;
            let mut inv = Invoke {
                comp: cell,
                inputs,
                outputs,
                attributes,
                comb_group: None,
                ref_cells: Vec::new(),
            };
            if let Some(cg) = comb_group {
                let cg_ref =
                    builder.component.find_comb_group(cg).ok_or_else(|| {
                        Error::undefined(cg, "combinational group".to_string())
                            .with_pos(&inv.attributes)
                    })?;
                inv.comb_group = Some(cg_ref);
            }
            if !ref_cells.is_empty() {
                let mut ext_cell_tuples = Vec::new();
                for (outcell, incell) in ref_cells {
                    let ext_cell_ref =
                        builder.component.find_cell(incell).ok_or_else(
                            || Error::undefined(incell, "cell".to_string()),
                        )?;
                    ext_cell_tuples.push((outcell, ext_cell_ref));
                }
                inv.ref_cells = ext_cell_tuples;
            }
            Control::Invoke(inv)
        }
        ast::Control::Seq { stmts, attributes } => {
            let mut s = Control::seq(
                stmts
                    .into_iter()
                    .map(|c| build_control(c, comp_latency_map, builder))
                    .collect::<CalyxResult<Vec<_>>>()?,
            );
            *s.get_mut_attributes() = attributes;
            s
        }
        ast::Control::StaticSeq {
            stmts,
            attributes,
            latency,
        } => {
            let s = build_static_seq(
                stmts,
                attributes,
                latency,
                builder,
                comp_latency_map,
            );
            Control::Static(s?)
        }
        ast::Control::StaticPar {
            stmts,
            attributes,
            latency,
        } => {
            let s = build_static_par(
                stmts,
                attributes,
                latency,
                builder,
                comp_latency_map,
            );
            Control::Static(s?)
        }
        ast::Control::StaticIf {
            port,
            tbranch,
            fbranch,
            attributes,
            latency,
        } => {
            let s = build_static_if(
                port,
                *tbranch,
                *fbranch,
                attributes,
                latency,
                builder,
                comp_latency_map,
            );
            Control::Static(s?)
        }
        ast::Control::StaticRepeat {
            attributes,
            num_repeats,
            body,
        } => {
            let s = build_static_repeat(
                num_repeats,
                *body,
                builder,
                attributes,
                comp_latency_map,
            );
            Control::Static(s?)
        }
        ast::Control::Par { stmts, attributes } => {
            let mut p = Control::par(
                stmts
                    .into_iter()
                    .map(|c| build_control(c, comp_latency_map, builder))
                    .collect::<CalyxResult<Vec<_>>>()?,
            );
            *p.get_mut_attributes() = attributes;
            p
        }
        ast::Control::If {
            port,
            cond: maybe_cond,
            tbranch,
            fbranch,
            attributes,
        } => {
            let group = maybe_cond
                .map(|cond| {
                    builder.component.find_comb_group(cond).ok_or_else(|| {
                        Error::undefined(
                            cond,
                            "combinational group".to_string(),
                        )
                        .with_pos(&attributes)
                    })
                })
                .transpose()?;
            let mut con = Control::if_(
                ensure_direction(
                    get_port_ref(port, builder.component)?,
                    Direction::Output,
                )?,
                group,
                Box::new(build_control(*tbranch, comp_latency_map, builder)?),
                Box::new(build_control(*fbranch, comp_latency_map, builder)?),
            );
            *con.get_mut_attributes() = attributes;
            con
        }
        ast::Control::While {
            port,
            cond: maybe_cond,
            body,
            attributes,
        } => {
            let group = maybe_cond
                .map(|cond| {
                    builder.component.find_comb_group(cond).ok_or_else(|| {
                        Error::undefined(
                            cond,
                            "combinational group".to_string(),
                        )
                        .with_pos(&attributes)
                    })
                })
                .transpose()?;
            let mut con = Control::while_(
                ensure_direction(
                    get_port_ref(port, builder.component)?,
                    Direction::Output,
                )?,
                group,
                Box::new(build_control(*body, comp_latency_map, builder)?),
            );
            *con.get_mut_attributes() = attributes;
            con
        }
        ast::Control::Empty { attributes } => {
            let mut emp = Control::empty();
            *emp.get_mut_attributes() = attributes;
            emp
        }
    };
    Ok(c)
}
