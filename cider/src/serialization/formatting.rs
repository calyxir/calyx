use fraction::Fraction;
use itertools::Itertools;
use serde::Serialize;
use std::fmt::{Debug, Display};

use crate::flatten::{
    flat_ir::cell_prototype::MemoryDimensions, text_utils::Color,
};
use baa::{BitVecOps, BitVecValue, WidthInt};

/// An enum wrapping over a tuple representing the shape of a multi-dimensional
/// array
#[derive(Clone, Debug)]
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

    pub fn as_string(&self) -> String {
        match self {
            Shape::D1(d0) => format!("({})", d0),
            Shape::D2(d0, d1) => format!("({}, {})", d0, d1),
            Shape::D3(d0, d1, d2) => format!("({}, {}, {})", d0, d1, d2),
            Shape::D4(d0, d1, d2, d3) => {
                format!("({}, {}, {}, {})", d0, d1, d2, d3)
            }
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

/// A wrapper enum used during serialization.
///
/// It represents either an unsigned integer, or a signed integer and is
/// serialized as the underlying integer. This also allows mixed serialization
/// of signed and unsigned values
#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum Entry {
    U(u64),
    I(i64),
    Frac(Fraction),
    Value(BitVecValue),
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
    pub fn from_val_code(val: &BitVecValue, code: &PrintCode) -> Self {
        match code {
            PrintCode::Unsigned => val.to_u64().unwrap().into(),
            PrintCode::Signed => val.to_i64().unwrap().into(),
            PrintCode::UFixed(f) => {
                val.to_unsigned_fixed_point(*f).unwrap().into()
            }
            PrintCode::SFixed(f) => {
                val.to_signed_fixed_point(*f).unwrap().into()
            }
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
            Entry::Value(v) => write!(f, "{}", v.to_bit_str()),
        }
    }
}

impl Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PrintCode {
    Binary,
    Unsigned,
    Signed,
    UFixed(WidthInt),
    SFixed(WidthInt),
}

impl Default for PrintCode {
    fn default() -> Self {
        Self::Unsigned
    }
}

impl Display for PrintCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PrintCode::Binary => "\\b".stylize_print_code().to_string(),
                PrintCode::Unsigned => "\\u".stylize_print_code().to_string(),
                PrintCode::Signed => "\\s".stylize_print_code().to_string(),
                PrintCode::UFixed(n) =>
                    format!("\\u.{}", n).stylize_print_code().to_string(),
                PrintCode::SFixed(n) =>
                    format!("\\s.{}", n).stylize_print_code().to_string(),
            }
        )
    }
}

#[derive(Clone)]
pub enum Serializable {
    Empty,
    Val(Entry),
    Array(Vec<Entry>, Shape),
}

impl Serializable {
    pub fn has_state(&self) -> bool {
        !matches!(self, Serializable::Empty)
    }
}

impl Display for Serializable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Serializable::Empty => write!(f, ""),
            Serializable::Val(v) => write!(f, "{}", v),
            Serializable::Array(arr, shape) => {
                write!(f, "{}", format_array(arr, shape))
            }
        }
    }
}

impl Serialize for Serializable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Serializable::Empty => serializer.serialize_unit(),
            Serializable::Val(u) => u.serialize(serializer),
            Serializable::Array(arr, shape) => {
                let arr: Vec<&Entry> = arr.iter().collect();
                if shape.is_1d() {
                    return arr.serialize(serializer);
                }
                // there's probably a better way to write this
                match shape {
                    Shape::D2(_d0, d1) => {
                        let mem = arr
                            .iter()
                            .chunks(*d1)
                            .into_iter()
                            .map(|x| x.into_iter().collect::<Vec<_>>())
                            .collect::<Vec<_>>();
                        mem.serialize(serializer)
                    }
                    Shape::D3(_d0, d1, d2) => {
                        let mem = arr
                            .iter()
                            .chunks(d1 * d2)
                            .into_iter()
                            .map(|x| {
                                x.into_iter()
                                    .chunks(*d2)
                                    .into_iter()
                                    .map(|y| y.into_iter().collect::<Vec<_>>())
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>();
                        mem.serialize(serializer)
                    }
                    Shape::D4(_d0, d1, d2, d3) => {
                        let mem = arr
                            .iter()
                            .chunks(d2 * d1 * d3)
                            .into_iter()
                            .map(|x| {
                                x.into_iter()
                                    .chunks(d2 * d3)
                                    .into_iter()
                                    .map(|y| {
                                        y.into_iter()
                                            .chunks(*d3)
                                            .into_iter()
                                            .map(|z| {
                                                z.into_iter()
                                                    .collect::<Vec<_>>()
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>();
                        mem.serialize(serializer)
                    }
                    Shape::D1(_) => unreachable!(),
                }
            }
        }
    }
}

fn format_array(arr: &[Entry], shape: &Shape) -> String {
    match shape {
        Shape::D2(_d0, d1) => {
            let mem = arr
                .iter()
                .chunks(*d1)
                .into_iter()
                .map(|x| x.into_iter().collect::<Vec<_>>())
                .collect::<Vec<_>>();
            format!("{:?}", mem)
        }
        Shape::D3(_d0, d1, d2) => {
            let mem = arr
                .iter()
                .chunks(d1 * d2)
                .into_iter()
                .map(|x| {
                    x.into_iter()
                        .chunks(*d2)
                        .into_iter()
                        .map(|y| y.into_iter().collect::<Vec<_>>())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            format!("{:?}", mem)
        }
        Shape::D4(_d0, d1, d2, d3) => {
            let mem = arr
                .iter()
                .chunks(d2 * d1 * d3)
                .into_iter()
                .map(|x| {
                    x.into_iter()
                        .chunks(d2 * d3)
                        .into_iter()
                        .map(|y| {
                            y.into_iter()
                                .chunks(*d3)
                                .into_iter()
                                .map(|z| z.into_iter().collect::<Vec<_>>())
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            format!("{:?}", mem)
        }
        Shape::D1(_) => {
            format!("{:?}", arr)
        }
    }
}
