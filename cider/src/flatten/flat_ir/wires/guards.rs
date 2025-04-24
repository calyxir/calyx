use calyx_ir as cir;
use cider_idx::maps::IndexedMap;

use crate::flatten::flat_ir::prelude::{GuardIdx, PortRef};

/// A map storing all the guards defined in the program
pub type GuardMap = IndexedMap<GuardIdx, Guard>;

/// The comparison operator between two ports. This re-implementation exists
/// solely to derive Hash on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortComp {
    Eq,
    Neq,
    Gt,
    Lt,
    Geq,
    Leq,
}

impl PortComp {
    pub fn to_str(&self) -> &str {
        match self {
            PortComp::Eq => "==",
            PortComp::Neq => "!=",
            PortComp::Gt => ">",
            PortComp::Lt => "<",
            PortComp::Geq => ">=",
            PortComp::Leq => "<=",
        }
    }
}

impl From<&cir::PortComp> for PortComp {
    fn from(value: &cir::PortComp) -> Self {
        match value {
            calyx_ir::PortComp::Eq => Self::Eq,
            calyx_ir::PortComp::Neq => Self::Neq,
            calyx_ir::PortComp::Gt => Self::Gt,
            calyx_ir::PortComp::Lt => Self::Lt,
            calyx_ir::PortComp::Geq => Self::Geq,
            calyx_ir::PortComp::Leq => Self::Leq,
        }
    }
}

/// A boolean expression that determines whether an assignment fires. Analogue
/// of [calyx_ir::Guard]
#[derive(Debug)]
pub enum Guard {
    /// A guard that always fires. This is the default value for a guard and
    /// applies when a guard is not explicitly defined.
    True,
    /// A disjunction of two guards
    Or(GuardIdx, GuardIdx),
    /// A conjunction of two guards
    And(GuardIdx, GuardIdx),
    /// A negation of a guard
    Not(GuardIdx),
    /// A guard that applies a comparison operator to two ports
    Comp(PortComp, PortRef, PortRef),
    /// A guard that evaluates a given port as a boolean. In such cases, the
    /// port must be a single bit.
    Port(PortRef),
}
