use serde::{self, Deserialize, Serialize, Serializer};
use serde_json::{Number, Value};
use std::collections::BTreeMap;
use std::{collections::HashMap, num::ParseFloatError};
use thiserror::Error;

use super::repr;

#[derive(Debug, Error)]
pub enum JsonParseError {
    #[error("Could not parse number as integer: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Could not parse number as float: {0}")]
    ParseFloat(#[from] ParseFloatError),
    #[error("bad dimension")]
    DimError,
    #[error("Non numerical value {0}")]
    NonNumError(String),
    #[error("Malformed fixed-point def")]
    MalformedFixed,
    #[error("No width / equivalent!")]
    NoWidth,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JsonTypes {
    Untyped,
    Bitnum,
    #[serde(alias = "fixed_point")]
    Fixed,
    #[serde(alias = "ieee754_float")]
    IEEE754Float,
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
    fn normalise_fixed(&self) -> Result<(u32, u32), JsonParseError> {
        if let Some(w) = self.width {
            if w > 64 {
                return Err(JsonParseError::MalformedFixed);
            }
            match (self.int_width, self.frac_width) {
                (Some(i), Some(f)) if i + f == w => Ok((w, f)),
                (None, Some(f)) if f < w => Ok((w, f)),
                (Some(i), None) if i < w => Ok((w, w - i)),
                _ => Err(JsonParseError::MalformedFixed),
            }
        } else {
            match (self.int_width, self.frac_width) {
                (Some(i), Some(f)) if i + f <= 64 => Ok((i + f, f)),
                _ => Err(JsonParseError::MalformedFixed),
            }
        }
    }
}

// NOTE: width stuff gets kinda dropped here. probably best to defer checking.

// operates on reference b/c the FormatInfo is used to fill fields in DataSet and DataType
impl TryFrom<&FormatInfo> for repr::DataType {
    type Error = JsonParseError;

    fn try_from(value: &FormatInfo) -> Result<Self, Self::Error> {
        match value.numeric_type {
            JsonTypes::Untyped => Ok(repr::DataType::Untyped),
            JsonTypes::Bitnum => Ok(repr::DataType::Int {
                is_signed: value.is_signed,
            }),
            JsonTypes::Fixed => Ok(repr::DataType::Fixed {
                frac_width: value.normalise_fixed()?.1,
                is_signed: value.is_signed,
            }),
            JsonTypes::IEEE754Float => Ok(repr::DataType::Float),
        }
    }
}

/*
    NOTE: as implemented, some fixed point numbers which exceed the precision of f64 but are still valid fixed-point may not work.

    ``Value`` is maybe not the ideal way to do this, but is good enough
*/

// handles unquoted+ quoted numbers when they probably shouldn't be?

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonDataEntry {
    pub data: Value,
    pub format: FormatInfo,
}

// using a hashmap here means that the serialization is non-deterministic but
// for now that's probably fine
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonData(pub HashMap<String, JsonDataEntry>);

struct JsonDataDestructor {
    pub dimensions: HashMap<u32, usize>, // maps dimension : dimension size
    pub status: Option<JsonParseError>,
}

/*
    destruct an n-dimensional array. Very Bad, no good
    probably an excess of allocations. if only there were some easy way to do this...
*/
fn destructure_helper(
    destr: &mut JsonDataDestructor,
    v: &Value,
    level: u32,
) -> Vec<Value> {
    use crate::json::JsonParseError::*;

    let Value::Array(arr) = v else {
        destr.status = Some(NonNumError(v.to_string()));
        return Vec::new();
    };
    destr
        .dimensions
        .entry(level)
        .and_modify(|e| {
            if arr.len() != *e {
                destr.status = Some(JsonParseError::DimError)
            }
        })
        .or_insert(arr.len());
    match arr.first().unwrap() {
        Value::Number(_) => arr.clone(),
        Value::Array(_) => arr
            .iter()
            .flat_map(|v: &Value| destructure_helper(destr, v, level + 1))
            .collect(),
        _ => {
            destr.status = Some(NonNumError(arr.first().unwrap().to_string()));
            return Vec::new();
        }
    }
}

// TODO: needs to return dimensions
fn destructure(v: &Value) -> Result<Vec<Value>, JsonParseError> {
    let mut destr = JsonDataDestructor {
        dimensions: HashMap::new(),
        status: None,
    };
    let res = destructure_helper(&mut destr, v, 0);
    match destr.status {
        Some(e) => Err(e),
        None => Ok(res),
    }
}

fn decode_val(
    v: &Value,
    t: &repr::DataType,
) -> Result<repr::Untypednum, JsonParseError> {
    todo!()
}

impl TryFrom<JsonDataEntry> for repr::DataSet {
    type Error = JsonParseError;
    fn try_from(value: JsonDataEntry) -> Result<Self, Self::Error> {
        let unconv_data = destructure(&value.data)?;
        let dtype: repr::DataType = (&value.format).try_into()?;

        let conv_data = unconv_data
            .iter()
            .map(|v| decode_val(v, &dtype))
            .collect()?; // TODO
        Ok(repr::DataSet {
            width: value.format.width,
            data: conv_data,
            dimensions: todo!(),
            dtype,
            end: todo!(),
        })
    }
}

/// For use with serde's [serialize_with] attribute
/// see: https://stackoverflow.com/questions/42723065/how-to-sort-hashmap-keys-when-serializing-with-serde
fn ordered_map<S, K: Ord + Serialize, V: Serialize>(
    value: &HashMap<K, V>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_data() {
        let data = r#"
{
  "in": {
    "data": [
    [
        [4.0],
        [5.0]
    ],
    [
        [3.0],
        [1.0]
    ]

    ],
    
    "format": {
      "numeric_type": "bitnum",
      "is_signed": false,
      "width": 32
    }
  },
  "out": {
    "data": [
      6
    ],
    "format": {
      "numeric_type": "bitnum",
      "is_signed": false,
      "width": 32
    }
  }
}"#;

        let json_data: JsonData = serde_json::from_str(data).unwrap();
        println!("{json_data:?}");
        let nested_data = &json_data.0.get("in").unwrap().data;
        println!("dest: {:?}", destructure(nested_data));
        println!("{}", serde_json::to_string_pretty(&json_data).unwrap());
    }
}
