use std::{error, fmt};

pub use interp::WidthInt;

#[derive(Debug)]
pub enum ValueConversionError {
    WidthTooLarge(interp::WidthInt),
}

impl fmt::Display for ValueConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueConversionError::WidthTooLarge(width) => {
                write!(
                    f,
                    "Failed to convert bitvector of width `{}` into `u64`",
                    width
                )
            }
        }
    }
}

impl error::Error for ValueConversionError {}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct Value<const N: interp::WidthInt> {
    pub inner: interp::BitVecValue,
}

impl<const N: interp::WidthInt> From<u64> for Value<N> {
    fn from(value: u64) -> Self {
        Self {
            inner: interp::BitVecValue::from_u64(value, N),
        }
    }
}

impl<const N: interp::WidthInt> TryInto<u64> for &Value<N> {
    type Error = ValueConversionError;

    fn try_into(self) -> Result<u64, Self::Error> {
        use interp::BitVecOps;
        self.inner
            .to_u64()
            .ok_or(Self::Error::WidthTooLarge(self.inner.width()))
    }
}

impl<const N: interp::WidthInt> interp::BitVecOps for Value<N> {
    fn width(&self) -> interp::WidthInt {
        N
    }

    fn words(&self) -> &[interp::Word] {
        self.inner.words()
    }
}
