use super::component::{
    Assignment, Cell, CellType, Component, Direction, Group, Guard, Port, RRC,
    WRC,
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
}

pub fn ast_to_ir<'a>(namespace: ast::NamespaceDef) -> Result<Component<'a>> {
    unimplemented!()
}

/// Build an `ir::component::Component` using an `lang::ast::ComponentDef`.
fn into_component<'a>(comp: ast::ComponentDef) -> Result<Component<'a>> {
    let mut ctx = TransformCtx::default();

    // For each ast::Cell, build an Cell that contains all the
    // required information.
    let cells = comp
        .cells
        .into_iter()
        .map(|cell| build_cell(cell, &mut ctx))
        .collect::<Result<Vec<_>>>()?;

    // Build Groups and Assignments using Connections.
    let (mut groups, mut continuous_assigns) = (vec![], vec![]);
    for conn in comp.connections.into_iter() {
        match conn {
            ast::Connection::Group(g) => groups.push(g),
            ast::Connection::Wire(w) => continuous_assigns.push(w),
        }
    }

    // Build the Control ast using ast::Control.
    unimplemented!()
}

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

fn build_group(group: ast::Group, ctx: &mut TransformCtx) -> Result<Group> {
    unimplemented!()
}

fn get_port(port: ast::Port, ctx: &TransformCtx) -> Result<RRC<Port>> {
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
        ast::Atom::Port(p) => get_port(p, ctx),
    }
}

fn build_assignment(
    wire: ast::Wire,
    ctx: &mut TransformCtx,
) -> Result<Assignment> {
    let src_port: RRC<Port> = atom_to_port(wire.src.expr, ctx)?;
    let dst_port: RRC<Port> = get_port(wire.dest, ctx)?;
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

fn build_guard(guard: ast::GuardExpr, ctx: &TransformCtx) -> Result<Guard> {
    unimplemented!()
}
