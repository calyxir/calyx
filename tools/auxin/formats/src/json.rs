use serde::{self, Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use struson::writer::WriterSettings;

use crate::filerep::{self, FileFmtErr, FileMems};
use num_ir::memrep::*;
use num_ir::typing::*;

use struson::{
    reader::*,
    writer::{JsonStreamWriter, JsonWriter},
};

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

fn json_err<T: ToString>(s: T) -> FileFmtErr {
    FileFmtErr::FileSpecific(format!("json: {}", s.to_string()))
}

/*
    ideally would also check for overspecified format (i.e. non-fixed_point with frac_width defined)
*/
impl FormatInfo {
    // returns fixed-point as (overall width, frac_width)
    // a bit verbose, but roughly self-documenting
    #[inline]
    fn normalise_fixed(&self) -> Result<(usize, i32), FileFmtErr> {
        if let Some(w) = self.width {
            if w > 64 {
                return Err(json_err("fixed width > 64"));
            }
            match (self.int_width, self.frac_width) {
                (Some(i), Some(f)) if i + f == w => Ok((w as usize, f as i32)),
                (None, Some(f)) if f < w => Ok((w as usize, f as i32)),
                (Some(i), None) if i < w => {
                    Ok((w as usize, (w as i32 - (i as i32))))
                }
                _ => Err(json_err(format!("malformed fixed type {self:?}"))),
            }
        } else {
            match (self.int_width, self.frac_width) {
                (Some(i), Some(f)) if i + f <= 64 => {
                    Ok(((i + f) as usize, f as i32))
                }
                _ => Err(json_err(format!("malformed fixed type {self:?}"))),
            }
        }
    }
}

impl TryFrom<&TypeSpec> for FormatInfo {
    type Error = FileFmtErr;

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
                return Err(FileFmtErr::FileSpecific(format!(
                    "unknown type class {:?}",
                    value.class
                )));
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
    type Error = FileFmtErr;
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

fn try_parse_format<R: Read>(
    r: &mut JsonStreamReader<R>,
) -> Result<FormatInfo, FileFmtErr> {
    r.deserialize_next().map_err(|e| {
        FileFmtErr::FileSpecific(format!("json: {}", e.to_string()))
    })
}

/*
    NOTE: as implemented, some fixed point numbers which exceed the precision of f64 but are still valid fixed-point may not work.

    ``Value`` is maybe not the ideal way to do this, but is good enough
*/

struct JsonEntry {
    data: Vec<String>,
    format: FormatInfo,
    dim_sizes: Vec<usize>,
    is_quoted: bool,
}

impl JsonEntry {
    fn try_entry_to_ir(
        self,
        t: &TypeSpec,
    ) -> Result<SingleMem, filerep::FileFmtErr> {
        let data: Vec<_> = self
            .data
            .iter()
            .map(|e| t.read_str(&e.to_string(), Endian::Little))
            .collect::<Result<_, _>>()?;

        let mut d = [0; 4];
        for (idx, v) in self.dim_sizes.iter().enumerate() {
            d[idx] = *v;
        }

        // assert_eq!(
        //     self.dim_sizes
        //         .iter()
        //         .filter(|e| { e != 0 })
        //         .product::<usize>(),
        //     data.len()
        // );
        Ok(SingleMem::new(
            data,
            d,
            self.dim_sizes.len(),
            t.clone(),
            Endian::Little,
        ))
    }
    fn try_entry_from_ir(
        inp: &SingleMem,
        opts: Option<&filerep::OutputOpts>,
    ) -> Result<JsonEntry, filerep::FileFmtErr> {
        let is_bin = inp.ty().class == TypeClass::Bits;
        let is_hex = opts.is_some_and(|x| x.print_hex);

        let as_num = inp
            .iter_data()
            .map(|e| {
                if is_hex {
                    inp.ty().write_hexstring(e, Endian::Little)
                } else {
                    inp.ty().write_string(e, Endian::Little)
                }
            })
            .collect();
        Ok(JsonEntry {
            data: as_num,
            format: FormatInfo::try_from(inp.ty())?,
            dim_sizes: inp.dimensions[..inp.num_dimensions].to_vec(),
            is_quoted: is_hex || is_bin,
        })
    }
}

impl From<struson::reader::ReaderError> for FileFmtErr {
    fn from(value: struson::reader::ReaderError) -> Self {
        FileFmtErr::FileSpecific(value.to_string())
    }
}

impl From<struson::writer::JsonNumberError> for FileFmtErr {
    fn from(value: struson::writer::JsonNumberError) -> Self {
        FileFmtErr::FileSpecific(value.to_string())
    }
}

impl From<struson::serde::SerializerError> for FileFmtErr {
    fn from(value: struson::serde::SerializerError) -> Self {
        FileFmtErr::FileSpecific(value.to_string())
    }
}

impl From<struson::serde::DeserializerError> for FileFmtErr {
    fn from(value: struson::serde::DeserializerError) -> Self {
        FileFmtErr::FileSpecific(value.to_string())
    }
}

// maps dimension : width

#[derive(Default)]
struct ArrayParseInfo {
    data: Vec<String>,
    dim_sizes: Vec<usize>,
}
fn write_value<W: Write>(
    v: &str,
    r: &mut JsonStreamWriter<W>,
    are_str: bool,
) -> Result<(), FileFmtErr> {
    if are_str {
        r.string_value(v).map_err(|e| json_err(e))
    } else {
        r.number_value_from_string(v).map_err(|e| json_err(e))
    }
}

fn write_arr_from_iterable<'a, W: Write>(
    i: impl Iterator<Item = &'a String>,
    w: &mut JsonStreamWriter<W>,
    are_str: bool,
) -> Result<(), FileFmtErr> {
    w.begin_array()?;

    for el in i {
        write_value(&el, w, are_str)?;
    }
    w.end_array()?;
    Ok(())
}

impl ArrayParseInfo {
    /// assumes that we are at the start of the array, no key attached to it
    fn parse_nested_inner<R: Read>(
        &mut self,
        r: &mut JsonStreamReader<R>,
        curr_dim: usize,
    ) -> Result<(), FileFmtErr> {
        r.begin_array()?;
        let mut curr_type: Option<ValueType> = None; // type of current level
        let mut item_ct = 0;
        if self.dim_sizes.len() == curr_dim {
            self.dim_sizes.push(0)
        }
        while r.has_next()? {
            let nt = r.peek()?;
            if curr_type.is_none() {
                curr_type = Some(nt)
            }
            if nt != curr_type.unwrap() {
                return Err(json_err("hit elements of mismatched type"));
            }
            match nt {
                ValueType::Array => self.parse_nested_inner(r, curr_dim + 1)?,
                ValueType::String => {
                    let s = r.next_string()?;
                    self.data.push(s)
                }
                ValueType::Number => {
                    let n = r.next_number_as_string()?;
                    self.data.push(n)
                }
                _ => return Err(json_err("unknown type")),
            };
            item_ct += 1;
        }
        if self.dim_sizes[curr_dim] == 0 {
            self.dim_sizes[curr_dim] = item_ct
        } else if self.dim_sizes[curr_dim] != item_ct {
            return Err(json_err("hit elements of mismatched length"));
        }
        r.end_array()?;
        Ok(())
    }

    // TODO: make this less evil
    /// basically no type verification, the assumption is that we're provided with some string array already
    /// this is pretty terrible but should suffice
    fn write_nested_inner<W: Write>(
        self,
        w: &mut JsonStreamWriter<W>,
        are_str: bool,
    ) -> Result<(), FileFmtErr> {
        debug_assert!(
            self.data.len() == self.dim_sizes.iter().product::<usize>()
        );
        w.begin_array()?;
        match self.dim_sizes.len() {
            1 => {
                write_arr_from_iterable(self.data.iter(), w, are_str)?;
            }
            2 => {
                let d1_size = self.dim_sizes[1];

                for d1_chunk in self.data.chunks(d1_size) {
                    write_arr_from_iterable(d1_chunk.iter(), w, are_str)?;
                }
            }
            3 => {
                let d1_size: usize = self.dim_sizes[1..3].iter().product();
                let d2_size = self.dim_sizes[2];
                for d1_chunk in self.data.chunks(d1_size) {
                    w.begin_array()?;
                    for d2_chunk in d1_chunk.chunks(d2_size) {
                        write_arr_from_iterable(d2_chunk.iter(), w, are_str)?;
                    }

                    w.end_array()?;
                }
            }
            4 => {
                let d1_size: usize = self.dim_sizes[1..4].iter().product();
                let d2_size: usize = self.dim_sizes[2..4].iter().product();
                let d3_size = self.dim_sizes[3];
                for d1_chunk in self.data.chunks(d1_size) {
                    w.begin_array()?;
                    for d2_chunk in d1_chunk.chunks(d2_size) {
                        w.begin_array()?;
                        for d3_chunk in d2_chunk.chunks(d3_size) {
                            write_arr_from_iterable(
                                d3_chunk.iter(),
                                w,
                                are_str,
                            )?;
                        }
                        w.end_array()?;
                    }

                    w.end_array()?;
                }
            }
            _ => panic!("bad dim_ct"),
        }
        w.end_array()?;

        Ok(())
    }
}

// using a hashmap here means that the serialization is non-deterministic but
// for now that's probably fine

pub struct JsonData(pub BTreeMap<String, JsonEntry>);

impl filerep::TryFromIR for JsonData {
    fn try_from_ir(inp: FileMems) -> Result<Self, filerep::FileFmtErr> {
        let mut res = BTreeMap::new();
        for (k, v) in inp.mems.iter() {
            res.insert(k.to_string(), JsonEntry::try_entry_from_ir(v, None)?);
        }
        Ok(JsonData(res))
    }
}

impl JsonData {
    pub fn try_from_ir_fmt(
        inp: FileMems,
        opts: &filerep::OutputOpts,
    ) -> Result<Self, FileFmtErr> {
        let mut res = BTreeMap::new();
        for (k, v) in inp.mems.iter() {
            res.insert(
                k.to_string(),
                JsonEntry::try_entry_from_ir(v, Some(opts))?,
            );
        }
        Ok(JsonData(res))
    }
}

impl filerep::TryToIR for JsonData {
    fn try_to_ir(
        self,
        types: &HashMap<String, TypeSpec>,
    ) -> Result<FileMems, filerep::FileFmtErr> {
        let mut new_mems = FileMems {
            mems: HashMap::new(),
        };

        for (k, v) in self.0.into_iter() {
            let ty = types.get(&k).unwrap();
            new_mems.mems.insert(k, v.try_entry_to_ir(ty)?);
        }
        Ok(new_mems)
    }
}

impl From<serde_json::Error> for FileFmtErr {
    fn from(value: serde_json::Error) -> Self {
        Self::FileSpecific(format!("json: {}", value))
    }
}

impl filerep::FileIO for JsonData {
    fn read_into(src: Box<dyn std::io::Read>) -> Result<JsonData, FileFmtErr> {
        let mut sr = JsonStreamReader::new(src);
        let mut res = BTreeMap::<String, JsonEntry>::new();
        sr.begin_object()?;

        while sr.has_next()? {
            let k = sr.next_name_owned()?;

            sr.begin_object()?;
            let e1 = sr.next_name()?;
            if e1 != "data" {
                panic!("not data")
            }
            let mut ap = ArrayParseInfo::default();
            ap.parse_nested_inner(&mut sr, 0)?;
            let e2 = sr.next_name()?;
            if e2 != "format" {
                panic!("not format")
            }
            let f: FormatInfo = sr.deserialize_next()?;
            res.insert(
                k,
                JsonEntry {
                    data: ap.data,
                    format: f,
                    dim_sizes: ap.dim_sizes,
                    is_quoted: false,
                },
            );
            sr.end_object()?;
        }
        sr.end_object()?;

        Ok(JsonData(res))
    }
    fn write_out(
        &self,
        dest: Box<dyn std::io::Write>,
    ) -> Result<(), FileFmtErr> {
        // let mut sw = JsonStreamWriter::new(dest);

        let mut sw = JsonStreamWriter::new_custom(
            dest,
            WriterSettings {
                pretty_print: true,
                // For all other settings use the default
                ..Default::default()
            },
        );
        sw.begin_object()?;
        for (k, v) in self.0.iter() {
            sw.name(k)?;
            sw.begin_object()?;
            sw.name("data")?;
            let ap = ArrayParseInfo {
                data: v.data.clone(),
                dim_sizes: v.dim_sizes.clone(),
            };
            ap.write_nested_inner(&mut sw, false)?;
            sw.name("format")?;
            sw.serialize_value(&v.format)?;
            sw.end_object()?;
        }
        sw.end_object()?;

        Ok(())
    }
}

impl filerep::ExtractType for JsonData {
    fn extract_types(&self) -> Result<HashMap<String, TypeSpec>, FileFmtErr> {
        let mut res = HashMap::new();
        for (k, v) in self.0.iter() {
            let new_spec = TypeSpec::try_from(&v.format)?;
            res.insert(k.clone(), new_spec);
        }
        Ok(res)
    }
}

impl filerep::HintedTryToIR for JsonData {}
