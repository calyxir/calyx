//! Abstract Syntax Tree for library declarations in FuTIL
use crate::errors::{Error, FutilResult};
use crate::ir;
use std::collections::HashMap;

pub type LibrarySignatures = HashMap<ir::Id, Primitive>;

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
    pub name: ir::Id,
    /// Paramters for this primitive.
    pub params: Vec<ir::Id>,
    /// The input/output signature for this primitive.
    pub signature: Vec<ParamPortdef>,
    /// Key-value attributes for this primitive.
    pub attributes: HashMap<String, u64>,
    /// Available implementations for this primitive.
    pub implementation: Vec<Implementation>,
}

impl Primitive {
    /// Retuns the bindings for all the paramters, the input ports and the
    /// output ports.
    pub fn resolve(
        &self,
        parameters: &[u64],
    ) -> FutilResult<(Vec<(ir::Id, u64)>, Vec<(ir::Id, u64)>, Vec<(ir::Id, u64)>)>
    {
        let bindings = self
            .params
            .iter()
            .cloned()
            .zip(parameters.iter().cloned())
            .collect::<HashMap<ir::Id, u64>>();

        let (input, output): (Vec<ParamPortdef>, Vec<ParamPortdef>) = self
            .signature
            .iter()
            .cloned()
            .partition(|ppd| ppd.direction == ir::Direction::Input);

        let inps = input
            .iter()
            .map(|ppd| ppd.resolve(&bindings))
            .collect::<FutilResult<_>>()?;
        let outs = output
            .iter()
            .map(|ppd| ppd.resolve(&bindings))
            .collect::<FutilResult<_>>()?;

        Ok((bindings.into_iter().collect(), inps, outs))
    }
}

/// A parameter port definition.
#[derive(Clone, Debug)]
pub struct ParamPortdef {
    pub name: ir::Id,
    pub width: Width,
    pub direction: ir::Direction,
}

/// Represents an abstract width of a primitive signature.
#[derive(Clone, Debug)]
pub enum Width {
    /// The width is a constant.
    Const { value: u64 },
    /// The width is a parameter.
    Param { value: ir::Id },
}

impl ParamPortdef {
    pub fn resolve(
        &self,
        val_map: &HashMap<ir::Id, u64>,
    ) -> FutilResult<(ir::Id, u64)> {
        match &self.width {
            Width::Const { value } => Ok((self.name.clone(), *value)),
            Width::Param { value } => match val_map.get(&value) {
                Some(width) => Ok((self.name.clone(), *width)),
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
