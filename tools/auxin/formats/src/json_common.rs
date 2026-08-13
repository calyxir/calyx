use num_ir::typing::{TypeClass, TypeSpec};
use serde::{self, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum JsonTypes {
    Bitnum,
    #[serde(alias = "fixed_point")]
    Fixed,
    #[serde(alias = "ieee754_float")]
    IEEE754Float,
}

#[derive(Debug, thiserror::Error)]
pub enum JsonTypeError {
    #[error("can't normalise {0} as fixed-point")]
    FixedErr(String),

    #[error("bad class {0:?}")]
    ClassErr(TypeClass),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormatInfo {
    pub numeric_type: JsonTypes,
    pub is_signed: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub int_width: Option<u32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frac_width: Option<u32>,
}

/*
    ideally would also check for overspecified format (i.e. non-fixed_point with frac_width defined)
*/
impl FormatInfo {
    // returns fixed-point as (overall width, frac_width)
    // a bit verbose, but roughly self-documenting
    #[inline]
    fn normalise_fixed(&self) -> Result<(usize, i32), JsonTypeError> {
        if let Some(w) = self.width {
            match (self.int_width, self.frac_width) {
                (Some(i), Some(f)) if i + f == w => Ok((w as usize, f as i32)),
                (None, Some(f)) if f < w => Ok((w as usize, f as i32)),
                (Some(i), None) if i < w => {
                    Ok((w as usize, (w as i32 - (i as i32))))
                }
                _ => Err(JsonTypeError::FixedErr(format!("{self:?}"))),
            }
        } else {
            match (self.int_width, self.frac_width) {
                (Some(i), Some(f)) => Ok(((i + f) as usize, f as i32)),
                _ => Err(JsonTypeError::FixedErr(format!("{self:?}"))),
            }
        }
    }
}

impl TryFrom<&TypeSpec> for FormatInfo {
    type Error = JsonTypeError;

    fn try_from(value: &TypeSpec) -> Result<Self, Self::Error> {
        use TypeClass as tc;
        let mut frac_width = None;
        let numeric_type = match value.class {
            tc::Bits => JsonTypes::Bitnum,
            tc::Int => JsonTypes::Bitnum,
            tc::Float => JsonTypes::IEEE754Float,
            tc::Fixed { exp_mag: e } => {
                frac_width = Some(if e < 0 { 0 } else { e } as u32);
                JsonTypes::Fixed
            }
            _ => {
                return Err(JsonTypeError::ClassErr(value.class.clone()));
            }
        };
        Ok(FormatInfo {
            numeric_type,
            is_signed: value.signed,
            width: Some(value.width as u32),
            int_width: None,
            frac_width,
        })
    }
}

impl TryFrom<&FormatInfo> for TypeSpec {
    type Error = JsonTypeError;
    fn try_from(value: &FormatInfo) -> Result<Self, Self::Error> {
        use TypeClass as tc;

        use JsonTypes::*;
        let mut total_width = value.width;
        let class = match value.numeric_type {
            Bitnum => tc::Int,
            IEEE754Float => tc::Float,
            Fixed => {
                let (t, frac_width) = value.normalise_fixed()?;
                total_width = Some(t as u32);
                tc::Fixed {
                    exp_mag: frac_width,
                }
            }
        };
        Ok(TypeSpec {
            width: total_width.unwrap() as usize,
            signed: value.is_signed,
            class,
        })
    }
}
