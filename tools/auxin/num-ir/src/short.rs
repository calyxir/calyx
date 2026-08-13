use std::num::ParseIntError;

use crate::typing::*;

// read / write compact types

// compact types are of form:
// ints: [u/i]<SIZE>
// floats: f<SIZE>
// fixed: d[u/i]<SIZE>:<EXP_MAG>

// 'd' is used as the prefix to make parsing easier

// TODO: some number of these are probably redundant at this point
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

#[inline]
fn strip_first_char(s: &str) -> Option<(char, &str)> {
    let mut c_iter = s.char_indices();
    if let Some((_, first_char)) = c_iter.next()
        && let Some((rem_start, _)) = c_iter.next()
    {
        return Some((first_char, &s[rem_start..]));
    }

    None
}

fn destruct_signed(s: &str) -> Option<(usize, bool)> {
    if let Some((first_c, rem)) = strip_first_char(s)
        && (first_c == 'i' || first_c == 'u')
    {
        let width = rem.parse::<usize>().ok()?;
        return Some((width, first_c == 'i'));
    }
    None
}

fn destruct_float_width(s: &str) -> Option<usize> {
    if s == "32" {
        Some(32)
    } else if s == "64" {
        Some(64)
    } else {
        None
    }
}

impl TryFromShort for TypeSpec {
    /// requires that the string is already trimmed.
    fn read_short_t(s: &str) -> Result<Self, ShortTypeErr> {
        let Some((first_c, rem)) = strip_first_char(s) else {
            return Err(ShortTypeErr::Parsing(s.to_string()));
        };
        let (width, signed, class) = match first_c {
            'f' if let Some(w) = destruct_float_width(s) => {
                (w, false, TypeClass::Float)
            }
            'd' if let Some((p, n)) = rem.split_once(':')
                && let Some((width, signed)) = destruct_signed(p) =>
            {
                let exp_mag = n.parse::<i32>()?;
                if exp_mag > (width as i32) {
                    return Err(ShortTypeErr::Parsing(s.to_string()));
                }

                (width, signed, TypeClass::Fixed { exp_mag })
            }
            'b' => (rem.parse::<usize>()?, false, TypeClass::Bits),
            'i' | 'u' if let Some((w, sgn)) = destruct_signed(s) => {
                (w, sgn, TypeClass::Int)
            }
            _ => return Err(ShortTypeErr::Parsing(s.to_string())),
        };

        // TODO: this type of thing is why we need guarded constructors
        if width == 0 {
            return Err(ShortTypeErr::Parsing(s.to_string()));
        }
        Ok(TypeSpec {
            width,
            signed,
            class,
        })
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
                    if self.signed { 'i' } else { 'u' },
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
                    if self.signed { 'i' } else { 'u' },
                    self.width,
                    e
                )
            }
            _ => panic!("can't shorten an unknown typeclass"),
        }
    }
}
