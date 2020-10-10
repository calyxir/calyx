use super::{
    Assignment, Cell, CellType, Component, Control, Direction, Empty, Enable,
    Group, Guard, Par, Port, Seq, While, RRC, WRC,
};
use crate::{
    errors::{Error, Result},
    lang::ast,
    lang::library,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
struct TransformCtx {
    /// Mapping from component names to signatures
    comp_sigs: HashMap<ast::Id, ast::Signature>,

    /// Mapping from library functions to signatures
    lib_sigs: HashMap<ast::Id, library::ast::Primitive>,

    /// Mapping from Id to Cells
    cell_map: HashMap<ast::Id, RRC<Cell>>,

    /// Mapping from Id to Groups
    group_map: HashMap<ast::Id, RRC<Group>>,
}

pub fn ast_to_ir<'a>(namespace: ast::NamespaceDef) -> Result<Component> {
    unimplemented!()
}

/// Build an `ir::component::Component` using an `lang::ast::ComponentDef`.
fn build_component<'a>(comp: ast::ComponentDef) -> Result<Component> {
    let mut ctx = TransformCtx::default();

    // For each ast::Cell, build an Cell that contains all the
    // required information.
    let cells = comp
        .cells
        .into_iter()
        .map(|cell| build_cell(cell, &mut ctx))
        .collect::<Result<Vec<_>>>()?;

    // Build Groups and Assignments using Connections.
    let (mut ast_groups, mut continuous_assigns) = (vec![], vec![]);
    for conn in comp.connections.into_iter() {
        match conn {
            ast::Connection::Group(g) => ast_groups.push(g),
            ast::Connection::Wire(w) => continuous_assigns.push(w),
        }
    }

    let groups = ast_groups.into_iter().map(|g| build_group(g, &mut ctx));

    // Build the Control ast using ast::Control.
    unimplemented!()
}

///////////////// Cell Construction /////////////////////////

fn build_cell(cell: ast::Cell, ctx: &mut TransformCtx) -> Result<RRC<Cell>> {
    // Get the name, inputs, and outputs.
    let (name, typ, inputs, outputs) = match cell {
        ast::Cell::Decl {
            data: ast::Decl { name, component },
        } => {
            let sig = ctx
                .comp_sigs
                .get(&component)
                .ok_or_else(|| Error::UndefinedComponent(name.clone()))?;
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
            let prim_sig = ctx
                .lib_sigs
                .get(&prim_name)
                .ok_or_else(|| Error::UndefinedComponent(name.clone()))?;
            let param_bindings = prim_sig
                .params
                .iter()
                .zip(instance.params)
                .collect::<HashMap<&ast::Id, u64>>();
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
                                (ppd.name, param_bindings[&value])
                            }
                        })
                        .collect::<Vec<_>>()
                };
            let inputs = instantiate_ports(&prim_sig.signature.inputs);
            let outputs = instantiate_ports(&prim_sig.signature.outputs);
            (name, CellType::Primitive, inputs, outputs)
        }
    };
    // Construct the Cell
    let cell = Rc::new(RefCell::new(Cell {
        ports: vec![],
        prototype: typ,
    }));
    // Construct ports
    for (name, width) in inputs {
        let port = Rc::new(RefCell::new(Port {
            id: name,
            width: width,
            direction: Direction::Input,
            cell: Rc::downgrade(&cell),
        }));
        cell.borrow_mut().ports.push(port);
    }
    for (name, width) in outputs {
        let port = Rc::new(RefCell::new(Port {
            id: name,
            width: width,
            direction: Direction::Output,
            cell: Rc::downgrade(&cell),
        }));
        cell.borrow_mut().ports.push(port);
    }

    // Add this cell to context
    ctx.cell_map.insert(name, Rc::clone(&cell));
    Ok(cell)
}

/// Build a Cell representing a number.
fn build_constant(
    num: ast::BitNum,
    ctx: &mut TransformCtx,
) -> Result<RRC<Cell>> {
    let cell = Rc::new(RefCell::new(Cell {
        ports: vec![],
        prototype: CellType::Constant,
    }));

    // Constants only has an out port
    let out = Rc::new(RefCell::new(Port {
        id: "out".into(),
        width: num.width,
        direction: Direction::Output,
        cell: Rc::downgrade(&cell),
    }));

    cell.borrow_mut().ports.push(out);

    // Add this constant to cell_map mapping a string for this constant
    // to this cell.
    ctx.cell_map
        .insert(num.val.to_string().into(), Rc::clone(&cell));

    Ok(cell)
}

///////////////// Wires Construction /////////////////////////

/// Build an IR group using the AST Group.
fn build_group(group: ast::Group, ctx: &mut TransformCtx) -> Result<Group> {
    let assigns = group
        .wires
        .into_iter()
        .map(|w| build_assignment(w, ctx))
        .collect::<Result<Vec<_>>>()?;
    Ok(Group {
        name: group.name,
        assignments: assigns,
    })
}

/// Get the pointer to the Port represented by `port`.
fn get_port_ref(port: ast::Port, ctx: &TransformCtx) -> Result<RRC<Port>> {
    let (comp, port) = match port {
        ast::Port::Comp { component, port } => (component, port),
        ast::Port::This { port } => ("this".into(), port),
        ast::Port::Hole { .. } => unimplemented!(),
    };
    let cell = ctx
        .cell_map
        .get(&comp)
        .ok_or_else(|| Error::UndefinedComponent(comp.clone()))?;

    Ok(Rc::clone(
        cell.borrow()
            .ports
            .iter()
            .find(|p| p.borrow().id == port)
            .ok_or_else(|| {
                Error::UndefinedPort(comp.clone(), port.to_string())
            })?,
    ))
}

/// Get an port using an ast::Atom.
/// If the atom is a number and the context doesn't already contain a cell
/// for this constant, instantiate the constant node and get the "out" port
/// from it.
fn atom_to_port(atom: ast::Atom, ctx: &mut TransformCtx) -> Result<RRC<Port>> {
    match atom {
        ast::Atom::Num(n) => {
            let key: ast::Id = n.val.to_string().into();
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
                .find(|p| p.borrow().id == port_name)
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
) -> Result<Assignment> {
    let src_port: RRC<Port> = atom_to_port(wire.src.expr, ctx)?;
    let dst_port: RRC<Port> = get_port_ref(wire.dest, ctx)?;
    let guard = match wire.src.guard {
        Some(g) => Some(build_guard(g, ctx)?),
        None => None,
    };

    Ok(Assignment {
        dst: dst_port,
        src: src_port,
        guard: guard,
    })
}

/// Transform an ast::GuardExpr to an ir::Guard.
fn build_guard(guard: ast::GuardExpr, ctx: &mut TransformCtx) -> Result<Guard> {
    use ast::GuardExpr as GE;

    let into_box_guard = |g: Box<GE>, ctx: &mut TransformCtx| -> Result<_> {
        Ok(Box::new(build_guard(*g, ctx)?))
    };

    Ok(match guard {
        GE::Atom(atom) => Guard::Port(atom_to_port(atom, ctx)?),
        GE::And(gs) => Guard::And(
            gs.into_iter()
                .map(|g| into_box_guard(Box::new(g), ctx).map(|b| *b))
                .collect::<Result<Vec<_>>>()?,
        ),
        GE::Or(gs) => Guard::Or(
            gs.into_iter()
                .map(|g| into_box_guard(Box::new(g), ctx).map(|b| *b))
                .collect::<Result<Vec<_>>>()?,
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
fn build_control(control: ast::Control, ctx: &TransformCtx) -> Result<Control> {
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
                .collect::<Result<Vec<_>>>()?,
        ),
        ast::Control::Par {
            data: ast::Par { stmts },
        } => Control::par(
            stmts
                .into_iter()
                .map(|c| build_control(c, ctx))
                .collect::<Result<Vec<_>>>()?,
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
