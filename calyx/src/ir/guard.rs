use super::{RRC, Port};

/// A guard which has pointers to the various ports from which it reads.
pub struct Guard {
    // TODO
    val: RRC<Port>,
}

