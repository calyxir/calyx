use serde::{self, Deserialize, Serialize, Serializer};
use serde_json::Value;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::{collections::HashMap, num::ParseFloatError};
use thiserror::Error;

use crate::filerep::{self, FileMems};
use crate::numimpl::Bits;
use crate::numrep as nr;

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
    // returns fixed-point as (overall width, exp_width)
    // a bit verbose, but roughly self-documenting
    #[inline]
    fn normalise_fixed(&self) -> Result<(u32, u32), JsonParseError> {
        if let Some(w) = self.width {
            if w > 64 {
                return Err(JsonParseError::MalformedFixed);
            }
            match (self.int_width, self.frac_width) {
                (Some(i), Some(f)) if i + f == w => Ok((w, i)),
                (None, Some(f)) if f < w => Ok((w, w - f)),
                (Some(i), None) if i < w => Ok((w, i)),
                _ => Err(JsonParseError::MalformedFixed),
            }
        } else {
            match (self.int_width, self.frac_width) {
                (Some(i), Some(f)) if i + f <= 64 => Ok((i + f, i)),
                _ => Err(JsonParseError::MalformedFixed),
            }
        }
    }

    fn from_ir_repr(t: nr::ReprAs, width: usize) -> Self {
        let mut ty = JsonTypes::Bitnum;
        let mut is_signed = false;
        let mut exp_width = None;
        match t {
            nr::ReprAs::Bits => (),
            nr::ReprAs::Int { signed: s } => {
                is_signed = s;
            }
            nr::ReprAs::Float => ty = JsonTypes::IEEE754Float,
            nr::ReprAs::Fixed {
                signed: s,
                exp_width: e,
            } => {
                is_signed = s;
                exp_width = Some(e)
            }
            _ => panic!("unknown type!"),
        };
        Self {
            numeric_type: ty,
            is_signed,
            width: Some(width as u32),
            int_width: exp_width,
            frac_width: None,
        }
    }
}

impl<T: nr::ReprType> From<PhantomData<T>> for FormatInfo {
    fn from(_value: PhantomData<T>) -> Self {
        Self {
            numeric_type: JsonTypes::IEEE754Float,
            is_signed: false,
            width: Some(64),
            int_width: None,
            frac_width: None,
        }
    }
}

// NOTE: width stuff gets kinda dropped here. probably best to defer checking.

// operates on reference b/c the FormatInfo is used to fill fields in DataSet and DataType
// impl TryFrom<&FormatInfo> for repr::DataType {
//     type Error = JsonParseError;

//     fn try_from(value: &FormatInfo) -> Result<Self, Self::Error> {
//         match value.numeric_type {
//             JsonTypes::Untyped => Ok(repr::DataType::Untyped),
//             JsonTypes::Bitnum => Ok(repr::DataType::Int {
//                 is_signed: value.is_signed,
//             }),
//             JsonTypes::Fixed => Ok(repr::DataType::Fixed {
//                 frac_width: value.normalise_fixed()?.1,
//                 is_signed: value.is_signed,
//             }),
//             JsonTypes::IEEE754Float => Ok(repr::DataType::Float),
//         }
//     }
// }

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

fn chunks_size_n(inp: Vec<Value>, size: usize) -> Vec<Value> {
    inp.chunks(size)
        .into_iter()
        .map(|e| serde_json::to_value(Vec::from(e)).unwrap())
        .collect()
}

fn reshape(inp: Vec<Value>, shape: [usize; 4], dims: usize) -> Vec<Value> {
    match dims {
        1 => inp
            .iter()
            .map(|e| serde_json::to_value(e).unwrap())
            .collect(),
        2 => {
            let d2_size = shape.get(1).unwrap();

            inp.chunks(*d2_size)
                .into_iter()
                .map(|e| serde_json::to_value(Vec::from(e)).unwrap())
                .collect()
        }
        3 => {
            let d2_size: usize = shape[1..3].iter().product();
            let d3_size = shape.get(2).unwrap();
            inp.chunks(d2_size)
                .into_iter()
                .map(|e| {
                    serde_json::to_value(chunks_size_n(Vec::from(e), *d3_size))
                        .unwrap()
                })
                .collect()
        }
        4 => {
            let d2_size: usize = shape[1..4].iter().product();
            let d3_size: usize = shape[2..4].iter().product();
            let d4_size = shape.get(3).unwrap();
            inp.chunks(d2_size)
                .into_iter()
                .map(|e| {
                    e.chunks(d3_size)
                        .into_iter()
                        .map(|e2| {
                            serde_json::to_value(chunks_size_n(
                                Vec::from(e2),
                                *d4_size,
                            ))
                            .unwrap()
                        })
                        .collect()
                })
                .collect()
        }
        _ => panic!("bad dim_ct"),
    }
}

impl filerep::TryToIR for JsonDataEntry {
    fn try_to_ir<T: nr::ReprType>(
        inp: Self,
    ) -> Result<nr::DataSet<T>, filerep::FileFmtErr> {
        let Ok((vals, dimensions, num_dimensions)) = destructure(&inp.data)
        else {
            return Err(String::from("bad flattening"));
        };
        let data: Vec<u64> = vals
            .iter()
            .map(|e| T::from_string_lossy(&e.to_string(), nr::Endian::Little))
            .collect();
        assert_eq!(
            dimensions
                .into_iter()
                .filter(|e| { *e != 0 })
                .product::<usize>(),
            data.len()
        );
        Ok(nr::DataSet::<T> {
            data,
            dimensions,
            num_dimensions,
            dtype: PhantomData::<T>,
            end: nr::Endian::Little,
        })
    }
}

impl JsonDataEntry {
    fn to_dyn_ir(self) -> Box<dyn nr::DataTrait> {
        use crate::filerep::*;
        match self.format.numeric_type {
            JsonTypes::Untyped => Box::new(
                JsonDataEntry::try_to_ir::<crate::numimpl::UInt64>(self)
                    .unwrap(),
            ),
            JsonTypes::Fixed => Box::new(
                JsonDataEntry::try_to_ir::<crate::numimpl::IFixed32E16>(self)
                    .unwrap(),
            ),
            JsonTypes::Bitnum => Box::new(
                JsonDataEntry::try_to_ir::<crate::numimpl::UInt64>(self)
                    .unwrap(),
            ),
            JsonTypes::IEEE754Float => Box::new(
                JsonDataEntry::try_to_ir::<crate::numimpl::Float64>(self)
                    .unwrap(),
            ),
        }
    }
}

impl<T: nr::ReprType> filerep::TryFromIR<T, JsonDataEntry> for JsonDataEntry {
    fn try_from_ir(
        inp: &nr::DataSet<T>,
    ) -> Result<JsonDataEntry, filerep::FileFmtErr> {
        use crate::numimpl;
        use crate::numrep::ReprType;

        let as_num = inp
            .data
            .iter()
            .map(|e| {
                serde_json::Value::Number(
                    serde_json::Number::from_string_unchecked(
                        numimpl::Float64::to_str(e, nr::Endian::Little),
                    ),
                )
            })
            .collect();
        Ok(JsonDataEntry {
            data: serde_json::to_value(reshape(
                as_num,
                inp.dimensions,
                inp.num_dimensions,
            ))
            .unwrap(),
            format: FormatInfo::from_ir_repr(T::repr_as(), T::WIDTH),
        })
    }
}

// using a hashmap here means that the serialization is non-deterministic but
// for now that's probably fine
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonData(pub HashMap<String, JsonDataEntry>);

impl JsonData {
    fn to_filemems(self) -> filerep::FileMems {
        let mut new_mems = FileMems {
            store: HashMap::new(),
        };

        // let mut json_data: JsonData = serde_json::from_str(data).unwrap();
        // let nested_data = json_data.0.remove("in").unwrap();

        // let t: nr::DataSet<crate::numimpl::Float64> =
        //     JsonDataEntry::try_to_ir(nested_data).unwrap();

        // use filerep::TryToIR;

        // let x: Box<dyn nr::DataTrait> = Box::new(t);
        // new_mems.store.insert(String::from("aa"), x);
        for (k, v) in self.0.into_iter() {
            new_mems.store.insert(k, JsonDataEntry::to_dyn_ir(v));
        }
        return new_mems;
    }
}

struct JsonDataDestructor {
    pub dimensions: [usize; 4], // maps dimension : dimension size
    pub num_dimensions: usize,
    // pub dimensions: HashMap<u32, usize>, // maps dimension : dimension size
    pub status: Option<JsonParseError>,
}

/*
    destruct an n-dimensional array. Very Bad, no good
    probably an excess of allocations. if only there were some easy way to do this...
*/
fn destructure_helper(
    destr: &mut JsonDataDestructor,
    v: &Value,
    level: usize,
) -> Vec<Value> {
    use crate::json::JsonParseError::*;

    let Value::Array(arr) = v else {
        destr.status = Some(NonNumError(v.to_string()));
        // return Err(NonNumError(v.to_string()));
        return Vec::new();
    };
    let level_size = destr.dimensions.get_mut(level).unwrap();
    if *level_size == 0 {
        destr.num_dimensions += 1;
        *level_size = arr.len();
    } else if *level_size != arr.len() {
        destr.status = Some(JsonParseError::DimError);
        // return Err(JsonParseError::DimError);

        return Vec::new();
    }
    match arr.first().unwrap() {
        Value::Number(_) => arr.clone(),
        Value::Array(_) => arr
            .iter()
            .flat_map(|v: &Value| destructure_helper(destr, v, level + 1))
            .collect(),
        _ => {
            destr.status = Some(NonNumError(arr.first().unwrap().to_string()));
            // return Err(NonNumError(arr.first().unwrap().to_string()));
            return Vec::new();
        }
    }
}

fn destructure(
    v: &Value,
) -> Result<(Vec<Value>, [usize; 4], usize), JsonParseError> {
    let mut destr = JsonDataDestructor {
        dimensions: [0, 0, 0, 0],
        num_dimensions: 0,
        status: None,
    };
    let res = destructure_helper(&mut destr, v, 0);
    match destr.status {
        Some(e) => Err(e),
        None => Ok((res, destr.dimensions, destr.num_dimensions)),
    }
}

// impl<T: nr::ReprType> StringFormat<T> for JsonDataEntry {}
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
    use std::borrow;

    use crate::{numimpl, numrep::DataSet};

    use super::*;
    use filerep::*;

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
        let fm = json_data.to_filemems();
        for (k, v) in fm.store {
            // println!("{json_data:?}");
            // let nested_data = json_data.0.get("in").unwrap();
            // println!("dest: {:?}", destructure(nested_data));

            // let t: DataSet<numimpl::Float64> =
            //     JsonDataEntry::try_to_ir(v).unwrap();
            let m = v.as_ref();
            println!("ir: {:?}", m);
            // let as_str = t
            //     .data
            //     .iter()
            //     .map(|e| {
            //         serde_json::Value::Number(
            //             serde_json::Number::from_string_unchecked(
            //                 numimpl::Float64::to_str(e, nr::Endian::Little),
            //             ),
            //         )
            //     })
            //     .collect();
            // println!("{:?}", t.dimensions);
            // println!("{:?}", reshape(as_str, t.dimensions, t.num_dimensions));
            // // for e in t.data {
            // //     use nr::ReprType;
            // //     println!(
            // //         "{}",
            // //         numimpl::Float64::to_str(&e, nr::Endian::Little)
            // //     );
            // // }
            // let njs = JsonDataEntry::try_from_ir(&t);
            // println!("{:?}", njs)
        }

        // println!("{}", serde_json::to_string_pretty(&json_data).unwrap());
    }
}
