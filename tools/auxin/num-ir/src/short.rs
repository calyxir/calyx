use std::num::ParseIntError;

use crate::typing::*;

// read / write compact types

// compact types are of form:
// ints: [u/i]<SIZE>
// floats: f<SIZE>
// fixed: d[u/i]<SIZE>:<EXP_MAG>

// 'd' is used as the prefix to make parsing easier

#[derive(Debug, thiserror::Error)]
pub enum ShortTypeErr {
    #[error("cannot represent float with size {0}")]
    FloatSpec(usize),
    #[error("cannot represent fixed with exp {1} in width {0}")]
    FixedSpec(usize, i32),
    #[error("cannot parse {0}")]
    Parsing(String),
    #[error("parseint: {0}")]
    ParseInt(#[from] ParseIntError),
}

pub trait TryFromShort
where
    Self: Sized,
{
    fn read_short_t(s: &str) -> Result<Self, ShortTypeErr>;
}

pub trait ToShort {
    fn as_short_t(t: &Self) -> String;
}

fn destruct_signed(s: &str) -> Result<(usize, bool), ShortTypeErr> {
    if let Some(rem_s) = s.strip_prefix("i") {
        let width = rem_s.parse::<usize>()?;
        return Ok((width, true));
    } else if let Some(rem_u) = s.strip_prefix("u") {
        let width = rem_u.parse::<usize>()?;
        return Ok((width, false));
    }
    Err(ShortTypeErr::Parsing(format!("can't parse {}", s)))
}

// TODO: definitely could be made more optimal
impl TryFromShort for TypeSpec {
    /// requires that the string is already trimmed.
    fn read_short_t(s: &str) -> Result<Self, ShortTypeErr> {
        if let Some(rem_f) = s.strip_prefix("f") {
            let width = rem_f.parse::<usize>()?;
            if width == 32 || width == 64 {
                return Ok(TypeSpec {
                    width,
                    signed: false,
                    class: TypeClass::Float,
                });
            }
            Err(ShortTypeErr::FloatSpec(width))
        } else if let Some(rem_d) = s.strip_prefix("d") {
            let Some((p, n)) = rem_d.split_once(":") else {
                return Err(ShortTypeErr::Parsing(rem_d.to_string()));
            };
            let (width, signed) = destruct_signed(p)?;
            let exp_mag = n.parse::<i32>()?;
            Ok(TypeSpec {
                width,
                signed,
                class: TypeClass::Fixed { exp_mag },
            })
        } else {
            let (width, signed) = destruct_signed(s)?;
            Ok(TypeSpec {
                width,
                signed,
                class: TypeClass::Int,
            })
        }
    }
}

impl ToShort for TypeSpec {
    fn as_short_t(t: &Self) -> String {
        match t.class {
            TypeClass::Bits => format!("b{}", t.width),
            TypeClass::Int => {
                format!("{}{}", if t.signed { "i" } else { "u" }, t.width)
            }
            TypeClass::Float => {
                format!("f{}", t.width)
            }
            TypeClass::Fixed { exp_mag: e } => {
                format!(
                    "d{}{}:{}",
                    if t.signed { "i" } else { "u" },
                    t.width,
                    e
                )
            }
            _ => panic!("can't shorten an unknown typeclass"),
        }
    }
}

impl std::fmt::Display for TypeSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.class {
            TypeClass::Bits => write!(f, "b{}", self.width),
            TypeClass::Int => {
                write!(
                    f,
                    "{}{}",
                    if self.signed { "i" } else { "u" },
                    self.width
                )
            }
            TypeClass::Float => {
                write!(f, "f{}", self.width)
            }
            TypeClass::Fixed { exp_mag: e } => {
                write!(
                    f,
                    "d{}{}:{}",
                    if self.signed { "i" } else { "u" },
                    self.width,
                    e
                )
            }
            _ => panic!("can't shorten an unknown typeclass"),
        }
    }
}
