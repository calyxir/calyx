use super::{
    Assignment, Attributes, BackendConf, Builder, Canonical, CellType,
    Component, Context, Control, Direction, GetAttributes, Guard, Id, Invoke,
    LibrarySignatures, Port, PortDef, RESERVED_NAMES, RRC,
};
use crate::{
    errors::{CalyxResult, Error},
    frontend::{self, ast},
    ir::PortComp,
    utils::{GPosIdx, NameGenerator, WithPos},
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

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
const INTERFACE_PORTS: [(&str, u64, Direction); 4] = [
    ("go", 1, Direction::Input),
    ("clk", 1, Direction::Input),
    ("reset", 1, Direction::Input),
    ("done", 1, Direction::Output),
];

/// Extend the signature with magical ports.
fn extend_signature(sig: &mut Vec<PortDef<u64>>) {
    let port_names: HashSet<_> = sig.iter().map(|pd| pd.name).collect();
    let mut namegen = NameGenerator::with_prev_defined_names(port_names);
    for (name, width, direction) in INTERFACE_PORTS.iter() {
        // Check if there is already another interface port defined for the
        // component
        if !sig.iter().any(|pd| pd.attributes.has(name)) {
            let mut attributes = Attributes::default();
            attributes.insert(name, 1);
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
pub fn ast_to_ir(mut workspace: frontend::Workspace) -> CalyxResult<Context> {
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
        if !comp.attributes.has("nointerface") && !comp.is_comb {
            extend_signature(sig);
        }
        sig_ctx.comp_sigs.insert(comp.name, sig.clone());
    }

    let comps: Vec<Component> = workspace
        .components
        .into_iter()
        .map(|comp| build_component(comp, &mut sig_ctx))
        .collect::<Result<_, _>>()?;

    // Find the entrypoint for the program.
    let entrypoint = comps
        .iter()
        .find(|c| c.attributes.get("toplevel").is_some())
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

        let proto_name = &cell.prototype.name;

        if sig_ctx.lib.find_primitive(proto_name).is_none()
            && !sig_ctx.comp_sigs.contains_key(proto_name)
        {
            return Err(Error::undefined(
                *proto_name,
                "primitive or component".to_string(),
            ));
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

    let mut ir_component =
        Component::new(comp.name, comp.signature, comp.is_comb);
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

    let continuous_assignments =
        build_assignments(comp.continuous_assignments, &mut builder)?;
    builder.component.continuous_assignments = continuous_assignments;

    // Build the Control ast using ast::Control.
    let control =
        Rc::new(RefCell::new(build_control(comp.control, &mut builder)?));
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
    let proto_name = &cell.prototype.name;

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
        let sig = &sig_ctx.comp_sigs[proto_name];
        let typ = CellType::Component { name: *proto_name };
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

///////////////// Assignment Construction /////////////////////////

/// Get the pointer to the Port represented by `port`.
fn get_port_ref(port: ast::Port, comp: &Component) -> CalyxResult<RRC<Port>> {
    match port {
        ast::Port::Comp { component, port } => comp
            .find_cell(&component)
            .ok_or_else(|| Error::undefined(component, "cell".to_string()))?
            .borrow()
            .find(port)
            .ok_or_else(|| Error::undefined(port, "port".to_string())),
        ast::Port::This { port } => {
            comp.signature.borrow().find(&port).ok_or_else(|| {
                Error::undefined(port, "component port".to_string())
            })
        }
        ast::Port::Hole { group, name: port } => comp
            .find_group(&group)
            .ok_or_else(|| Error::undefined(group, "group".to_string()))?
            .borrow()
            .find(port)
            .ok_or_else(|| Error::undefined(port, "hole".to_string())),
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
) -> CalyxResult<Assignment> {
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

fn build_assignments(
    assigns: Vec<ast::Wire>,
    builder: &mut Builder,
) -> CalyxResult<Vec<Assignment>> {
    assigns
        .into_iter()
        .map(|w| {
            let attrs = w.attributes.clone();
            build_assignment(w, builder).map_err(|err| err.with_pos(&attrs))
        })
        .collect::<CalyxResult<Vec<_>>>()
}

/// Transform an ast::GuardExpr to an ir::Guard.
fn build_guard(guard: ast::GuardExpr, bd: &mut Builder) -> CalyxResult<Guard> {
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
        GE::CompOp(op, l, r) => {
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

/// Transform ast::Control to ir::Control.
fn build_control(
    control: ast::Control,
    builder: &mut Builder,
) -> CalyxResult<Control> {
    Ok(match control {
        ast::Control::Enable {
            comp: component,
            attributes,
        } => {
            let mut en = Control::enable(Rc::clone(
                &builder.component.find_group(&component).ok_or_else(|| {
                    Error::undefined(component, "group".to_string())
                        .with_pos(&attributes)
                })?,
            ));
            *en.get_mut_attributes() = attributes;
            en
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
                &builder.component.find_cell(&component).ok_or_else(|| {
                    Error::undefined(component, "cell".to_string())
                        .with_pos(&attributes)
                })?,
            );
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
                let cg_ref = builder
                    .component
                    .find_comb_group(&cg)
                    .ok_or_else(|| {
                        Error::undefined(cg, "combinational group".to_string())
                            .with_pos(&inv.attributes)
                    })?;
                inv.comb_group = Some(cg_ref);
            }
            if !ref_cells.is_empty() {
                let mut ext_cell_tuples = Vec::new();
                for (outcell, incell) in ref_cells {
                    let ext_cell_ref =
                        builder.component.find_cell(&incell).ok_or_else(
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
                    .map(|c| build_control(c, builder))
                    .collect::<CalyxResult<Vec<_>>>()?,
            );
            *s.get_mut_attributes() = attributes;
            s
        }
        ast::Control::Par { stmts, attributes } => {
            let mut p = Control::par(
                stmts
                    .into_iter()
                    .map(|c| build_control(c, builder))
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
                    builder.component.find_comb_group(&cond).ok_or_else(|| {
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
                Box::new(build_control(*tbranch, builder)?),
                Box::new(build_control(*fbranch, builder)?),
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
                    builder.component.find_comb_group(&cond).ok_or_else(|| {
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
                Box::new(build_control(*body, builder)?),
            );
            *con.get_mut_attributes() = attributes;
            con
        }
        ast::Control::Empty { attributes } => {
            let mut emp = Control::empty();
            *emp.get_mut_attributes() = attributes;
            emp
        }
    })
}
