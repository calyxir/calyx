use super::{
    Assignment, Attributes, BackendConf, Builder, Cell, CellType, Component,
    Context, Control, Direction, GetAttributes, Guard, Id, Invoke,
    LibrarySignatures, Port, PortDef, StaticControl, StaticInvoke,
    RESERVED_NAMES, RRC,
};
use crate::{Nothing, PortComp, StaticTiming};
use calyx_frontend::{ast, BoolAttr, NumAttr, Workspace};
use calyx_utils::{CalyxResult, Error, GPosIdx, WithPos};
use itertools::Itertools;

use std::collections::{HashMap, HashSet};
use std::num::NonZeroU64;
use std::rc::Rc;

/// Context to store the signature information for all defined primitives and
/// components.
#[derive(Default)]
struct SigCtx {
    /// Mapping from component names to (signature, Option<static_latency>)
    comp_sigs: HashMap<Id, (Vec<PortDef<u64>>, Option<NonZeroU64>)>,

    /// Mapping from library functions to signatures
    lib: LibrarySignatures,
}

// Assumes cell has a name (i.e., not a constant/ThisComponent)
// uses sig_ctx to check the latency of comp_name, either thru static<n> or
// @interval(n)
fn get_comp_latency(
    sig_ctx: &SigCtx,
    cell: RRC<Cell>,
    attrs: &Attributes,
) -> CalyxResult<Option<NonZeroU64>> {
    let comp_name = cell
        .borrow()
        .type_name()
        .unwrap_or_else(|| unreachable!("invoked component without a name"));
    if let Some(prim) = sig_ctx.lib.find_primitive(comp_name) {
        match prim.latency {
            Some(val) => Ok(Some(val)),
            None => {
                let prim_sig = &prim.signature;
                // We can just look at any go port, since if there is an
                // @interval(n), it must be the same for all ports.
                let interval_value = prim_sig
                    .iter()
                    .find(|port_def| port_def.attributes.has(NumAttr::Go))
                    .and_then(|go_port| {
                        go_port.attributes.get(NumAttr::Interval)
                    });
                // Annoying thing we have to do bc NonZeroU64.
                match interval_value {
                    Some(lat) => Ok(NonZeroU64::new(lat)),
                    None => Ok(None),
                }
            }
        }
    } else if let Some((comp_sig, latency)) = sig_ctx.comp_sigs.get(&comp_name)
    {
        match latency {
            Some(val) => Ok(Some(*val)),
            None => {
                // We can just look at any go port, since if there is an
                // @interval(n), it must be the same for all ports.
                let interval_value = comp_sig
                    .iter()
                    .find(|port_def| port_def.attributes.has(NumAttr::Go))
                    .and_then(|go_port| {
                        go_port.attributes.get(NumAttr::Interval)
                    });
                // Annoying thing we have to do bc NonZeroU64.
                match interval_value {
                    Some(lat) => Ok(NonZeroU64::new(lat)),
                    None => Ok(None),
                }
            }
        }
    } else {
        return Err(Error::undefined(
            comp_name,
            "primitive or component".to_string(),
        )
        .with_pos(attrs));
    }
}

// Assumes cell has a name (i.e., not a constant/ThisComponent)
// uses sig_ctx to check the latency of comp_name, but only checks static<n> latency
fn get_static_latency(
    sig_ctx: &SigCtx,
    cell: RRC<Cell>,
    attrs: &Attributes,
) -> CalyxResult<Option<NonZeroU64>> {
    let comp_name = cell
        .borrow()
        .type_name()
        .unwrap_or_else(|| unreachable!("invoked component without a name"));
    if let Some(prim) = sig_ctx.lib.find_primitive(comp_name) {
        Ok(prim.latency)
    } else if let Some((_, latency)) = sig_ctx.comp_sigs.get(&comp_name) {
        Ok(*latency)
    } else {
        return Err(Error::undefined(
            comp_name,
            "primitive or component".to_string(),
        )
        .with_pos(attrs));
    }
}

// assumes cell has a name (i.e., not a constant/ThisComponent)
// Uses sig_ctx to check whether port_name is a valid port on cell.
fn check_valid_port(
    cell: RRC<Cell>,
    port_name: &Id,
    attrs: &Attributes,
    sig_ctx: &SigCtx,
) -> CalyxResult<()> {
    let cell_name = cell.borrow().name();
    let comp_name = cell
        .borrow()
        .type_name()
        .unwrap_or_else(|| unreachable!("invoked component without a name"));
    let sig_ports: HashSet<_> =
        if let Some(prim) = sig_ctx.lib.find_primitive(comp_name) {
            prim.signature
                .iter()
                .map(|port_def| port_def.name())
                .collect()
        } else if let Some((comp_sigs, _)) = sig_ctx.comp_sigs.get(&comp_name) {
            comp_sigs.iter().map(|port_def| port_def.name()).collect()
        } else {
            return Err(Error::undefined(
                comp_name,
                "primitive or component".to_string(),
            )
            .with_pos(attrs));
        };
    if !sig_ports.contains(port_name) {
        return Err(Error::malformed_structure(format!(
            "cell `{}` (which is an instance of {}) does not have port named `{}`",
            cell_name, comp_name, port_name
        ))
        .with_pos(attrs));
    };
    Ok(())
}

/// Validates a component signature to make sure there are not duplicate ports.
fn check_signature(pds: &[PortDef<u64>]) -> CalyxResult<()> {
    let mut ports: HashSet<Id> = HashSet::new();
    for pd in pds {
        let name = pd.name();
        // check for uniqueness
        match &pd.direction {
            Direction::Input => {
                if !ports.contains(&name) {
                    ports.insert(name);
                } else {
                    return Err(Error::already_bound(name, "port".to_string()));
                }
            }
            Direction::Output => {
                if !ports.contains(&name) {
                    ports.insert(name);
                } else {
                    return Err(Error::already_bound(name, "port".to_string()));
                }
            }
            Direction::Inout => {
                panic!("Components shouldn't have inout ports.")
            }
        }
    }
    Ok(())
}

/// Construct an IR representation using a parsed AST and command line options.
pub fn ast_to_ir(mut workspace: Workspace) -> CalyxResult<Context> {
    let prims = workspace.lib.signatures().collect_vec();
    let mut all_names: HashSet<&Id> =
        HashSet::with_capacity(workspace.components.len() + prims.len());

    let prim_names = prims.iter().map(|p| &p.name);

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
        lib: workspace.lib,
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
            Component::extend_signature(sig);
        }
        sig_ctx
            .comp_sigs
            .insert(comp.name, (sig.clone(), comp.latency));
    }

    // building components from `ast::ComponentDef`s to `ir::Component`
    let comps: Vec<Component> = workspace
        .components
        .into_iter()
        .map(|comp| build_component(comp, &mut sig_ctx))
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
    sig_ctx: &mut SigCtx,
) -> CalyxResult<Component> {
    // Validate the component before building it.
    validate_component(&comp, sig_ctx)?;

    let mut ir_component = Component::new(
        comp.name,
        comp.signature,
        !comp.attributes.has(BoolAttr::NoInterface) && !comp.is_comb,
        comp.is_comb,
        // we may change latency from None to Some(inferred latency)
        // after we iterate thru the control of the Component
        comp.latency,
    );

    let mut builder =
        Builder::new(&mut ir_component, &sig_ctx.lib).not_generated();

    // For each ast::Cell, add a Cell that contains all the
    // required information.
    comp.cells
        .into_iter()
        .try_for_each(|cell| add_cell(cell, sig_ctx, &mut builder))?;

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
    let control =
        super::rrc(build_control(comp.control, sig_ctx, &mut builder)?);

    builder.component.control = control;

    ir_component.attributes = comp.attributes;

    // Add reserved names to the component's namegenerator so future conflicts
    // don't occur
    ir_component
        .add_names(RESERVED_NAMES.iter().map(|s| Id::from(*s)).collect());

    Ok(ir_component)
}

///////////////// Cell Construction /////////////////////////

fn add_cell(
    cell: ast::Cell,
    sig_ctx: &SigCtx,
    builder: &mut Builder,
) -> CalyxResult<()> {
    let proto_name = cell.prototype.name;

    let res = if sig_ctx.lib.find_primitive(proto_name).is_some() {
        let c = builder
            .try_add_primitive(cell.name, proto_name, &cell.prototype.params)
            .map_err(|e| e.with_pos(&cell.attributes))?;
        c.borrow_mut().set_reference(cell.reference);
        c
    } else {
        // Validator ensures that if the protoype is not a primitive, it
        // is a component.
        let name = builder.component.generate_name(cell.name);
        let sig = &sig_ctx.comp_sigs[&proto_name].0;
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

    Ok(())
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
            let name = pr.borrow().canonical();
            Err(Error::malformed_structure(format!(
                "Port `{name}` occurs in write position but is an output port",
            )))
        }
        (Direction::Output, Direction::Input) => {
            let name = pr.borrow().canonical();
            Err(Error::malformed_structure(format!(
                "Port `{name}` occurs in write position but is an output port",
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
    sig_ctx: &SigCtx,
) -> CalyxResult<StaticControl> {
    let ir_stmts = stmts
        .into_iter()
        .map(|c| build_static_control(c, sig_ctx, builder))
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
    sig_ctx: &SigCtx,
) -> CalyxResult<StaticControl> {
    let ir_stmts = stmts
        .into_iter()
        .map(|c| build_static_control(c, sig_ctx, builder))
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
    sig_ctx: &SigCtx,
) -> CalyxResult<StaticControl> {
    let ir_tbranch = build_static_control(tbranch, sig_ctx, builder)?;
    let ir_fbranch = build_static_control(fbranch, sig_ctx, builder)?;
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

fn build_static_repeat(
    num_repeats: u64,
    body: ast::Control,
    builder: &mut Builder,
    attributes: Attributes,
    sig_ctx: &SigCtx,
) -> CalyxResult<StaticControl> {
    let body = build_static_control(body, sig_ctx, builder)?;
    let total_latency = body.get_latency() * num_repeats;
    let mut scon =
        StaticControl::repeat(num_repeats, total_latency, Box::new(body));
    *scon.get_mut_attributes() = attributes;
    Ok(scon)
}

// checks whether `control` is static
fn build_static_control(
    control: ast::Control,
    sig_ctx: &SigCtx,
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
            comb_group,
        } => {
            let cell = Rc::clone(
                &builder.component.find_cell(comp).ok_or_else(|| {
                    Error::undefined(comp, "cell".to_string())
                        .with_pos(&attributes)
                })?,
            );
            let comp_name = cell.borrow().type_name().unwrap_or_else(|| {
                unreachable!("invoked component without a name")
            });
            let c_latency =
                get_comp_latency(sig_ctx, Rc::clone(&cell), &attributes)?;
            let unwrapped_latency = if let Some(v) = c_latency {
                v
            } else {
                return Err(Error::malformed_control(format!(
                    "component {} is statically invoked, but is neither static nor does it have @interval attribute its @go port",
                    comp_name
                ))
                .with_pos(&attributes));
            };
            assert_latencies_eq(latency, unwrapped_latency.into());

            let inputs = inputs
                .into_iter()
                .map(|(id, port)| {
                    // checking that comp_name.id exists on comp's signature
                    check_valid_port(
                        Rc::clone(&cell),
                        &id,
                        &attributes,
                        sig_ctx,
                    )?;
                    atom_to_port(port, builder)
                        .and_then(|pr| ensure_direction(pr, Direction::Output))
                        .map(|p| (id, p))
                })
                .collect::<Result<_, _>>()?;
            let outputs = outputs
                .into_iter()
                .map(|(id, port)| {
                    // checking that comp_name.id exists on comp's signature
                    check_valid_port(
                        Rc::clone(&cell),
                        &id,
                        &attributes,
                        sig_ctx,
                    )?;
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
                latency: unwrapped_latency.into(),
                comb_group: None,
            };
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
            if let Some(cg) = comb_group {
                let cg_ref =
                    builder.component.find_comb_group(cg).ok_or_else(|| {
                        Error::undefined(cg, "combinational group".to_string())
                            .with_pos(&inv.attributes)
                    })?;
                inv.comb_group = Some(cg_ref);
            }
            let mut con = StaticControl::Invoke(inv);
            *con.get_mut_attributes() = attributes;
            return Ok(con);
        }
        ast::Control::StaticSeq {
            stmts,
            attributes,
            latency,
        } => {
            return build_static_seq(
                stmts, attributes, latency, builder, sig_ctx,
            )
        }
        ast::Control::StaticPar {
            stmts,
            attributes,
            latency,
        } => {
            return build_static_par(
                stmts, attributes, latency, builder, sig_ctx,
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
                port, *tbranch, *fbranch, attributes, latency, builder, sig_ctx,
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
                sig_ctx,
            )
        }
        ast::Control::Par { .. }
        | ast::Control::If { .. }
        | ast::Control::While { .. }
        | ast::Control::Seq { .. }
        | ast::Control::Repeat { .. }
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
        }
    };
    Ok(sc)
}

/// Transform ast::Control to ir::Control.
fn build_control(
    control: ast::Control,
    sig_ctx: &SigCtx,
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
            comb_group,
        } => {
            let cell = Rc::clone(
                &builder.component.find_cell(comp).ok_or_else(|| {
                    Error::undefined(comp, "cell".to_string())
                        .with_pos(&attributes)
                })?,
            );
            let comp_name = cell.borrow().type_name().unwrap_or_else(|| {
                unreachable!("invoked component without a name")
            });
            let c_latency =
                get_comp_latency(sig_ctx, Rc::clone(&cell), &attributes)?;
            let unwrapped_latency = if let Some(v) = c_latency {
                v
            } else {
                return Err(Error::malformed_control(format!(
                    "component {} is statically invoked, but is neither static nor does it have @interval attribute its @go port",
                    comp_name
                ))
                .with_pos(&attributes));
            };
            assert_latencies_eq(latency, unwrapped_latency.into());

            let inputs = inputs
                .into_iter()
                .map(|(id, port)| {
                    // checking that comp_name.id exists on comp's signature
                    check_valid_port(
                        Rc::clone(&cell),
                        &id,
                        &attributes,
                        sig_ctx,
                    )?;
                    atom_to_port(port, builder)
                        .and_then(|pr| ensure_direction(pr, Direction::Output))
                        .map(|p| (id, p))
                })
                .collect::<Result<_, _>>()?;
            let outputs = outputs
                .into_iter()
                .map(|(id, port)| {
                    // checking that comp_name.id exists on comp's signature
                    check_valid_port(
                        Rc::clone(&cell),
                        &id,
                        &attributes,
                        sig_ctx,
                    )?;
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
                latency: unwrapped_latency.into(),
                comb_group: None,
            };
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
            if let Some(cg) = comb_group {
                let cg_ref =
                    builder.component.find_comb_group(cg).ok_or_else(|| {
                        Error::undefined(cg, "combinational group".to_string())
                            .with_pos(&inv.attributes)
                    })?;
                inv.comb_group = Some(cg_ref);
            }
            let mut con = StaticControl::Invoke(inv);
            *con.get_mut_attributes() = attributes;
            Control::Static(con)
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

            // Error to dynamically invoke static component
            if get_static_latency(sig_ctx, Rc::clone(&cell), &attributes)?
                .is_some()
            {
                return Err(Error::malformed_control(format!(
                    "static component {} is dynamically invoked",
                    comp_name
                ))
                .with_pos(&attributes));
            }

            let inputs = inputs
                .into_iter()
                .map(|(id, port)| {
                    // check that comp_name.id is a valid port based on comp_name's signature
                    check_valid_port(
                        Rc::clone(&cell),
                        &id,
                        &attributes,
                        sig_ctx,
                    )?;
                    atom_to_port(port, builder)
                        .and_then(|pr| ensure_direction(pr, Direction::Output))
                        .map(|p| (id, p))
                })
                .collect::<Result<_, _>>()?;
            let outputs = outputs
                .into_iter()
                .map(|(id, port)| {
                    // check that comp_name.id is a valid port based on comp_name's signature
                    check_valid_port(
                        Rc::clone(&cell),
                        &id,
                        &attributes,
                        sig_ctx,
                    )?;
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
                    .map(|c| build_control(c, sig_ctx, builder))
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
            let s =
                build_static_seq(stmts, attributes, latency, builder, sig_ctx);
            Control::Static(s?)
        }
        ast::Control::StaticPar {
            stmts,
            attributes,
            latency,
        } => {
            let s =
                build_static_par(stmts, attributes, latency, builder, sig_ctx);
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
                port, *tbranch, *fbranch, attributes, latency, builder, sig_ctx,
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
                sig_ctx,
            );
            Control::Static(s?)
        }
        ast::Control::Par { stmts, attributes } => {
            let mut p = Control::par(
                stmts
                    .into_iter()
                    .map(|c| build_control(c, sig_ctx, builder))
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
                Box::new(build_control(*tbranch, sig_ctx, builder)?),
                Box::new(build_control(*fbranch, sig_ctx, builder)?),
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
                Box::new(build_control(*body, sig_ctx, builder)?),
            );
            *con.get_mut_attributes() = attributes;
            con
        }
        ast::Control::Repeat {
            num_repeats,
            body,
            attributes,
        } => {
            let mut con = Control::repeat(
                num_repeats,
                Box::new(build_control(*body, sig_ctx, builder)?),
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
