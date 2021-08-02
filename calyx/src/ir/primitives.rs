use super::{Attributes, Direction, Id};
use crate::errors::{CalyxResult, Error};
use linked_hash_map::LinkedHashMap;
use smallvec::SmallVec;

/// Representation of a external primitive definition.
///
/// # Example
/// ```
/// primitive std_reg<"static"=1>[WIDTH](
///   in: WIDTH, write_en: 1, clk: 1
/// ) -> (
///   out: WIDTH, done: 1
/// );
/// ```
///
/// The signature of a port is represented using [`PortDef`] which also specify
/// the direction of the port.
#[derive(Clone, Debug)]
pub struct Primitive {
    /// Name of this primitive.
    pub name: Id,
    /// Paramters for this primitive.
    pub params: Vec<Id>,
    /// The input/output signature for this primitive.
    pub signature: Vec<PortDef>,
    /// Key-value attributes for this primitive.
    pub attributes: Attributes,
}

impl Primitive {
    /// Retuns the bindings for all the paramters, the input ports and the
    /// output ports.
    #[allow(clippy::type_complexity)]
    pub fn resolve(
        &self,
        parameters: &[u64],
    ) -> CalyxResult<(
        SmallVec<[(Id, u64); 5]>,
        Vec<(Id, u64, Direction, Attributes)>,
    )> {
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
            .map(|pd| {
                pd.resolve(&bindings)
                    .map(|(n, w, attrs)| (n, w, pd.direction, attrs))
            })
            .collect::<Result<_, _>>()?;

        Ok((bindings.into_iter().collect(), ports))
    }
}

/// Definition of a port.
#[derive(Clone, Debug)]
pub struct PortDef {
    /// The name of the port.
    pub name: Id,
    /// The width of the port. Can be either a number ([`Width::Const`]) or
    /// a parameter ([`Width::Param`]).
    pub width: Width,
    /// The direction of the port. Only allowed to be [`Direction::Input`]
    /// or [`Direction::Output`].
    pub direction: Direction,
    /// Attributes attached to this port definition
    pub attributes: Attributes,
}

impl From<(Id, u64, Direction)> for PortDef {
    fn from(port: (Id, u64, Direction)) -> Self {
        PortDef {
            name: port.0,
            width: Width::Const { value: port.1 },
            direction: port.2,
            attributes: Attributes::default(),
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
    /// Given a map from names of parameters to their values, attempt to
    /// resolve this definition.
    /// Returns [`SignatureResolutionFailed`](crate::errors::Error::SignatureResolutionFailed) if there is no binding for a required parameter binding.
    pub fn resolve(
        &self,
        binding: &LinkedHashMap<Id, u64>,
    ) -> CalyxResult<(Id, u64, Attributes)> {
        match &self.width {
            Width::Const { value } => {
                Ok((self.name.clone(), *value, self.attributes.clone()))
            }
            Width::Param { value } => match binding.get(value) {
                Some(width) => {
                    Ok((self.name.clone(), *width, self.attributes.clone()))
                }
                None => Err(Error::SignatureResolutionFailed(
                    self.name.clone(),
                    value.clone(),
                )),
            },
        }
    }
}
