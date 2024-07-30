use std::{collections::HashMap, num::ParseFloatError, str::FromStr};

use interp::serialization::Dimensions;
use num_bigint::{BigInt, ParseBigIntError};
use serde::{self, Deserialize, Serialize};
use serde_json::Number;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum NumericType {
    Bitnum,
    #[serde(alias = "fixed_point")]
    Fixed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormatInfo {
    pub numeric_type: NumericType,
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

impl FormatInfo {
    pub fn get_width(&self) -> u32 {
        if let Some(w) = self.width {
            w
        } else if self.int_width.is_some() && self.frac_width.is_some() {
            self.int_width.unwrap() + self.frac_width.unwrap()
        } else {
            panic!("Either width or int_width and frac_width must be set")
        }
    }

    pub fn is_fixedpt(&self) -> bool {
        self.int_width.is_some() && self.frac_width.is_some()
            || self.width.is_some() && self.frac_width.is_some()
            || self.width.is_some() && self.int_width.is_some()
    }

    pub fn int_width(&self) -> Option<u32> {
        if self.int_width.is_some() {
            self.int_width
        } else if self.width.is_some() && self.frac_width.is_some() {
            Some(self.width.unwrap() - self.frac_width.unwrap())
        } else {
            None
        }
    }

    pub fn frac_width(&self) -> Option<u32> {
        if self.frac_width.is_some() {
            self.frac_width
        } else if self.int_width.is_some() && self.width.is_some() {
            Some(self.width.unwrap() - self.int_width.unwrap())
        } else {
            None
        }
    }

    pub fn as_data_dump_format(&self) -> interp::serialization::FormatInfo {
        match &self.numeric_type {
            NumericType::Bitnum => interp::serialization::FormatInfo::Bitnum {
                signed: self.is_signed,
                width: self.width.unwrap(),
            },
            NumericType::Fixed => {
                let (int_width, frac_width) = if self.int_width.is_some()
                    && self.frac_width.is_some()
                {
                    (self.int_width.unwrap(), self.frac_width.unwrap())
                } else if self.width.is_some() && self.frac_width.is_some() {
                    (
                        self.width.unwrap() - self.frac_width.unwrap(),
                        self.frac_width.unwrap(),
                    )
                } else if self.width.is_some() && self.int_width.is_some() {
                    (
                        self.int_width.unwrap(),
                        self.width.unwrap() - self.int_width.unwrap(),
                    )
                } else {
                    panic!(
                        "Either width or int_width and frac_width must be set"
                    )
                };

                interp::serialization::FormatInfo::Fixed {
                    signed: self.is_signed,
                    int_width,
                    frac_width,
                }
            }
        }
    }
}

pub enum ParsedNumber {
    Int(BigInt),
    Float(f64),
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Could not parse number as integer: {0}")]
    ParseInt(#[from] ParseBigIntError),
    #[error("Could not parse number as float: {0}")]
    ParseFloat(#[from] ParseFloatError),
}

#[derive(Debug, Clone)]
pub struct QuoteWrappedNumber(Number);

impl Serialize for QuoteWrappedNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.0.is_f64() {
            serializer.serialize_str(&self.0.to_string())
        } else {
            self.0.serialize(serializer)
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PrintVec {
    D1(Vec<QuoteWrappedNumber>),
    D2(Vec<Vec<QuoteWrappedNumber>>),
    D3(Vec<Vec<Vec<QuoteWrappedNumber>>>),
    D4(Vec<Vec<Vec<Vec<QuoteWrappedNumber>>>>),
}

impl From<ParseVec> for PrintVec {
    fn from(v: ParseVec) -> Self {
        match v {
            ParseVec::D1(v) => {
                PrintVec::D1(v.into_iter().map(QuoteWrappedNumber).collect())
            }
            ParseVec::D2(v) => PrintVec::D2(
                v.into_iter()
                    .map(|v| v.into_iter().map(QuoteWrappedNumber).collect())
                    .collect(),
            ),
            ParseVec::D3(v) => PrintVec::D3(
                v.into_iter()
                    .map(|v| {
                        v.into_iter()
                            .map(|v| {
                                v.into_iter().map(QuoteWrappedNumber).collect()
                            })
                            .collect()
                    })
                    .collect(),
            ),
            ParseVec::D4(v) => PrintVec::D4(
                v.into_iter()
                    .map(|v| {
                        v.into_iter()
                            .map(|v| {
                                v.into_iter()
                                    .map(|v| {
                                        v.into_iter()
                                            .map(QuoteWrappedNumber)
                                            .collect()
                                    })
                                    .collect()
                            })
                            .collect()
                    })
                    .collect(),
            ),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ParseVec {
    D1(Vec<Number>),
    D2(Vec<Vec<Number>>),
    D3(Vec<Vec<Vec<Number>>>),
    D4(Vec<Vec<Vec<Vec<Number>>>>),
}

impl From<Vec<Vec<Vec<Vec<Number>>>>> for ParseVec {
    fn from(v: Vec<Vec<Vec<Vec<Number>>>>) -> Self {
        Self::D4(v)
    }
}

impl From<Vec<Vec<Vec<Number>>>> for ParseVec {
    fn from(v: Vec<Vec<Vec<Number>>>) -> Self {
        Self::D3(v)
    }
}

impl From<Vec<Vec<Number>>> for ParseVec {
    fn from(v: Vec<Vec<Number>>) -> Self {
        Self::D2(v)
    }
}

impl From<Vec<Number>> for ParseVec {
    fn from(v: Vec<Number>) -> Self {
        Self::D1(v)
    }
}

impl ParseVec {
    fn parse_int(val: &Number) -> Result<BigInt, ParseBigIntError> {
        BigInt::from_str(&val.to_string())
    }

    fn parse_float(val: &Number) -> Result<f64, ParseFloatError> {
        f64::from_str(&val.to_string())
    }

    pub fn parse(&self, format: &FormatInfo) -> Result<DataVec, ParseError> {
        if format.is_fixedpt() {
            match self {
                ParseVec::D1(v) => {
                    let parsed: Vec<_> = v
                        .iter()
                        .map(Self::parse_float)
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(DataVec::Fd1(parsed))
                }
                ParseVec::D2(v1) => {
                    let parsed: Vec<_> = v1
                        .iter()
                        .map(|v2| {
                            v2.iter()
                                .map(Self::parse_float)
                                .collect::<Result<Vec<_>, _>>()
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(DataVec::Fd2(parsed))
                }
                ParseVec::D3(v1) => {
                    let parsed: Vec<_> = v1
                        .iter()
                        .map(|v2| {
                            v2.iter()
                                .map(|v3| {
                                    v3.iter()
                                        .map(Self::parse_float)
                                        .collect::<Result<Vec<_>, _>>()
                                })
                                .collect::<Result<Vec<_>, _>>()
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(DataVec::Fd3(parsed))
                }
                ParseVec::D4(v1) => {
                    let parsed: Vec<_> = v1
                        .iter()
                        .map(|v2| {
                            v2.iter()
                                .map(|v3| {
                                    v3.iter()
                                        .map(|v4| {
                                            v4.iter()
                                                .map(Self::parse_float)
                                                .collect::<Result<Vec<_>, _>>()
                                        })
                                        .collect::<Result<Vec<_>, _>>()
                                })
                                .collect::<Result<Vec<_>, _>>()
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(DataVec::Fd4(parsed))
                }
            }
        } else {
            match self {
                ParseVec::D1(v) => {
                    let parsed: Vec<_> = v
                        .iter()
                        .map(Self::parse_int)
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(DataVec::Id1(parsed))
                }
                ParseVec::D2(v1) => {
                    let parsed: Vec<_> = v1
                        .iter()
                        .map(|v2| {
                            v2.iter()
                                .map(Self::parse_int)
                                .collect::<Result<Vec<_>, _>>()
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(DataVec::Id2(parsed))
                }
                ParseVec::D3(v1) => {
                    let parsed: Vec<_> = v1
                        .iter()
                        .map(|v2| {
                            v2.iter()
                                .map(|v3| {
                                    v3.iter()
                                        .map(Self::parse_int)
                                        .collect::<Result<Vec<_>, _>>()
                                })
                                .collect::<Result<Vec<_>, _>>()
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(DataVec::Id3(parsed))
                }
                ParseVec::D4(v1) => {
                    let parsed: Vec<_> = v1
                        .iter()
                        .map(|v2| {
                            v2.iter()
                                .map(|v3| {
                                    v3.iter()
                                        .map(|v4| {
                                            v4.iter()
                                                .map(Self::parse_int)
                                                .collect::<Result<Vec<_>, _>>()
                                        })
                                        .collect::<Result<Vec<_>, _>>()
                                })
                                .collect::<Result<Vec<_>, _>>()
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(DataVec::Id4(parsed))
                }
            }
        }
    }
}

// this is stupid
#[derive(Debug, PartialEq)]
pub enum DataVec {
    // integers for the bitnum values
    Id1(Vec<BigInt>),
    Id2(Vec<Vec<BigInt>>),
    Id3(Vec<Vec<Vec<BigInt>>>),
    Id4(Vec<Vec<Vec<Vec<BigInt>>>>),
    // the float initialization of fixed point values
    Fd1(Vec<f64>),
    Fd2(Vec<Vec<f64>>),
    Fd3(Vec<Vec<Vec<f64>>>),
    Fd4(Vec<Vec<Vec<Vec<f64>>>>),
}

impl From<Vec<BigInt>> for DataVec {
    fn from(v: Vec<BigInt>) -> Self {
        Self::Id1(v)
    }
}

impl From<Vec<Vec<BigInt>>> for DataVec {
    fn from(v: Vec<Vec<BigInt>>) -> Self {
        Self::Id2(v)
    }
}

impl From<Vec<Vec<Vec<BigInt>>>> for DataVec {
    fn from(v: Vec<Vec<Vec<BigInt>>>) -> Self {
        Self::Id3(v)
    }
}

impl From<Vec<Vec<Vec<Vec<BigInt>>>>> for DataVec {
    fn from(v: Vec<Vec<Vec<Vec<BigInt>>>>) -> Self {
        Self::Id4(v)
    }
}

impl From<Vec<Vec<Vec<Vec<f64>>>>> for DataVec {
    fn from(v: Vec<Vec<Vec<Vec<f64>>>>) -> Self {
        Self::Fd4(v)
    }
}

impl From<Vec<Vec<Vec<f64>>>> for DataVec {
    fn from(v: Vec<Vec<Vec<f64>>>) -> Self {
        Self::Fd3(v)
    }
}

impl From<Vec<Vec<f64>>> for DataVec {
    fn from(v: Vec<Vec<f64>>) -> Self {
        Self::Fd2(v)
    }
}

impl From<Vec<f64>> for DataVec {
    fn from(v: Vec<f64>) -> Self {
        Self::Fd1(v)
    }
}

impl DataVec {
    /// Returns the number of elements in the memory. Will panic if the vectors
    /// do not all have the same length within a given dimension.
    pub fn size(&self) -> usize {
        // TODO griffin: make the variable names more reasonable
        match self {
            DataVec::Id1(v1) => v1.len(),
            DataVec::Id2(v1) => {
                let v0_size = v1[0].len();

                // Check that sizes are the same across each dimension
                assert!(v1.iter().all(|v2| v2.len() == v0_size));
                v1.len() * v0_size
            }
            DataVec::Id3(v1) => {
                let v1_0_size = v1[0].len();
                let v1_0_0_size = v1[0][0].len();

                // Check that sizes are the same across each dimension
                assert!(v1.iter().all(|v2| { v2.len() == v1_0_size }));
                assert!(v1
                    .iter()
                    .all(|v2| v2.iter().all(|v3| v3.len() == v1_0_0_size)));
                v1.len() * v1_0_size * v1_0_0_size
            }
            DataVec::Id4(v1) => {
                let v1_0_size = v1[0].len();
                let v1_0_0_size = v1[0][0].len();
                let v1_0_0_0_size = v1[0][0][0].len();
                // Check that sizes are the same across each dimension
                assert!(v1.iter().all(|v2| { v2.len() == v1_0_size }));
                assert!(v1
                    .iter()
                    .all(|v2| { v2.iter().all(|v3| v3.len() == v1_0_0_size) }));
                assert!(v1.iter().all(|v2| v2
                    .iter()
                    .all(|v3| v3.iter().all(|v4| v4.len() == v1_0_0_0_size))));

                v1.len() * v1_0_size * v1_0_0_size * v1_0_0_0_size
            }
            DataVec::Fd1(v1) => v1.len(),
            DataVec::Fd2(v1) => {
                let v0_size = v1[0].len();

                // Check that sizes are the same across each dimension
                assert!(v1.iter().all(|v2| v2.len() == v0_size));
                v1.len() * v0_size
            }
            DataVec::Fd3(v1) => {
                let v1_0_size = v1[0].len();
                let v1_0_0_size = v1[0][0].len();

                // Check that sizes are the same across each dimension
                assert!(v1.iter().all(|v2| { v2.len() == v1_0_size }));
                assert!(v1
                    .iter()
                    .all(|v2| v2.iter().all(|v3| v3.len() == v1_0_0_size)));
                v1.len() * v1_0_size * v1_0_0_size
            }
            DataVec::Fd4(v1) => {
                let v1_0_size = v1[0].len();
                let v1_0_0_size = v1[0][0].len();
                let v1_0_0_0_size = v1[0][0][0].len();
                // Check that sizes are the same across each dimension
                assert!(v1.iter().all(|v2| { v2.len() == v1_0_size }));
                assert!(v1
                    .iter()
                    .all(|v2| { v2.iter().all(|v3| v3.len() == v1_0_0_size) }));
                assert!(v1.iter().all(|v2| v2
                    .iter()
                    .all(|v3| v3.iter().all(|v4| v4.len() == v1_0_0_0_size))));

                v1.len() * v1_0_size * v1_0_0_size * v1_0_0_0_size
            }
        }
    }

    pub fn dimensions(&self) -> Dimensions {
        match self {
            DataVec::Id1(v) => Dimensions::D1(v.len()),
            DataVec::Id2(v) => Dimensions::D2(v.len(), v[0].len()),
            DataVec::Id3(v) => {
                Dimensions::D3(v.len(), v[0].len(), v[0][0].len())
            }
            DataVec::Id4(v) => Dimensions::D4(
                v.len(),
                v[0].len(),
                v[0][0].len(),
                v[0][0][0].len(),
            ),
            DataVec::Fd1(v) => Dimensions::D1(v.len()),
            DataVec::Fd2(v) => Dimensions::D2(v.len(), v[0].len()),
            DataVec::Fd3(v) => {
                Dimensions::D3(v.len(), v[0].len(), v[0][0].len())
            }
            DataVec::Fd4(v) => Dimensions::D4(
                v.len(),
                v[0].len(),
                v[0][0].len(),
                v[0][0][0].len(),
            ),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonDataEntry {
    pub data: ParseVec,
    pub format: FormatInfo,
}

// using a hashmap here means that the serialization is non-deterministic but
// for now that's probably fine
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonData(pub HashMap<String, JsonDataEntry>);

#[derive(Debug, Serialize)]
#[serde(untagged)]
/// A structure meant to mimic the old style of data dump printing.
pub enum JsonPrintDump {
    Normal(HashMap<String, ParseVec>),
    Quoted(HashMap<String, PrintVec>),
}

impl JsonPrintDump {
    #[must_use]
    pub fn as_normal(&self) -> Option<&HashMap<String, ParseVec>> {
        if let Self::Normal(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl From<HashMap<String, PrintVec>> for JsonPrintDump {
    fn from(v: HashMap<String, PrintVec>) -> Self {
        Self::Quoted(v)
    }
}

impl From<HashMap<String, ParseVec>> for JsonPrintDump {
    fn from(v: HashMap<String, ParseVec>) -> Self {
        Self::Normal(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_data() {
        let data = r#"
{
  "in": {
    "data": [[
      4.0
    ]],
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
        println!("{:?}", json_data);
        println!("{}", serde_json::to_string_pretty(&json_data).unwrap());
    }
}
