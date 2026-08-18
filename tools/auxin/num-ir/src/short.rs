//! read / write compact types
//!
//! compact types are of form:
//! - ints: ``[u/i]<SIZE>``
//! - floats: ``f<SIZE>``
//! - fixed: ``d[u/i]<SIZE>:<EXP_MAG>``
//!
//! 'd' is used as the prefix to simplify parsing

use std::num::ParseIntError;

use crate::typing::*;

#[derive(Debug, thiserror::Error)]
pub enum ShortTypeErr {
    #[error("invalid floating-point: not f32 / f64")]
    FloatSpec,
    #[error("cannot represent fixed with exp {1} in width {0}")]
    FixedSpec(usize, i32),
    #[error("cannot parse {0}")]
    Parsing(String),
    #[error("parseint: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("tried to parse type with width 0")]
    WidthZero,
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
        let (width, signed, class) = if first_c == 'f' {
            let float_w =
                destruct_float_width(rem).ok_or(ShortTypeErr::FloatSpec)?;
            (float_w, false, TypeClass::Float)
        } else if first_c == 'd'
            && let Some((p, n)) = rem.split_once(':')
            && let Some((width, signed)) = destruct_signed(p)
        {
            let exp_mag = n.parse::<i32>()?;
            if (exp_mag > 0) && (exp_mag as usize) > width {
                return Err(ShortTypeErr::FixedSpec(width, exp_mag));
            }

            (width, signed, TypeClass::Fixed { exp_mag })
        } else if first_c == 'b' {
            (rem.parse::<usize>()?, false, TypeClass::Bits)
        } else if (first_c == 'i' || first_c == 'u')
            && let Some((w, sgn)) = destruct_signed(s)
        {
            (w, sgn, TypeClass::Int)
        } else {
            return Err(ShortTypeErr::Parsing(s.to_string()));
        };

        // TODO: this type of thing is why we need guarded constructors
        if width == 0 {
            return Err(ShortTypeErr::WidthZero);
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

#[cfg(all(test, feature = "prop-utils"))]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // proptests on arbitrary 'things which might look parseable'
    proptest! {
    #[test]
    fn parse_ints(s in "[iu][1-9][0-9]{6}"){
        let t = TypeSpec::read_short_t(&s)?;
        prop_assert_eq!(t.class.clone(), TypeClass::Int);
        prop_assert_eq!(t.to_string(), s);
        let o = t.to_string();
        prop_assert_eq!(t, TypeSpec::read_short_t(&o)?)
    }

    #[test]
    fn parse_float(s in "f.*"){
        if s == "f64" || s == "f32"{
            prop_assert!(TypeSpec::read_short_t(&s).is_ok());
        } else{
             prop_assert!(TypeSpec::read_short_t(&s).is_err());
        }
    }

    #[test]
    fn parse_bits(s in "b[1-9][0-9]{6}"){
        let t= TypeSpec::read_short_t(&s)?;
        prop_assert_eq!(t.class.clone(), TypeClass::Bits);
        prop_assert_eq!(t.to_string(), s);
        let o = t.to_string();
        prop_assert_eq!(t, TypeSpec::read_short_t(&o)?)
    }

    #[test]
    fn parse_fixed(s in "d[iu]", w in 1..i32::MAX, exp in any::<i32>()){
        // typenames wider than 64 are okay. but parsing a decimal wider than f64 might not
        prop_assume!(exp <= (w as i32));
        let full = format!("{}{}:{}",s,w,exp);
        let t= TypeSpec::read_short_t(&full)?;
        prop_assert_eq!(t.class.clone(), TypeClass::Fixed{exp_mag: exp});
        prop_assert_eq!(t.to_string(), full);
        let o = t.to_string();
        prop_assert_eq!(t, TypeSpec::read_short_t(&o)?)
    }


    }

    // make sure that types roundtrip through short_t correctly
    use crate::props::*;
    proptest! {
        #[test]
        fn roundtrip(t in arb_type()){
            let s = t.to_string();
            let res = TypeSpec::read_short_t(&s).unwrap();
            compare_typeclasses(&res, &t)?;


        }
    }
}
