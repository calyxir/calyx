use super::{Attributes, Direction, GetName, Id};
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
/// The signature of a port is represented using [PortDef] which also specify
/// the direction of the port.
#[derive(Clone, Debug)]
pub struct Primitive {
    /// Name of this primitive.
    pub name: Id,
    /// Paramters for this primitive.
    pub params: Vec<Id>,
    /// The input/output signature for this primitive.
    pub signature: Vec<PortDef<Width>>,
    /// Key-value attributes for this primitive.
    pub attributes: Attributes,
    /// True iff this is a combinational primitive
    pub is_comb: bool,
}

impl Primitive {
    /// Retuns the bindings for all the paramters, the input ports and the
    /// output ports.
    #[allow(clippy::type_complexity)]
    pub fn resolve(
        &self,
        parameters: &[u64],
    ) -> CalyxResult<(SmallVec<[(Id, u64); 5]>, Vec<PortDef<u64>>)> {
        if self.params.len() != parameters.len() {
            let msg = format!(
               "Invalid parameter binding for primitive `{}`. Requires {} parameters but provided with {}.",
               self.name.clone(),
               self.params.len(),
               parameters.len(),
            );
            return Err(Error::malformed_structure(msg));
        }
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
            .map(|pd| pd.resolve(&bindings))
            .collect::<Result<_, _>>()?;

        Ok((bindings.into_iter().collect(), ports))
    }

    /// Return all ports that have the attribute `attr`.
    pub fn find_all_with_attr<'a, S>(
        &'a self,
        attr: S,
    ) -> impl Iterator<Item = &'a PortDef<Width>>
    where
        S: AsRef<str> + 'a,
    {
        self.signature
            .iter()
            .filter(move |&g| g.attributes.has(attr.as_ref()))
    }
}

impl GetName for Primitive {
    fn name(&self) -> &Id {
        &self.name
    }
}

/// Definition of a port parameterized by a width type.
/// Ports on Primitives can be parameteris and use [Width].
/// Ports on Components cannot be parameterized and therefore use `u64`.
#[derive(Clone, Debug)]
pub struct PortDef<W> {
    /// The name of the port.
    pub name: Id,
    /// The width of the port. .
    pub width: W,
    /// The direction of the port. Only allowed to be [Direction::Input]
    /// or [Direction::Output].
    pub direction: Direction,
    /// Attributes attached to this port definition
    pub attributes: Attributes,
}

impl<I> From<(I, u64, Direction)> for PortDef<Width>
where
    I: Into<Id>,
{
    fn from(port: (I, u64, Direction)) -> Self {
        PortDef {
            name: port.0.into(),
            width: Width::Const { value: port.1 },
            direction: port.2,
            attributes: Attributes::default(),
        }
    }
}

impl<I> From<(I, u64, Direction)> for PortDef<u64>
where
    I: Into<Id>,
{
    fn from(port: (I, u64, Direction)) -> Self {
        PortDef {
            name: port.0.into(),
            width: port.1,
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

impl std::fmt::Display for Width {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Width::Const { value } => write!(f, "{}", value),
            Width::Param { value } => write!(f, "{}", value),
        }
    }
}

impl PortDef<Width> {
    /// Given a map from names of parameters to their values, attempt to
    /// resolve this definition.
    /// Errors if there is no binding for a required parameter binding.
    pub fn resolve(
        self,
        binding: &LinkedHashMap<Id, u64>,
    ) -> CalyxResult<PortDef<u64>> {
        match &self.width {
            Width::Const { value } => Ok(PortDef {
                name: self.name,
                width: *value,
                attributes: self.attributes,
                direction: self.direction,
            }),
            Width::Param { value } => match binding.get(value) {
                Some(width) => Ok(PortDef {
                    name: self.name,
                    width: *width,
                    attributes: self.attributes,
                    direction: self.direction,
                }),
                None => {
                    let param_name = &self.name;
                    let msg = format!("Failed to resolve: {param_name}");
                    Err(Error::malformed_structure(msg))
                }
            },
        }
    }
}
