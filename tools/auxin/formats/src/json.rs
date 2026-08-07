use baa::BitVecValue;
use std::collections::HashMap;
use std::io::{Read, Write};
use struson::writer::WriterSettings;

use crate::filerep::{self, FileFmtErr, FileMems, OutputOpts};
use crate::json_common::*;
use num_ir::memrep::*;
use num_ir::typing::*;

use smallvec::SmallVec;

use struson::{
    reader::*,
    writer::{JsonStreamWriter, JsonWriter},
};

pub struct JsonEntry {
    data: Vec<String>,
    format: Option<FormatInfo>,
    dim_sizes: SmallVec<[usize; 4]>,
    is_quoted: bool,
}

impl Default for JsonEntry {
    fn default() -> Self {
        Self {
            data: Vec::with_capacity(10),
            format: None,
            dim_sizes: SmallVec::with_capacity(4),
            is_quoted: false,
        }
    }
}

// private b/c only used during the array read operation
#[derive(Default)]
pub struct ArrayReader {
    data: Vec<BitVecValue>,
    dim_sizes: SmallVec<[usize; 4]>,
    is_quoted: bool,
}

// TODO: can use number of elements in dim, once known, as guiding assumption
impl ArrayReader {
    fn read_arr_helper<R: Read>(
        &mut self,
        r: &mut JsonStreamReader<R>,
        t: &TypeSpec,
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
                ValueType::Array => self.read_arr_helper(r, t, curr_dim + 1)?,
                ValueType::String => {
                    let s = r.next_string()?;
                    let parsed = t.read_str(&s, Endian::Little)?;
                    self.data.push(parsed)
                }
                ValueType::Number => {
                    let n = r.next_number_as_string()?;
                    let parsed = t.read_str(&n, Endian::Little)?;
                    self.data.push(parsed)
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
        if matches!(curr_type, Some(ValueType::String)) {
            self.is_quoted = true;
        }
        r.end_array()?;
        Ok(())
    }
}

pub mod ops {
    use std::{io::Read, io::Write};

    use num_ir::{
        memrep::SingleMem,
        typing::{Endian, TypeClass, TypeSpec},
    };
    use struson::{
        json_path,
        reader::{JsonReader, JsonStreamReader, json_path::JsonPath},
        writer::{JsonStreamWriter, JsonWriter},
    };

    use crate::{
        filerep::FileFmtErr,
        json::ArrayReader,
        json_common::{FormatInfo, json_err},
    };

    /// assumes that we are at the start of the array, no key attached to it
    pub fn try_read_arr<R: Read>(
        r: &mut JsonStreamReader<R>,
        t: &TypeSpec,
    ) -> Result<super::ArrayReader, FileFmtErr> {
        let mut a = ArrayReader::default();
        let _ = a.read_arr_helper(r, t, 0)?;
        Ok(a)
    }

    // NOTE: expects elements to be ordered "format" "data"
    // when not reading to string, if format is not first, need to set a seek at current point, scan forwards until find format, read, go back, and then skip format

    // pub fn try_read_obj<R: Read>(
    //     r: &mut JsonStreamReader<R>,
    // ) -> Result<SingleMem, FileFmtErr> {
    //     r.begin_object()?;

    //     while r.has_next()? {
    //         let el_name = r.next_name_owned()?;
    //         if el_name == "data" {
    //             if ty == None {
    //                 println!("no format");
    //                 r.seek_to(&json_path!["format"])?;
    //                 return_path = true;
    //                 continue;
    //             } else {
    //                 println!("m2");
    //                 let t_ref = ty.as_ref();
    //                 ar = Some(try_read_arr(r, t_ref.unwrap())?);
    //             }
    //         } else if el_name == "format" {
    //             println!("m3");

    //             let f: FormatInfo = r.deserialize_next()?;
    //             ty = Some(TypeSpec::try_from(&f)?);
    //             if return_path {
    //                 r.seek_back(&json_path!["format"])?;
    //                 return_path = false;
    //             }
    //         } else {
    //             r.skip_value()?;
    //         }
    //     }

    //     r.end_object()?;

    //     // TODO: get this working again
    //     // assert_eq!(
    //     //     self.dim_sizes
    //     //         .iter()
    //     //         .filter(|e| { e != 0 })
    //     //         .product::<usize>(),
    //     //     data.len()
    //     // );
    //     let Some(d) = ar else {
    //         return Err(FileFmtErr::FileSpecific("bad".to_string()));
    //     };

    //     Ok(SingleMem::new(
    //         d.data,
    //         d.dim_sizes,
    //         ty.unwrap(),
    //         Endian::Little,
    //     ))
    // }

    pub fn try_write_obj<W: Write>(
        m: SingleMem,
        w: &mut JsonStreamWriter<W>,
    ) -> Result<(), FileFmtErr> {
        let is_bin = m.ty().class == TypeClass::Bits;
        // let is_hex = opts.is_some_and(|x| x.print_hex);
        let is_hex = false;

        w.begin_object()?;
        w.name("data")?;

        // debug_assert!(
        //     self.data.len() == self.dim_sizes.iter().product::<usize>()
        // );

        // when writing out json data, only do so in a 1D array.
        w.begin_array()?;

        for el in m.iter_data() {
            let s_to_write = if is_hex {
                m.ty().write_hexstring(el, Endian::Little)
            } else {
                m.ty().write_string(el, Endian::Little)
            };
            if is_bin || is_hex {
                w.string_value(&s_to_write).map_err(json_err)?;
            } else {
                w.number_value_from_string(&s_to_write).map_err(json_err)?;
            }
        }
        w.end_array()?;

        w.name("format")?;
        let json_t = FormatInfo::try_from(m.ty())?;

        w.serialize_value(&json_t)?;
        w.end_object()?;

        Ok(())
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

pub struct JsonRx {
    pub src: Box<dyn Read>,
}

pub struct JsonTx {
    pub dest: Box<dyn Write>,
    pub out_args: Option<OutputOpts>,
}

impl filerep::TryToIR for JsonRx {
    fn try_to_ir(self) -> Result<FileMems, filerep::FileFmtErr> {
        let mut new_mems = FileMems::default();
        let mut sr = JsonStreamReader::new(self.src);
        let mut typemap: HashMap<String, TypeSpec> = HashMap::new();
        sr.begin_object()?;

        // read types first
        let type_p = struson::json_path!["format"];
        while sr.has_next()? {
            let k = sr.next_name_owned()?;
            sr.seek_to(&type_p)?;
            let v: FormatInfo = sr.deserialize_next()?;
            sr.seek_back(&type_p)?;
            typemap.insert(k, TypeSpec::try_from(&v)?);
        }
        sr.end_object()?;

        // then, read data
        let data_p = struson::json_path!["data"];
        sr.begin_object()?;
        while sr.has_next()? {
            let k = sr.next_name_owned()?;
            sr.seek_to(&data_p)?;
            let t = typemap.remove(&k).unwrap();
            let v = crate::json::ops::try_read_arr(&mut sr, &t)?;

            new_mems.mems.insert(
                k,
                SingleMem::new(v.data, v.dim_sizes, t, Endian::Little),
            );
        }
        sr.end_object()?;

        Ok(new_mems)
    }
}

pub fn read_types<R: Read>(
    inp: R,
) -> Result<HashMap<String, TypeSpec>, FileFmtErr> {
    let mut sr = JsonStreamReader::new(inp);
    let mut typemap: HashMap<String, TypeSpec> = HashMap::new();
    sr.begin_object()?;

    // read types first
    while sr.has_next()? {
        let k = sr.next_name_owned()?;
        let type_p = struson::json_path!["format"];
        sr.seek_to(&type_p)?;
        let v: FormatInfo = sr.deserialize_next()?;
        sr.seek_back(&type_p)?;
        typemap.insert(k, TypeSpec::try_from(&v)?);
    }
    sr.end_object()?;
    Ok(typemap)
}

pub fn read_data<R: Read>(
    inp: R,
    mut t: HashMap<String, TypeSpec>,
) -> Result<FileMems, FileFmtErr> {
    let mut new_mems = FileMems::default();

    let mut sr = JsonStreamReader::new(inp);

    let data_p = struson::json_path!["data"];
    sr.begin_object()?;
    while sr.has_next()? {
        let k = sr.next_name_owned()?;
        sr.seek_to(&data_p)?;
        let t = t.remove(&k).unwrap();
        let v = crate::json::ops::try_read_arr(&mut sr, &t)?;
        sr.seek_back(&data_p)?;

        new_mems
            .mems
            .insert(k, SingleMem::new(v.data, v.dim_sizes, t, Endian::Little));
    }
    sr.end_object()?;

    Ok(new_mems)
}
