use super::{Direction, Id};
use crate::errors::{Error, FutilResult};
use linked_hash_map::LinkedHashMap;
use smallvec::SmallVec;

/// Representation of a Primitive.
#[derive(Clone, Debug)]
pub struct Primitive {
    /// Name of this primitive.
    pub name: Id,
    /// Paramters for this primitive.
    pub params: Vec<Id>,
    /// The input/output signature for this primitive.
    pub signature: Vec<PortDef>,
    /// Key-value attributes for this primitive.
    pub attributes: LinkedHashMap<String, u64>,
}

impl Primitive {
    /// Retuns the bindings for all the paramters, the input ports and the
    /// output ports.
    #[allow(clippy::type_complexity)]
    pub fn resolve(
        &self,
        parameters: &[u64],
    ) -> FutilResult<(SmallVec<[(Id, u64); 5]>, Vec<(Id, u64, Direction)>)>
    {
        let bindings = self
            .params
            .iter()
            .cloned()
            .zip(parameters.iter().cloned())
            .collect::<LinkedHashMap<Id, u64>>();

        let ports = self
            .signature
            .iter()
            .cloned()
            .map(|pd| pd.resolve(&bindings).map(|(n, w)| (n, w, pd.direction)))
            .collect::<Result<_, _>>()?;

        Ok((bindings.into_iter().collect(), ports))
    }

    /// Return the value associated with this attribute key.
    pub fn get_attribute<S>(&self, attr: S) -> Option<&u64>
    where
        S: AsRef<str>,
    {
        self.attributes.get(attr.as_ref())
    }
}

/// A parameter port definition.
#[derive(Clone, Debug)]
pub struct PortDef {
    pub name: Id,
    pub width: Width,
    pub direction: Direction,
}

impl From<(Id, u64, Direction)> for PortDef {
    fn from(port: (Id, u64, Direction)) -> Self {
        PortDef {
            name: port.0,
            width: Width::Const { value: port.1 },
            direction: port.2,
        }
    }
}

/// Represents an abstract width of a primitive signature.
#[derive(Clone, Debug)]
pub enum Width {
    /// The width is a constant.
    Const { value: u64 },
    /// The width is a parameter.
    Param { value: Id },
}

impl PortDef {
    pub fn resolve(
        &self,
        binding: &LinkedHashMap<Id, u64>,
    ) -> FutilResult<(Id, u64)> {
        match &self.width {
            Width::Const { value } => Ok((self.name.clone(), *value)),
            Width::Param { value } => match binding.get(&value) {
                Some(width) => Ok((self.name.clone(), *width)),
                None => Err(Error::SignatureResolutionFailed(
                    self.name.clone(),
                    value.clone(),
                )),
            },
        }
    }
}
