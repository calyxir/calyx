use std::collections::HashMap;

use interp::serialization::data_dump::Dimensions;
use serde::{self, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum NumericType {
    Bitnum,
    Fixed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormatInfo {
    pub numeric_type: NumericType,
    pub is_signed: bool,
    pub width: u64,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub int_width: Option<u64>,
}

// this is stupid
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum DataVec {
    // integers for the bitnum values
    Id1(Vec<u64>),
    Id2(Vec<Vec<u64>>),
    Id3(Vec<Vec<Vec<u64>>>),
    Id4(Vec<Vec<Vec<Vec<u64>>>>),
    // the float initialization of fixed point values
    Fd1(Vec<f64>),
    Fd2(Vec<Vec<f64>>),
    Fd3(Vec<Vec<Vec<f64>>>),
    Fd4(Vec<Vec<Vec<Vec<f64>>>>),
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

impl From<Vec<Vec<Vec<Vec<u64>>>>> for DataVec {
    fn from(v: Vec<Vec<Vec<Vec<u64>>>>) -> Self {
        Self::Id4(v)
    }
}

impl From<Vec<Vec<Vec<u64>>>> for DataVec {
    fn from(v: Vec<Vec<Vec<u64>>>) -> Self {
        Self::Id3(v)
    }
}

impl From<Vec<Vec<u64>>> for DataVec {
    fn from(v: Vec<Vec<u64>>) -> Self {
        Self::Id2(v)
    }
}

impl From<Vec<u64>> for DataVec {
    fn from(v: Vec<u64>) -> Self {
        Self::Id1(v)
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
            DataVec::Fd1(_) => todo!("implement fixed-point"),
            DataVec::Fd2(_) => todo!("implement fixed-point"),
            DataVec::Fd3(_) => todo!("implement fixed-point"),
            DataVec::Fd4(_) => todo!("implement fixed-point"),
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
    pub data: DataVec,
    pub format: FormatInfo,
}

// using a hashmap here means that the serialization is non-deterministic but
// for now that's probably fine
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonData(pub HashMap<String, JsonDataEntry>);

#[derive(Debug, Serialize)]
#[serde(transparent)]
/// A structure meant to mimic the old style of data dump printing.
pub struct JsonPrintDump(pub HashMap<String, DataVec>);

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
