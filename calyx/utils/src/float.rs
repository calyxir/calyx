//! Implement parsing and generation for floating point constants used by std_float_const.

use crate::{CalyxResult, Error};

pub fn parse(rep: u64, width: u64, fl: String) -> CalyxResult<u64> {
    if rep != 0 {
        return Err(Error::misc(format!(
            "Unknown representation: {rep}. Support representations: 0 (IEEE754)"
        )));
    }

    let bits: u64 = match width {
        32 => {
            let fl = fl.parse::<f32>().map_err(|e| {
                Error::misc(format!(
                    "Expected valid floating point number: {e}"
                ))
            })?;
            fl.to_bits() as u64
        }
        64 => {
            let fl = fl.parse::<f64>().map_err(|e| {
                Error::misc(format!(
                    "Expected valid floating point number: {e}"
                ))
            })?;
            fl.to_bits()
        }
        r => {
            return Err(Error::misc(format!(
                "Unsupported floating point width: {r}. Supported values: 32, 64"
            )));
        }
    };

    Ok(bits)
}

pub fn emit(bits: u64, width: u64) -> CalyxResult<String> {
    match width {
        32 => {
            let fl = f32::from_bits(bits as u32);
            Ok(format!("{fl}"))
        }
        64 => {
            let fl = f64::from_bits(bits);
            Ok(format!("{fl}"))
        }
        r => Err(Error::misc(format!(
            "Unsupported floating point width: {r}. Supported values: 32, 64"
        ))),
    }
}
