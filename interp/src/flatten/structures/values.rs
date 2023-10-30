use crate::values::Value as BvValue;

pub enum Value {
    /// An undefined value from which it is dangerous to read.
    Undefined,

    // insert small value here
    /// An arbitrarily large value. Should be replaced with a pointer to keep
    /// the size manageable
    Large(BvValue),
}

impl Value {
    /// Returns `true` if the value is [`Undefined`].
    ///
    /// [`Undefined`]: Value::Undefined
    #[must_use]
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }
}
