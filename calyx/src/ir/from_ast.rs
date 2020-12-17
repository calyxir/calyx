use super::{
    Assignment, Builder, CellType, Component, Context, Control, Guard, Id,
    Port, RRC,
};
use crate::{
    errors::{Error, FutilResult},
    frontend::ast,
    frontend::library,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Context to store the signature information for all defined primitives and
/// components.
#[derive(Default)]
struct SigCtx {
    /// Mapping from component names to signatures
    comp_sigs: HashMap<Id, ast::Signature>,

    /// Mapping from library functions to signatures
    lib_sigs: HashMap<Id, library::ast::Primitive>,
}

/// Extend the signature with magical ports.
fn extend_signature(sig: &mut ast::Signature) {
    // XXX(Sam): checking to see if the port exists is a hack.
    // The solution to the 'four big problems' will solve this.
    if sig.inputs.iter().find(|(name, _)| name == "go").is_none() {
        sig.inputs.push(("go".into(), 1))
    }

    if sig.inputs.iter().find(|(name, _)| name == "clk").is_none() {
        sig.inputs.push(("clk".into(), 1))
    }

    if sig
        .outputs
        .iter()
        .find(|(name, _)| name == "done")
        .is_none()
    {
        sig.outputs.push(("done".into(), 1))
    }
}

/// Construct an IR representation using a parsed AST and command line options.
pub fn ast_to_ir(
    mut components: Vec<ast::ComponentDef>,
    libraries: &[library::ast::Library],
    import_statements: Vec<String>,
    debug_mode: bool,
    synthesis_mode: bool,
) -> FutilResult<Context> {
    // Build the signature context
    let mut sig_ctx = SigCtx::default();

    // Add primitive signatures
    for library in libraries {
        sig_ctx.lib_sigs.extend(
            library
                .primitives
                .iter()
                .map(|prim| (prim.name.clone(), prim.clone())),
        );
    }

    // Add component signatures to context
    for comp in &mut components {
        // extend the signature
        extend_signature(&mut comp.signature);
        sig_ctx
            .comp_sigs
            .insert(comp.name.clone(), comp.signature.clone());
    }

    let comps = components
        .into_iter()
        .map(|comp| build_component(comp, &sig_ctx))
        .collect::<Result<_, _>>()?;

    Ok(Context {
        components: comps,
        lib_sigs: sig_ctx.lib_sigs,
        import_statements,
        debug_mode,
        synthesis_mode,
    })
}

fn validate_component(
    comp: &ast::ComponentDef,
    sig_ctx: &SigCtx,
) -> FutilResult<()> {
    let mut cells = HashSet::new();
    let mut groups = HashSet::new();

    for cell in &comp.cells {
        let name = cell.name();
        if cells.contains(name) {
            return Err(Error::AlreadyBound(name.clone(), "cell".to_string()));
        }
        cells.insert(name.clone());

        match cell {
            ast::Cell::Prim { prim, .. } => {
                if !sig_ctx.lib_sigs.contains_key(prim) {
                    return Err(Error::Undefined(
                        prim.clone(),
                        "primitive".to_string(),
                    ));
                }
            }
            ast::Cell::Decl { component, .. } => {
                if !sig_ctx.comp_sigs.contains_key(component) {
                    return Err(Error::Undefined(
                        component.clone(),
                        "component".to_string(),
                    ));
                }
            }
        }
    }

    for group in &comp.groups {
        let name = &group.name;
        if groups.contains(name) {
            return Err(Error::AlreadyBound(name.clone(), "group".to_string()));
        }
        if cells.contains(name) {
            return Err(Error::AlreadyBound(name.clone(), "cell".to_string()));
        }
        groups.insert(name.clone());
    }

    Ok(())
}

/// Build an `ir::component::Component` using an `frontend::ast::ComponentDef`.
fn build_component(
    comp: ast::ComponentDef,
    sig_ctx: &SigCtx,
) -> FutilResult<Component> {
    // Validate the component before building it.
    validate_component(&comp, sig_ctx)?;

    // Cell to represent the signature of this component
    let mut ir_component = Component::new(
        comp.name,
        comp.signature.inputs,
        comp.signature.outputs,
    );
    let mut builder =
        Builder::from(&mut ir_component, &sig_ctx.lib_sigs, false);

    // For each ast::Cell, add a Cell that contains all the
    // required information.
    comp.cells
        .into_iter()
        .for_each(|cell| add_cell(cell, &sig_ctx, &mut builder));

    comp.groups
        .into_iter()
        .map(|g| add_group(g, &mut builder))
        .collect::<FutilResult<()>>()?;

    let continuous_assignments = comp
        .continuous_assignments
        .into_iter()
        .map(|w| build_assignment(w, &mut builder))
        .collect::<FutilResult<Vec<_>>>()?;
    builder.component.continuous_assignments = continuous_assignments;

    // Build the Control ast using ast::Control.
    let control = Rc::new(RefCell::new(build_control(
        comp.control,
        &builder.component,
    )?));
    builder.component.control = control;

    Ok(ir_component)
}

///////////////// Cell Construction /////////////////////////

fn add_cell(cell: ast::Cell, sig_ctx: &SigCtx, builder: &mut Builder) {
    match cell {
        ast::Cell::Decl {
            name: prefix,
            component,
        } => {
            let name = builder.component.generate_name(prefix);
            let sig = &sig_ctx.comp_sigs[&component];
            let typ = CellType::Component {
                name: component.clone(),
            };
            let cell = Builder::cell_from_signature(
                name,
                typ,
                sig.inputs.clone(),
                sig.outputs.clone(),
            );
            builder.component.cells.push(cell);
        }
        ast::Cell::Prim { name, prim, params } => {
            builder.add_primitive(name, prim, &params);
        }
    }
}

///////////////// Group Construction /////////////////////////

/// Build an IR group using the AST Group.
fn add_group(group: ast::Group, builder: &mut Builder) -> FutilResult<()> {
    let ir_group = builder.add_group(group.name, group.attributes);

    // Add assignemnts to the group
    for wire in group.wires {
        let assign = build_assignment(wire, builder)?;
        ir_group.borrow_mut().assignments.push(assign)
    }

    Ok(())
}

///////////////// Assignment Construction /////////////////////////

/// Get the pointer to the Port represented by `port`.
fn get_port_ref(port: ast::Port, comp: &Component) -> FutilResult<RRC<Port>> {
    match port {
        ast::Port::Comp { component, port } => comp
            .find_cell(&component)
            .ok_or_else(|| {
                Error::Undefined(component.clone(), "cell".to_string())
            })?
            .borrow()
            .find(&port)
            .ok_or_else(|| Error::Undefined(port, "port".to_string())),
        ast::Port::This { port } => {
            comp.signature.borrow().find(&port).ok_or_else(|| {
                Error::Undefined(port, "component port".to_string())
            })
        }
        ast::Port::Hole { group, name: port } => comp
            .find_group(&group)
            .ok_or_else(|| Error::Undefined(group, "group".to_string()))?
            .borrow()
            .find(&port)
            .ok_or_else(|| Error::Undefined(port, "hole".to_string())),
    }
}

/// Get an port using an ast::Atom.
/// If the atom is a number and the context doesn't already contain a cell
/// for this constant, instantiate the constant node and get the "out" port
/// from it.
fn atom_to_port(
    atom: ast::Atom,
    builder: &mut Builder,
) -> FutilResult<RRC<Port>> {
    match atom {
        ast::Atom::Num(n) => {
            let port = builder.add_constant(n.val, n.width).borrow().get("out");
            Ok(Rc::clone(&port))
        }
        ast::Atom::Port(p) => get_port_ref(p, &builder.component),
    }
}

/// Build an ir::Assignment from ast::Wire.
/// The Assignment contains pointers to the relevant ports.
fn build_assignment(
    wire: ast::Wire,
    builder: &mut Builder,
) -> FutilResult<Assignment> {
    let src_port: RRC<Port> = atom_to_port(wire.src.expr, builder)?;
    let dst_port: RRC<Port> = get_port_ref(wire.dest, &builder.component)?;
    let guard = match wire.src.guard {
        Some(g) => build_guard(g, builder)?,
        None => Guard::True,
    };

    Ok(builder.build_assignment(dst_port, src_port, guard))
}

/// Transform an ast::GuardExpr to an ir::Guard.
fn build_guard(guard: ast::GuardExpr, bd: &mut Builder) -> FutilResult<Guard> {
    use ast::GuardExpr as GE;

    let into_box_guard = |g: Box<GE>, bd: &mut Builder| -> FutilResult<_> {
        Ok(Box::new(build_guard(*g, bd)?))
    };

    Ok(match guard {
        GE::Atom(atom) => Guard::Port(atom_to_port(atom, bd)?),
        GE::And(gs) => Guard::And(
            gs.into_iter()
                .map(|g| into_box_guard(Box::new(g), bd).map(|b| *b))
                .collect::<FutilResult<Vec<_>>>()?,
        ),
        GE::Or(gs) => Guard::Or(
            gs.into_iter()
                .map(|g| into_box_guard(Box::new(g), bd).map(|b| *b))
                .collect::<FutilResult<Vec<_>>>()?,
        ),
        GE::Eq(l, r) => {
            Guard::Eq(into_box_guard(l, bd)?, into_box_guard(r, bd)?)
        }
        GE::Neq(l, r) => {
            Guard::Neq(into_box_guard(l, bd)?, into_box_guard(r, bd)?)
        }
        GE::Gt(l, r) => {
            Guard::Gt(into_box_guard(l, bd)?, into_box_guard(r, bd)?)
        }
        GE::Lt(l, r) => {
            Guard::Lt(into_box_guard(l, bd)?, into_box_guard(r, bd)?)
        }
        GE::Geq(l, r) => {
            Guard::Geq(into_box_guard(l, bd)?, into_box_guard(r, bd)?)
        }
        GE::Leq(l, r) => {
            Guard::Leq(into_box_guard(l, bd)?, into_box_guard(r, bd)?)
        }
        GE::Not(g) => Guard::Not(into_box_guard(g, bd)?),
    })
}

///////////////// Control Construction /////////////////////////

/// Transform ast::Control to ir::Control.
fn build_control(
    control: ast::Control,
    comp: &Component,
) -> FutilResult<Control> {
    Ok(match control {
        ast::Control::Enable { comp: component } => Control::enable(Rc::clone(
            &comp.find_group(&component).ok_or_else(|| {
                Error::Undefined(component.clone(), "group".to_string())
            })?,
        )),
        ast::Control::Seq { stmts } => Control::seq(
            stmts
                .into_iter()
                .map(|c| build_control(c, comp))
                .collect::<FutilResult<Vec<_>>>()?,
        ),
        ast::Control::Par { stmts } => Control::par(
            stmts
                .into_iter()
                .map(|c| build_control(c, comp))
                .collect::<FutilResult<Vec<_>>>()?,
        ),
        ast::Control::If {
            port,
            cond,
            tbranch,
            fbranch,
        } => Control::if_(
            get_port_ref(port, comp)?,
            Rc::clone(&comp.find_group(&cond).ok_or_else(|| {
                Error::Undefined(cond.clone(), "group".to_string())
            })?),
            Box::new(build_control(*tbranch, comp)?),
            Box::new(build_control(*fbranch, comp)?),
        ),
        ast::Control::While { port, cond, body } => Control::while_(
            get_port_ref(port, comp)?,
            Rc::clone(&comp.find_group(&cond).ok_or_else(|| {
                Error::Undefined(cond.clone(), "group".to_string())
            })?),
            Box::new(build_control(*body, comp)?),
        ),
        ast::Control::Empty { .. } => Control::empty(),
    })
}
