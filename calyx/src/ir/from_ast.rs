use super::{
    Assignment, Cell, CellType, Component, Context, Control, Direction, Group,
    Guard, Port, PortParent, RRC,
};
use crate::{
    errors::{Error, FutilResult},
    frontend::ast,
    frontend::library,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const THIS_ID: &str = "this";

/// Context to store the signature information for all defined primitives and
/// components.
#[derive(Default)]
struct SigCtx {
    /// Mapping from component names to signatures
    comp_sigs: HashMap<ast::Id, ast::Signature>,

    /// Mapping from library functions to signatures
    lib_sigs: HashMap<ast::Id, library::ast::Primitive>,
}

/// Component-specific transformation context.
struct TransformCtx<'a> {
    /// Immutable reference top the global signature context.
    sig_ctx: &'a SigCtx,

    /// Mapping from Id to Cells
    cell_map: HashMap<ast::Id, RRC<Cell>>,

    /// Mapping from Id to Groups
    group_map: HashMap<ast::Id, RRC<Group>>,
}

/// Construct an IR representation using a parsed AST and command line options.
pub fn ast_to_ir(
    components: Vec<ast::ComponentDef>,
    libraries: &[library::ast::Library],
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

    // Add component signatures
    for comp in &components {
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
        debug_mode,
    })
}

/// Build an `ir::component::Component` using an `frontend::ast::ComponentDef`.
fn build_component(
    comp: ast::ComponentDef,
    sig_ctx: &SigCtx,
) -> FutilResult<Component> {
    let mut ctx = TransformCtx {
        sig_ctx,
        cell_map: HashMap::new(),
        group_map: HashMap::new(),
    };

    // Cell to represent the signature of this component
    let signature = Component::cell_from_signature(
        THIS_ID.into(),
        CellType::ThisComponent,
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
    // Add signature to the context
    ctx.cell_map.insert(THIS_ID.into(), Rc::clone(&signature));

    // For each ast::Cell, build an Cell that contains all the
    // required information.
    let cells = comp
        .cells
        .into_iter()
        .map(|cell| build_cell(cell, &mut ctx))
        .collect::<FutilResult<Vec<_>>>()?;

    // Build Groups and Assignments using Connections.
    let (mut ast_groups, mut continuous) = (vec![], vec![]);
    for conn in comp.connections.into_iter() {
        match conn {
            ast::Connection::Group(g) => ast_groups.push(g),
            ast::Connection::Wire(w) => continuous.push(w),
        }
    }

    let groups = ast_groups
        .into_iter()
        .map(|g| build_group(g, &mut ctx))
        .collect::<FutilResult<Vec<_>>>()?;

    let continuous_assignments = continuous
        .into_iter()
        .map(|w| build_assignment(w, &mut ctx))
        .collect::<FutilResult<Vec<_>>>()?;

    // Build the Control ast using ast::Control.
    let control = Rc::new(RefCell::new(build_control(comp.control, &ctx)?));

    Ok(Component {
        name: comp.name,
        signature,
        cells,
        groups,
        continuous_assignments,
        control,
    })
}

///////////////// Cell Construction /////////////////////////

fn build_cell(
    cell: ast::Cell,
    ctx: &mut TransformCtx,
) -> FutilResult<RRC<Cell>> {
    // Get the name, inputs, and outputs.
    let (name, typ, inputs, outputs) =
        match cell {
            ast::Cell::Decl {
                data: ast::Decl { name, component },
            } => {
                let sig =
                    ctx.sig_ctx.comp_sigs.get(&component).ok_or_else(|| {
                        Error::UndefinedComponent(name.clone())
                    })?;
                (
                    name,
                    CellType::Component,
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
                )
            }
            ast::Cell::Prim {
                data: ast::Prim { name, instance },
            } => {
                let prim_name = instance.name;
                let prim_sig =
                    ctx.sig_ctx.lib_sigs.get(&prim_name).ok_or_else(|| {
                        Error::UndefinedComponent(name.clone())
                    })?;
                let param_binding = prim_sig
                    .params
                    .iter()
                    .cloned()
                    .zip(instance.params)
                    .collect::<HashMap<ast::Id, u64>>();
                let instantiate_ports =
                    |ports: &Vec<library::ast::ParamPortdef>| {
                        ports
                            .iter()
                            .cloned()
                            .map(|ppd| match ppd.width {
                                library::ast::Width::Const { value } => {
                                    (ppd.name, value)
                                }
                                library::ast::Width::Param { value } => {
                                    (ppd.name, param_binding[&value])
                                }
                            })
                            .collect::<Vec<_>>()
                    };
                let inputs = instantiate_ports(&prim_sig.signature.inputs);
                let outputs = instantiate_ports(&prim_sig.signature.outputs);
                (
                    name.clone(),
                    CellType::Primitive {
                        name,
                        param_binding,
                    },
                    inputs,
                    outputs,
                )
            }
        };
    // Construct the Cell
    let cell =
        Component::cell_from_signature(name.clone(), typ, inputs, outputs);

    // Add this cell to context
    ctx.cell_map.insert(name, Rc::clone(&cell));
    Ok(cell)
}

/// Build a Cell representing a number.
fn build_constant(
    num: ast::BitNum,
    ctx: &mut TransformCtx,
) -> FutilResult<RRC<Cell>> {
    let name: ast::Id = Cell::constant_name(num.val, num.width);
    let cell = Component::cell_from_signature(
        name.clone(),
        CellType::Constant,
        vec![],
        vec![("out".into(), num.width)],
    );

    // Add this constant to cell_map mapping a string for this constant
    // to this cell.
    ctx.cell_map.insert(name, Rc::clone(&cell));

    Ok(cell)
}

///////////////// Group Construction /////////////////////////

/// Build an IR group using the AST Group.
fn build_group(
    group: ast::Group,
    ctx: &mut TransformCtx,
) -> FutilResult<RRC<Group>> {
    let ir_group = Rc::new(RefCell::new(Group {
        name: group.name.clone(),
        assignments: vec![],
        holes: vec![],
        attributes: group.attributes,
    }));

    // Add default holes to this Group
    for (name, width) in vec![("go", 1), ("done", 1)] {
        let hole = Rc::new(RefCell::new(Port {
            name: name.into(),
            width,
            direction: Direction::Inout,
            parent: PortParent::Group(Rc::downgrade(&ir_group)),
        }));
        ir_group.borrow_mut().holes.push(hole);
    }

    // Add this group to the group map. We need to do this before constructing
    // the assignments since an assignments might use this group's holes.
    ctx.group_map.insert(group.name, Rc::clone(&ir_group));

    // Add assignemnts to the group
    for wire in group.wires {
        let assign = build_assignment(wire, ctx)?;
        ir_group.borrow_mut().assignments.push(assign)
    }

    Ok(ir_group)
}

///////////////// Assignment Construction /////////////////////////

/// Get the pointer to the Port represented by `port`.
fn get_port_ref(port: ast::Port, ctx: &TransformCtx) -> FutilResult<RRC<Port>> {
    let find_and_clone_port = |comp: &ast::Id,
                               port_name: ast::Id,
                               all_ports: &[RRC<Port>]|
     -> FutilResult<RRC<Port>> {
        all_ports
            .iter()
            .find(|p| p.borrow().name == port_name)
            .map(|p| Rc::clone(p))
            .ok_or_else(|| {
                Error::UndefinedPort(comp.clone(), port_name.to_string())
            })
    };

    match port {
        ast::Port::Comp { component, port } => {
            let cell = ctx
                .cell_map
                .get(&component)
                .ok_or_else(|| Error::UndefinedComponent(component.clone()))?
                .borrow();
            find_and_clone_port(&component, port, &cell.ports)
        }
        ast::Port::This { port } => {
            let cell = ctx.cell_map.get(&THIS_ID.into()).unwrap().borrow();
            find_and_clone_port(&THIS_ID.into(), port, &cell.ports)
        }
        ast::Port::Hole { group, name } => {
            let group_ref = ctx
                .group_map
                .get(&group)
                .ok_or_else(|| Error::UndefinedGroup(group.clone()))?
                .borrow();
            find_and_clone_port(&group, name, &group_ref.holes)
        }
    }
}

/// Get an port using an ast::Atom.
/// If the atom is a number and the context doesn't already contain a cell
/// for this constant, instantiate the constant node and get the "out" port
/// from it.
fn atom_to_port(
    atom: ast::Atom,
    ctx: &mut TransformCtx,
) -> FutilResult<RRC<Port>> {
    match atom {
        ast::Atom::Num(n) => {
            let key: ast::Id = Cell::constant_name(n.val, n.width);
            let cell = if ctx.cell_map.contains_key(&key) {
                Rc::clone(&ctx.cell_map[&key])
            } else {
                build_constant(n, ctx)?
            };

            let port_name: ast::Id = "out".into();

            let borrowed_cell = cell.borrow();
            let port = borrowed_cell
                .ports
                .iter()
                .find(|p| p.borrow().name == port_name)
                .expect("Constant doesn't have the out port.");

            Ok(Rc::clone(&port))
        }
        ast::Atom::Port(p) => get_port_ref(p, ctx),
    }
}

/// Build an ir::Assignment from ast::Wire.
/// The Assignment contains pointers to the relevant ports.
fn build_assignment(
    wire: ast::Wire,
    ctx: &mut TransformCtx,
) -> FutilResult<Assignment> {
    let src_port: RRC<Port> = atom_to_port(wire.src.expr, ctx)?;
    let dst_port: RRC<Port> = get_port_ref(wire.dest, ctx)?;
    let guard = match wire.src.guard {
        Some(g) => Some(build_guard(g, ctx)?),
        None => None,
    };

    Ok(Assignment {
        dst: dst_port,
        src: src_port,
        guard,
    })
}

/// Transform an ast::GuardExpr to an ir::Guard.
fn build_guard(
    guard: ast::GuardExpr,
    ctx: &mut TransformCtx,
) -> FutilResult<Guard> {
    use ast::GuardExpr as GE;

    let into_box_guard =
        |g: Box<GE>, ctx: &mut TransformCtx| -> FutilResult<_> {
            Ok(Box::new(build_guard(*g, ctx)?))
        };

    Ok(match guard {
        GE::Atom(atom) => Guard::Port(atom_to_port(atom, ctx)?),
        GE::And(gs) => Guard::And(
            gs.into_iter()
                .map(|g| into_box_guard(Box::new(g), ctx).map(|b| *b))
                .collect::<FutilResult<Vec<_>>>()?,
        ),
        GE::Or(gs) => Guard::Or(
            gs.into_iter()
                .map(|g| into_box_guard(Box::new(g), ctx).map(|b| *b))
                .collect::<FutilResult<Vec<_>>>()?,
        ),
        GE::Eq(l, r) => {
            Guard::Eq(into_box_guard(l, ctx)?, into_box_guard(r, ctx)?)
        }
        GE::Neq(l, r) => {
            Guard::Neq(into_box_guard(l, ctx)?, into_box_guard(r, ctx)?)
        }
        GE::Gt(l, r) => {
            Guard::Gt(into_box_guard(l, ctx)?, into_box_guard(r, ctx)?)
        }
        GE::Lt(l, r) => {
            Guard::Lt(into_box_guard(l, ctx)?, into_box_guard(r, ctx)?)
        }
        GE::Geq(l, r) => {
            Guard::Geq(into_box_guard(l, ctx)?, into_box_guard(r, ctx)?)
        }
        GE::Leq(l, r) => {
            Guard::Leq(into_box_guard(l, ctx)?, into_box_guard(r, ctx)?)
        }
        GE::Not(g) => Guard::Not(into_box_guard(g, ctx)?),
    })
}

///////////////// Control Construction /////////////////////////

/// Transform ast::Control to ir::Control.
fn build_control(
    control: ast::Control,
    ctx: &TransformCtx,
) -> FutilResult<Control> {
    Ok(match control {
        ast::Control::Enable {
            data: ast::Enable { comp },
        } => Control::enable(Rc::clone(
            ctx.group_map
                .get(&comp)
                .ok_or_else(|| Error::UndefinedGroup(comp.clone()))?,
        )),
        ast::Control::Seq {
            data: ast::Seq { stmts },
        } => Control::seq(
            stmts
                .into_iter()
                .map(|c| build_control(c, ctx))
                .collect::<FutilResult<Vec<_>>>()?,
        ),
        ast::Control::Par {
            data: ast::Par { stmts },
        } => Control::par(
            stmts
                .into_iter()
                .map(|c| build_control(c, ctx))
                .collect::<FutilResult<Vec<_>>>()?,
        ),
        ast::Control::If {
            data:
                ast::If {
                    port,
                    cond,
                    tbranch,
                    fbranch,
                },
        } => Control::if_(
            get_port_ref(port, ctx)?,
            Rc::clone(
                ctx.group_map
                    .get(&cond)
                    .ok_or_else(|| Error::UndefinedGroup(cond.clone()))?,
            ),
            Box::new(build_control(*tbranch, ctx)?),
            Box::new(build_control(*fbranch, ctx)?),
        ),
        ast::Control::While {
            data: ast::While { port, cond, body },
        } => Control::while_(
            get_port_ref(port, ctx)?,
            Rc::clone(
                ctx.group_map
                    .get(&cond)
                    .ok_or_else(|| Error::UndefinedGroup(cond.clone()))?,
            ),
            Box::new(build_control(*body, ctx)?),
        ),
        ast::Control::Empty { .. } => Control::empty(),
        ast::Control::Print { .. } => {
            unreachable!("Print statements are not supported by the IR.")
        }
    })
}
