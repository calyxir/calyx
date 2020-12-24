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
    #[allow(clippy::type_complexity)]
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

        Ok((
            // XXX: Recreating the binding so that they have deterministic
            // order.
            self.params
                .iter()
                .cloned()
                .zip(parameters.iter().cloned())
                .collect(),
            inps,
            outs,
        ))
    }
}

// Parsing for providing particular backend implementations for primitive definitions

#[derive(Clone, Debug)]
pub enum Implementation {
    Verilog(Verilog),
}

#[derive(Clone, Debug)]
pub struct Verilog {
    pub code: String,
}
