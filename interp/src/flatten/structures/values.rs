use crate::values::Value as BvValue;

pub enum Value {
    /// An undefined value from which it is dangerous to read.
    Undefined,
    /// A defined value that can be computed with
    Defined(DefinedValue),
}

pub enum DefinedValue {
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

    /// Returns `true` if the value is [`Defined`].
    ///
    /// [`Defined`]: Value::Defined
    #[must_use]
    pub fn is_defined(&self) -> bool {
        matches!(self, Self::Defined(..))
    }
}
