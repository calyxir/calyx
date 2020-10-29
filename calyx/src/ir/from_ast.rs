use super::{
    Assignment, Builder, Cell, CellType, Component, Context, Control, Guard,
    Id, Port, RRC,
};
use crate::{
    errors::{Error, FutilResult},
    frontend::ast,
    frontend::library,
};
use std::cell::RefCell;
use std::collections::HashMap;
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

fn extend_signature(sig: &mut ast::Signature) {
    sig.inputs.push(ast::Portdef {
        name: "go".into(),
        width: 1,
    });
    sig.inputs.push(ast::Portdef {
        name: "clk".into(),
        width: 1,
    });
    sig.outputs.push(ast::Portdef {
        name: "done".into(),
        width: 1,
    });
}

/// Construct an IR representation using a parsed AST and command line options.
pub fn ast_to_ir(
    mut components: Vec<ast::ComponentDef>,
    libraries: &[library::ast::Library],
    import_statements: Vec<String>,
    debug_mode: bool,
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
    })
}

/// Build an `ir::component::Component` using an `frontend::ast::ComponentDef`.
fn build_component(
    comp: ast::ComponentDef,
    sig_ctx: &SigCtx,
) -> FutilResult<Component> {
    // Cell to represent the signature of this component
    let mut ir_component = Component::new(
        comp.name,
        comp.signature
            .inputs
            .into_iter()
            .map(|pd| (pd.name, pd.width))
            .collect(),
        comp.signature
            .outputs
            .into_iter()
            .map(|pd| (pd.name, pd.width))
            .collect(),
    );

    // For each ast::Cell, build an Cell that contains all the
    // required information.
    let cells = comp
        .cells
        .into_iter()
        .map(|cell| build_cell(cell, &sig_ctx))
        .collect::<FutilResult<Vec<_>>>()?;
    ir_component.cells = cells;

    // Build Groups and Assignments using Connections.
    let (mut ast_groups, mut continuous) = (vec![], vec![]);
    for conn in comp.connections.into_iter() {
        match conn {
            ast::Connection::Group(g) => ast_groups.push(g),
            ast::Connection::Wire(w) => continuous.push(w),
        }
    }

    let mut builder =
        Builder::from(&mut ir_component, &sig_ctx.lib_sigs, false);

    ast_groups
        .into_iter()
        .map(|g| build_group(g, &mut builder))
        .collect::<FutilResult<()>>()?;

    let continuous_assignments = continuous
        .into_iter()
        .map(|w| build_assignment(w, &mut builder))
        .collect::<FutilResult<Vec<_>>>()?;
    ir_component.continuous_assignments = continuous_assignments;

    // Build the Control ast using ast::Control.
    let control =
        Rc::new(RefCell::new(build_control(comp.control, &ir_component)?));
    ir_component.control = control;

    Ok(ir_component)
}

///////////////// Cell Construction /////////////////////////

fn build_cell(cell: ast::Cell, sig_ctx: &SigCtx) -> FutilResult<RRC<Cell>> {
    // Get the name, inputs, and outputs.
    let res: FutilResult<(Id, CellType, Vec<_>, Vec<_>)> = match cell {
        ast::Cell::Decl {
            data: ast::Decl { name, component },
        } => {
            let sig = sig_ctx
                .comp_sigs
                .get(&component)
                .ok_or_else(|| Error::UndefinedComponent(component.clone()))?;
            Ok((
                name,
                CellType::Component {
                    name: component.clone(),
                },
                sig.inputs
                    .iter()
                    .cloned()
                    .map(|pd| (pd.name, pd.width))
                    .collect::<Vec<_>>(),
                sig.outputs
                    .iter()
                    .cloned()
                    .map(|pd| (pd.name, pd.width))
                    .collect::<Vec<_>>(),
            ))
        }
        ast::Cell::Prim {
            data: ast::Prim { name, instance },
        } => {
            let prim_name = instance.name;
            let prim_sig = sig_ctx
                .lib_sigs
                .get(&prim_name)
                .ok_or_else(|| Error::UndefinedComponent(prim_name.clone()))?;
            let (param_binding, inputs, outputs) =
                prim_sig.resolve(&instance.params)?;
            Ok((
                name,
                CellType::Primitive {
                    name: prim_name,
                    param_binding,
                },
                inputs,
                outputs,
            ))
        }
    };
    let (name, typ, inputs, outputs) = res?;
    // Construct the Cell
    let cell = Builder::cell_from_signature(name, typ, inputs, outputs);

    Ok(cell)
}

///////////////// Group Construction /////////////////////////

/// Build an IR group using the AST Group.
fn build_group(group: ast::Group, builder: &mut Builder) -> FutilResult<()> {
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
            .ok_or_else(|| Error::UndefinedComponent(component.clone()))?
            .borrow()
            .find(&port)
            .ok_or_else(|| {
                Error::UndefinedPort(component.clone(), port.to_string())
            }),
        ast::Port::This { port } => {
            comp.signature.borrow().find(&port).ok_or_else(|| {
                Error::UndefinedPort(comp.name.clone(), port.to_string())
            })
        }
        ast::Port::Hole { group, name: port } => comp
            .find_group(&group)
            .ok_or_else(|| Error::UndefinedGroup(group.clone()))?
            .borrow()
            .find(&port)
            .ok_or_else(|| {
                Error::UndefinedPort(group.clone(), port.to_string())
            }),
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
            &comp
                .find_group(&component)
                .ok_or_else(|| Error::UndefinedGroup(component.clone()))?,
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
            Rc::clone(
                &comp
                    .find_group(&cond)
                    .ok_or_else(|| Error::UndefinedGroup(cond.clone()))?,
            ),
            Box::new(build_control(*tbranch, comp)?),
            Box::new(build_control(*fbranch, comp)?),
        ),
        ast::Control::While { port, cond, body } => Control::while_(
            get_port_ref(port, comp)?,
            Rc::clone(
                &comp
                    .find_group(&cond)
                    .ok_or_else(|| Error::UndefinedGroup(cond.clone()))?,
            ),
            Box::new(build_control(*body, comp)?),
        ),
        ast::Control::Empty { .. } => Control::empty(),
    })
}
