//! Abstract Syntax Tree for library declarations in FuTIL
use crate::ir;
use crate::errors::{Error, FutilResult};
use crate::frontend::ast::{Id, Portdef};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Library {
    pub primitives: Vec<Primitive>,
}

#[derive(Clone, Debug)]
pub struct Primitive {
    pub name: Id,
    pub params: Vec<Id>,
    pub signature: Vec<ParamPortdef>,
    pub attributes: HashMap<String, u64>,
    pub implementation: Vec<Implementation>,
}

#[derive(Clone, Debug)]
pub struct ParamPortdef {
    pub name: Id,
    pub width: Width,
    pub direction: ir::Direction,
}

#[derive(Clone, Debug)]
pub enum Width {
    Const { value: u64 },
    Param { value: Id },
}

impl ParamPortdef {
    pub fn resolve(
        &self,
        prim: &Id,
        val_map: &HashMap<&Id, u64>,
    ) -> FutilResult<Portdef> {
        match &self.width {
            Width::Const { value } => Ok(Portdef {
                name: self.name.clone(),
                width: *value,
            }),
            Width::Param { value } => match val_map.get(&value) {
                Some(width) => Ok(Portdef {
                    name: self.name.clone(),
                    width: *width,
                }),
                None => Err(Error::SignatureResolutionFailed(
                    prim.clone(),
                    value.clone(),
                )),
            },
        }
    }
}

// Parsing for providing particular backend implementations for primitive definitions

#[derive(Clone, Debug)]
pub enum Implementation {
    Verilog { data: Verilog },
}

#[derive(Clone, Debug)]
pub struct Verilog {
    pub code: String,
}
