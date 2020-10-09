use super::component::{Cell, Component, CellType, Direction, Port};
use crate::{
    errors::{Error, Result},
    lang::ast,
    lang::library,
};
use std::collections::HashMap;
use std::rc::Rc;

struct TransformCtx {
    /// Mapping from component names to signatures
    comp_sigs: HashMap<ast::Id, ast::Signature>,

    /// Mapping from library functions to signatures
    lib_sigs: HashMap<ast::Id, library::ast::Primitive>,
}

pub fn ast_to_ir<'a>(namespace: ast::NamespaceDef) -> Result<Component<'a>> {
    unimplemented!()
}

/// Build an `ir::component::Component` using an `lang::ast::ComponentDef`.
fn into_component<'a>(comp: ast::ComponentDef) -> Result<Component<'a>> {
    // For each ast::Cell, build an Cell that contains all the
    // required information.

    // Build Groups and Assignments using Connections.

    // Build the Control ast using ast::Control.
    unimplemented!()
}

fn into_cell(cell: ast::Cell, ctx: &TransformCtx) -> Result<Cell> {
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
    let mut cell = Rc::new(Cell {
        ports: vec![],
        prototype: typ,
    });
    // Construct ports
    for (name, width) in inputs {
        let port = Port {
            id: name,
            width: width,
            direction: Direction::Input,
            cell: Rc::downgrade(&Rc::clone(&cell)),
        };
        cell.ports.push(port);
    }

    Ok(cell)
}
