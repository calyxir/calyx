// Abstract Syntax Tree for library declarations in Futil
use crate::errors::Error;
use crate::lang::ast::{Id, Portdef};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Library {
    pub primitives: Vec<Primitive>,
}

#[derive(Clone, Debug)]
pub struct Primitive {
    pub name: Id,
    pub params: Vec<Id>,
    pub signature: ParamSignature,
    pub implementation: Vec<Implementation>,
}

#[derive(Clone, Debug)]
pub struct ParamSignature {
    pub inputs: Vec<ParamPortdef>,
    pub outputs: Vec<ParamPortdef>,
}

#[derive(Clone, Debug)]
pub struct ParamPortdef {
    pub name: Id,
    pub width: Width,
}

#[derive(Clone, Debug)]
pub enum Width {
    Const { value: u64 },
    Param { value: Id },
}

impl ParamSignature {
    /// Returns an iterator over the inputs of signature
    pub fn inputs(&self) -> std::slice::Iter<ParamPortdef> {
        self.inputs.iter()
    }

    /// Returns an iterator over the outputs of signature
    pub fn outputs(&self) -> std::slice::Iter<ParamPortdef> {
        self.outputs.iter()
    }
}

impl ParamPortdef {
    pub fn resolve(
        &self,
        val_map: &HashMap<&Id, u64>,
    ) -> Result<Portdef, Error> {
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
                None => {
                    Err(Error::SignatureResolutionFailed(self.name.clone()))
                }
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
