use std::collections::HashMap;

use serde::{self, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NumericType {
    Bitnum,
    Fixed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FormatInfo {
    pub numeric_type: NumericType,
    pub is_signed: bool,
    pub width: u64,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub int_width: Option<u64>,
}

// this is stupid
#[derive(Debug, Serialize, Deserialize)]
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

impl DataVec {
    /// Returns the number of elements in the memory. Will panic if the vectors
    /// do not all have the same length within a given dimension.
    pub fn size(&self) -> usize {
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

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_json_data() {
//         let data = r#"
// {
//   "in": {
//     "data": [[
//       4.0
//     ]],
//     "format": {
//       "numeric_type": "bitnum",
//       "is_signed": false,
//       "width": 32
//     }
//   },
//   "out": {
//     "data": [
//       6
//     ],
//     "format": {
//       "numeric_type": "bitnum",
//       "is_signed": false,
//       "width": 32
//     }
//   }
// }"#;

//         let json_data: JsonData = serde_json::from_str(data).unwrap();
//         println!("{:?}", json_data);
//         println!("{}", serde_json::to_string_pretty(&json_data).unwrap());
//     }
// }
