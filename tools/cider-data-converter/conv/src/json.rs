use serde::{self, Deserialize, Serialize, Serializer};
use serde_json::Value;
use std::collections::BTreeMap;
use std::{collections::HashMap, num::ParseFloatError};
use thiserror::Error;

use crate::filerep::{self, FileFmtErr, FileMems};
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
    #[error("Unknown type")]
    BadType,
}

impl From<JsonParseError> for filerep::FileFmtErr {
    fn from(value: JsonParseError) -> Self {
        FileFmtErr::from(value.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JsonTypes {
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
    fn normalise_fixed(&self) -> Result<(usize, i32), JsonParseError> {
        if let Some(w) = self.width {
            if w > 64 {
                return Err(JsonParseError::MalformedFixed);
            }
            match (self.int_width, self.frac_width) {
                (Some(i), Some(f)) if i + f == w => Ok((w as usize, f as i32)),
                (None, Some(f)) if f < w => Ok((w as usize, f as i32)),
                (Some(i), None) if i < w => {
                    Ok((w as usize, (w as i32 - (i as i32))))
                }
                _ => Err(JsonParseError::MalformedFixed),
            }
        } else {
            match (self.int_width, self.frac_width) {
                (Some(i), Some(f)) if i + f <= 64 => {
                    Ok(((i + f) as usize, f as i32))
                }
                _ => Err(JsonParseError::MalformedFixed),
            }
        }
    }
}

impl TryFrom<nr::TypeSpec> for FormatInfo {
    type Error = JsonParseError;

    fn try_from(value: nr::TypeSpec) -> Result<Self, Self::Error> {
        use nr::TypeClass as tc;
        let mut frac_width = None;
        let numeric_type = match value.class {
            tc::Bits => JsonTypes::Bitnum,
            tc::Int => JsonTypes::Bitnum,
            tc::Float => JsonTypes::IEEE754Float,
            tc::Fixed { exp_mag: e } => {
                frac_width = Some(if e < 0 { 0 } else { e } as u32);
                JsonTypes::Fixed
            }
            _ => return Err(JsonParseError::BadType),
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

impl TryFrom<&FormatInfo> for nr::TypeSpec {
    type Error = JsonParseError;
    fn try_from(value: &FormatInfo) -> Result<Self, Self::Error> {
        use nr::TypeClass as tc;

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
        Ok(nr::TypeSpec {
            width: total_width.unwrap() as usize,
            signed: value.is_signed,
            class,
        })
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

// split array into sub-arrays of [size] length
fn chunks_size_n(inp: Vec<Value>, size: usize) -> Vec<Value> {
    inp.chunks(size)
        .into_iter()
        .map(|e| serde_json::to_value(Vec::from(e)).unwrap())
        .collect()
}

// this is awful!
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

impl JsonDataEntry {
    fn try_entry_to_ir(
        self,
        t: &nr::TypeSpec,
    ) -> Result<nr::SingleMem, filerep::FileFmtErr> {
        let Ok((vals, dimensions, num_dimensions)) = destructure(&self.data)
        else {
            return Err(FileFmtErr::FileSpecific(String::from(
                "bad flattening",
            )));
        };
        let data: Vec<u64> = vals
            .iter()
            .map(|e| t.read_string(e.to_string(), nr::Endian::Little))
            .collect::<Result<_, _>>()?;

        assert_eq!(
            dimensions
                .into_iter()
                .filter(|e| { *e != 0 })
                .product::<usize>(),
            data.len()
        );
        return Ok(nr::SingleMem::new(
            data,
            dimensions,
            num_dimensions,
            t.clone(),
            nr::Endian::Little,
        ));
    }
    fn try_entry_from_ir(
        inp: &nr::SingleMem,
    ) -> Result<JsonDataEntry, filerep::FileFmtErr> {
        let as_num = inp
            .iter_data()
            .map(|e| {
                serde_json::Value::Number(
                    serde_json::Number::from_string_unchecked(
                        inp.ty().write_string(*e, nr::Endian::Little),
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
            format: FormatInfo::try_from(inp.ty())?,
        })
    }
}

// using a hashmap here means that the serialization is non-deterministic but
// for now that's probably fine
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonData(
    #[serde(serialize_with = "ordered_map")] pub HashMap<String, JsonDataEntry>,
);

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

impl filerep::TryFromIR for JsonData {
    fn try_from_ir(inp: &FileMems) -> Result<Self, filerep::FileFmtErr> {
        let mut res = HashMap::new();
        for (k, v) in inp.mems.iter() {
            res.insert(k.clone(), JsonDataEntry::try_entry_from_ir(v)?);
        }
        Ok(JsonData(res))
    }
}

impl filerep::TryToIR for JsonData {
    fn try_to_ir(
        self,
        types: &HashMap<String, nr::TypeSpec>,
    ) -> Result<FileMems, filerep::FileFmtErr> {
        let mut new_mems = FileMems {
            mems: HashMap::new(),
        };

        for (k, v) in self.0.into_iter() {
            let ty = types.get(&k).unwrap();
            new_mems.mems.insert(k, v.try_entry_to_ir(ty)?);
        }
        return Ok(new_mems);
    }
}

impl From<serde_json::Error> for FileFmtErr {
    fn from(value: serde_json::Error) -> Self {
        FileFmtErr::FileSpecific(value.to_string())
    }
}

impl filerep::FileIO for JsonData {
    fn read_into(src: Box<dyn std::io::Read>) -> Result<JsonData, FileFmtErr> {
        let res: JsonData = serde_json::from_reader(src)?;
        Ok(res)
    }
    fn write_out(
        &self,
        dest: Box<dyn std::io::Write>,
    ) -> Result<(), FileFmtErr> {
        serde_json::to_writer_pretty(dest, self)?;
        Ok(())
    }
}

impl filerep::ExtractType for JsonData {
    fn extract_types(
        &self,
    ) -> Result<HashMap<String, nr::TypeSpec>, FileFmtErr> {
        let mut res = HashMap::new();
        for (k, v) in self.0.iter() {
            let new_spec = nr::TypeSpec::try_from(&v.format)?;
            res.insert(k.clone(), new_spec);
        }
        Ok(res)
    }
}

impl filerep::HintedTryToIR for JsonData {}

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
      "numeric_type": "ieee754_float",
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
        use filerep::HintedTryToIR;

        let fm = json_data.hinted_try_to_ir().unwrap();
        for (_, v) in fm.mems.iter() {
            let as_str = v
                .iter_data()
                .map(|e| {
                    serde_json::Value::Number(
                        serde_json::Number::from_string_unchecked(
                            crate::numimpl::float_write(
                                *e,
                                nr::Endian::Little,
                                32,
                            ),
                        ),
                    )
                })
                .collect::<Vec<Value>>();
            println!("{:?}", v.dimensions);
            println!("{:?}", as_str);
            // println!("{:?}", reshape(as_str, v.dimensions, v.num_dimensions));
            // for e in t.data {
            //     use nr::ReprType;
            //     println!(
            //         "{}",
            //         numimpl::Float64::to_str(&e, nr::Endian::Little)
            //     );
            // }
        }
        // use crate::filerep::TryFromIR;
        // let njs = JsonData::try_from_ir(&fm, &typeprops).unwrap();
        // println!("{:?}", njs)

        // println!("{}", serde_json::to_string_pretty(&json_data).unwrap());
    }
}
