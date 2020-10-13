//! Abstract Syntax Tree for library declarations in FuTIL
use crate::ir;
use crate::errors::{Error, FutilResult};
use crate::frontend::ast::{Id, Portdef};
use std::collections::HashMap;

/// A FuTIL library.
#[derive(Clone, Debug)]
pub struct Library {
    /// The primitives defined by this library.
    pub primitives: Vec<Primitive>,
}

/// Representation of a Primitive.
#[derive(Clone, Debug)]
pub struct Primitive {
    /// Name of this primitive.
    pub name: Id,
    /// Paramters for this primitive.
    pub params: Vec<Id>,
    /// The input/output signature for this primitive.
    pub signature: Vec<ParamPortdef>,
    /// Key-value attributes for this primitive.
    pub attributes: HashMap<String, u64>,
    /// Available implementations for this primitive.
    pub implementation: Vec<Implementation>,
}

/// A parameter port definition.
#[derive(Clone, Debug)]
pub struct ParamPortdef {
    pub name: Id,
    pub width: Width,
    pub direction: ir::Direction,
}

/// Represents an abstract width of a primitive signature.
#[derive(Clone, Debug)]
pub enum Width {
    /// The width is a constant.
    Const { value: u64 },
    /// The width is a parameter.
    Param { value: Id },
}

impl ParamPortdef {
    pub fn resolve(
        &self,
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
                    self.name.clone(),
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
