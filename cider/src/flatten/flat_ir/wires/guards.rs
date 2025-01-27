use calyx_ir::PortComp;
use cider_idx::maps::IndexedMap;

use crate::flatten::flat_ir::prelude::{GuardIdx, PortRef};

/// A map storing all the guards defined in the program
pub type GuardMap = IndexedMap<GuardIdx, Guard>;

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
