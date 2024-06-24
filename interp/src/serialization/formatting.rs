use fraction::Fraction;
use serde::Serialize;
use std::fmt::{Debug, Display};

use crate::{
    flatten::flat_ir::cell_prototype::MemoryDimensions, utils::PrintCode,
    values::Value,
};

/// An enum wrapping over a tuple representing the shape of a multi-dimensional
/// array
#[derive(Clone)]
pub enum Shape {
    D1(usize),
    D2(usize, usize),
    D3(usize, usize, usize),
    D4(usize, usize, usize, usize),
}

impl Shape {
    fn is_1d(&self) -> bool {
        matches!(self, Shape::D1(_))
    }

    pub(crate) fn dim_str(&self) -> String {
        match self {
            Shape::D1(_) => String::from("1D"),
            Shape::D2(_, _) => String::from("2D"),
            Shape::D3(_, _, _) => String::from("3D"),
            Shape::D4(_, _, _, _) => String::from("4D"),
        }
    }

    /// returns the total number of entries in the memory, i.e. it's size based
    /// on the dimensions of it.
    pub fn size(&self) -> usize {
        match self {
            Shape::D1(d0) => *d0,
            Shape::D2(d0, d1) => d0 * d1,
            Shape::D3(d0, d1, d2) => d0 * d1 * d2,
            Shape::D4(d0, d1, d2, d3) => d0 * d1 * d2 * d3,
        }
    }
}
impl From<usize> for Shape {
    fn from(u: usize) -> Self {
        Shape::D1(u)
    }
}
impl From<(usize,)> for Shape {
    fn from(u: (usize,)) -> Self {
        Shape::D1(u.0)
    }
}
impl From<(usize, usize)> for Shape {
    fn from(u: (usize, usize)) -> Self {
        Shape::D2(u.0, u.1)
    }
}

impl From<(usize, usize, usize)> for Shape {
    fn from(u: (usize, usize, usize)) -> Self {
        Shape::D3(u.0, u.1, u.2)
    }
}

impl From<(usize, usize, usize, usize)> for Shape {
    fn from(u: (usize, usize, usize, usize)) -> Self {
        Shape::D4(u.0, u.1, u.2, u.3)
    }
}

impl From<&MemoryDimensions> for Shape {
    fn from(value: &MemoryDimensions) -> Self {
        match value {
            MemoryDimensions::D1 { d0_size, .. } => {
                Shape::D1(*d0_size as usize)
            }
            MemoryDimensions::D2 {
                d0_size, d1_size, ..
            } => Shape::D2(*d0_size as usize, *d1_size as usize),
            MemoryDimensions::D3 {
                d0_size,
                d1_size,
                d2_size,
                ..
            } => Shape::D3(
                *d0_size as usize,
                *d1_size as usize,
                *d2_size as usize,
            ),
            MemoryDimensions::D4 {
                d0_size,
                d1_size,
                d2_size,
                d3_size,
                ..
            } => Shape::D4(
                *d0_size as usize,
                *d1_size as usize,
                *d2_size as usize,
                *d3_size as usize,
            ),
        }
    }
}

/// A wrapper enum used during serialization. It represents either an unsigned integer,
/// or a signed integer and is serialized as the underlying integer. This also allows
/// mixed serialization of signed and unsigned values
#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum Entry {
    U(u64),
    I(i64),
    Frac(Fraction),
    Value(Value),
}

impl From<u64> for Entry {
    fn from(u: u64) -> Self {
        Self::U(u)
    }
}

impl From<i64> for Entry {
    fn from(i: i64) -> Self {
        Self::I(i)
    }
}

impl From<Fraction> for Entry {
    fn from(f: Fraction) -> Self {
        Self::Frac(f)
    }
}

impl Entry {
    pub fn from_val_code(val: &Value, code: &PrintCode) -> Self {
        match code {
            PrintCode::Unsigned => val.as_u64().into(),
            PrintCode::Signed => val.as_i64().into(),
            PrintCode::UFixed(f) => val.as_ufp(*f).into(),
            PrintCode::SFixed(f) => val.as_sfp(*f).into(),
            PrintCode::Binary => Entry::Value(val.clone()),
        }
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Entry::U(v) => write!(f, "{}", v),
            Entry::I(v) => write!(f, "{}", v),
            Entry::Frac(v) => write!(f, "{}", v),
            Entry::Value(v) => write!(f, "{}", v),
        }
    }
}

impl Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
