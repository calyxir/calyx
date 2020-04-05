// Abstract Syntax Tree for library declarations in Futil
use crate::errors::Error;
use crate::lang::ast::{Id, Portdef};
use sexpy::Sexpy;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn parse_file(file: &PathBuf) -> Result<Library, Error> {
    let content = &fs::read(file)?;
    let string_content = std::str::from_utf8(content)?;
    match Library::parse(string_content) {
        Ok(ns) => Ok(ns),
        Err(msg) => Err(Error::ParseError(msg)),
    }
}

#[derive(Sexpy, Clone, Debug)]
#[sexpy(head = "define/library")]
pub struct Library {
    pub primitives: Vec<Primitive>,
}

#[derive(Sexpy, Clone, Debug)]
#[sexpy(head = "define/prim")]
pub struct Primitive {
    pub name: Id,
    #[sexpy(surround)]
    pub params: Vec<Id>,
    pub signature: ParamSignature,
}

#[derive(Clone, Debug, Sexpy)]
#[sexpy(nohead, nosurround)]
pub struct ParamSignature {
    #[sexpy(surround)]
    pub inputs: Vec<ParamPortdef>,
    #[sexpy(surround)]
    pub outputs: Vec<ParamPortdef>,
}

#[derive(Sexpy, Clone, Debug)]
#[sexpy(head = "port")]
pub struct ParamPortdef {
    pub name: Id,
    pub width: Width,
}

#[derive(Sexpy, Clone, Debug)]
#[sexpy(nohead, nosurround)]
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
