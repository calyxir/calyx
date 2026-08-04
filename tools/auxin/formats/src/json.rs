use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use struson::writer::WriterSettings;

use crate::filerep::{self, FileFmtErr, FileMems};
use crate::json_common::*;
use num_ir::memrep::*;
use num_ir::typing::*;

use struson::{
    reader::*,
    writer::{JsonStreamWriter, JsonWriter},
};

pub struct JsonEntry {
    data: Vec<String>,
    format: Option<FormatInfo>,
    dim_sizes: Vec<usize>,
    is_quoted: bool,
}

impl Default for JsonEntry {
    fn default() -> Self {
        Self {
            data: Vec::with_capacity(10),
            format: None,
            dim_sizes: Vec::with_capacity(4),
            is_quoted: false,
        }
    }
}

// NOTE: expects elements to be ordered "format" "data"
impl JsonEntry {
    fn try_read<R: Read>(
        r: &mut JsonStreamReader<R>,
    ) -> Result<Self, FileFmtErr> {
        r.begin_object()?;
        let mut res = JsonEntry::default();
        let e1 = r.next_name()?;
        if e1 != "data" {
            return Err(json_err(format!("expected data. got {}", e1)));
        }
        res.read_arr(r, 0)?;
        let e2 = r.next_name()?;
        if e2 != "format" {
            return Err(json_err(format!("expected format. got {}", e2)));
        }
        res.format = r.deserialize_next()?;

        r.end_object()?;
        Ok(res)
    }

    fn try_write<W: Write>(
        mut self,
        w: &mut JsonStreamWriter<W>,
    ) -> Result<(), FileFmtErr> {
        w.begin_object()?;
        w.name("data")?;
        let f = std::mem::take(&mut self.format).unwrap();
        self.write_nested_inner(w)?;
        w.name("format")?;
        w.serialize_value(&f)?;
        w.end_object()?;

        Ok(())
    }

    /// assumes that we are at the start of the array, no key attached to it
    fn read_arr<R: Read>(
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
                ValueType::Array => self.read_arr(r, curr_dim + 1)?,
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
        if matches!(curr_type, Some(ValueType::String)) {
            self.is_quoted = true;
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
    ) -> Result<(), FileFmtErr> {
        debug_assert!(
            self.data.len() == self.dim_sizes.iter().product::<usize>()
        );
        w.begin_array()?;
        match self.dim_sizes.len() {
            1 => {
                write_arr_from_iterable(self.data.iter(), w, self.is_quoted)?;
            }
            2 => {
                let d1_size = self.dim_sizes[1];

                for d1_chunk in self.data.chunks(d1_size) {
                    write_arr_from_iterable(
                        d1_chunk.iter(),
                        w,
                        self.is_quoted,
                    )?;
                }
            }
            3 => {
                let d1_size: usize = self.dim_sizes[1..3].iter().product();
                let d2_size = self.dim_sizes[2];
                for d1_chunk in self.data.chunks(d1_size) {
                    w.begin_array()?;
                    for d2_chunk in d1_chunk.chunks(d2_size) {
                        write_arr_from_iterable(
                            d2_chunk.iter(),
                            w,
                            self.is_quoted,
                        )?;
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
                                self.is_quoted,
                            )?;
                        }
                        w.end_array()?;
                    }

                    w.end_array()?;
                }
            }
            _ => {
                return Err(json_err("cannot write an array of >4 dimensions"));
            }
        }
        w.end_array()?;

        Ok(())
    }

    fn try_entry_to_ir(
        self,
        t: &TypeSpec,
    ) -> Result<SingleMem, filerep::FileFmtErr> {
        let data: Vec<_> = self
            .data
            .iter()
            .map(|e| t.read_str(e, Endian::Little))
            .collect::<Result<_, _>>()?;

        let mut d = [0; 4];
        for (idx, v) in self.dim_sizes.iter().enumerate() {
            d[idx] = *v;
        }

        // TODO: get this working again
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
            format: Some(FormatInfo::try_from(inp.ty())?),
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

fn write_arr_from_iterable<'a, W: Write>(
    i: impl Iterator<Item = &'a String>,
    w: &mut JsonStreamWriter<W>,
    are_str: bool,
) -> Result<(), FileFmtErr> {
    w.begin_array()?;

    for el in i {
        if are_str {
            w.string_value(el).map_err(json_err)?;
        } else {
            w.number_value_from_string(el).map_err(json_err)?;
        }
    }
    w.end_array()?;
    Ok(())
}

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
            let v = JsonEntry::try_read(&mut sr)?;

            res.insert(k, v);
        }
        sr.end_object()?;

        Ok(JsonData(res))
    }
    fn write_out(
        self,
        dest: Box<dyn std::io::Write>,
    ) -> Result<(), FileFmtErr> {
        let mut sw = JsonStreamWriter::new_custom(
            dest,
            WriterSettings {
                pretty_print: true,
                ..Default::default()
            },
        );
        sw.begin_object()?;
        for (k, v) in self.0.into_iter() {
            sw.name(&k)?;
            v.try_write(&mut sw)?;
        }
        sw.end_object()?;

        Ok(())
    }
}

impl filerep::ExtractType for JsonData {
    fn extract_types(&self) -> Result<HashMap<String, TypeSpec>, FileFmtErr> {
        let mut res = HashMap::new();
        for (k, v) in self.0.iter() {
            let Some(ref t) = v.format else {
                return Err(json_err(format!("{} has no type", k)));
            };
            let new_spec = TypeSpec::try_from(t)?;
            res.insert(k.clone(), new_spec);
        }
        Ok(res)
    }
}

impl filerep::HintedTryToIR for JsonData {}
