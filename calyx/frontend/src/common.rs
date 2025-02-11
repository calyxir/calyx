use super::Attributes;
use crate::Attribute;
use calyx_utils::{CalyxResult, Error, GetName, Id};
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
    /// (Optional) latency; for static primitives
    pub latency: Option<std::num::NonZeroU64>,
    /// body of the string, if it is an inlined primitive
    pub body: Option<String>,
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
               "primitive `{}` requires {} parameters but instantiation provides {} parameters",
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
    pub fn find_all_with_attr<A>(
        &self,
        attr: A,
    ) -> impl Iterator<Item = &PortDef<Width>>
    where
        A: Into<Attribute> + Copy,
    {
        self.signature
            .iter()
            .filter(move |&g| g.attributes.has(attr))
    }
}

impl GetName for Primitive {
    fn name(&self) -> Id {
        self.name
    }
}

/// Definition of a port parameterized by a width type.
/// Ports on Primitives can be parameteris and use [Width].
/// Ports on Components cannot be parameterized and therefore use `u64`.
#[derive(Clone, Debug)]
pub struct PortDef<W> {
    /// The name of the port.
    name: Id,
    /// The width of the port. .
    pub width: W,
    /// The direction of the port. Only allowed to be [Direction::Input]
    /// or [Direction::Output].
    pub direction: Direction,
    /// Attributes attached to this port definition
    pub attributes: Attributes,
}

impl<W> PortDef<W> {
    pub fn new(
        name: impl Into<Id>,
        width: W,
        direction: Direction,
        attributes: Attributes,
    ) -> Self {
        assert!(
            matches!(direction, Direction::Input | Direction::Output),
            "Direction must be either Input or Output"
        );

        Self {
            name: name.into(),
            width,
            direction,
            attributes,
        }
    }

    /// Return the name of the port definition
    pub fn name(&self) -> Id {
        self.name
    }
}

/// Represents an abstract width of a primitive signature.
#[derive(Clone, Debug, PartialEq)]
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

impl From<u64> for Width {
    fn from(value: u64) -> Self {
        Width::Const { value }
    }
}

impl From<Id> for Width {
    fn from(value: Id) -> Self {
        Width::Param { value }
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

/// Direction of a port on a cell.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub enum Direction {
    /// Input port.
    Input,
    /// Output port.
    Output,
    /// Input-Output "port". Should only be used by holes.
    Inout,
}

impl Direction {
    /// Return the direction opposite to the current direction
    pub fn reverse(&self) -> Self {
        match self {
            Direction::Input => Direction::Output,
            Direction::Output => Direction::Input,
            Direction::Inout => Direction::Inout,
        }
    }
}
