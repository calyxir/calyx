//! implements the [crate::filerep::FileStore] trait for [JsonHandler]

use baa::BitVecValue;
use std::collections::HashMap;
use std::io::{BufRead, Read, Seek, Write};

use crate::filerep::*;
use crate::json_common::*;
use num_ir::memrep::*;
use num_ir::typing::*;

use smallvec::SmallVec;

use struson::{
    reader::*,
    writer::{JsonStreamWriter, JsonWriter},
};

#[derive(Default)]
struct ArrayReader {
    data: Vec<BitVecValue>,
    dim_sizes: SmallVec<[usize; 4]>,
    is_quoted: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum JsonErr {
    #[error("error reading array at {0:?}")]
    ArrayRead(struson::reader::JsonReaderPosition),

    #[error(transparent)]
    NumIR(#[from] NumParseErr),

    #[error(transparent)]
    Typing(#[from] JsonTypeError),
    #[error(transparent)]
    JsonRead(#[from] struson::reader::ReaderError),

    #[error(transparent)]
    JsonNum(#[from] struson::writer::JsonNumberError),

    #[error(transparent)]
    JsonSer(#[from] struson::serde::SerializerError),

    #[error(transparent)]
    JsonDe(#[from] struson::serde::DeserializerError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[inline(always)]
fn arr_err<R: Read>(r: &JsonStreamReader<R>) -> JsonErr {
    JsonErr::ArrayRead(r.current_position(false))
}

// TODO: can use number of elements in dim, once known, as guiding assumption
// we can do this a lot more unsafely / maybe faster by trying to read ``n`` elements of (current level's type) once we know the type and the number of elements in the level
impl ArrayReader {
    fn read_arr_helper<R: Read>(
        &mut self,
        r: &mut JsonStreamReader<R>,
        ty: &TypeSpec,
        curr_dim: usize,
    ) -> Result<(), JsonErr> {
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
                return Err(arr_err(r));
            }

            match nt {
                ValueType::Array => {
                    self.read_arr_helper(r, ty, curr_dim + 1)?
                }
                ValueType::String => {
                    let s = r.next_str()?;
                    let parsed = ty.read_str(s, Endian::Little)?;
                    self.data.push(parsed)
                }
                ValueType::Number => {
                    let num_s = r.next_number_as_str()?;
                    let parsed = ty.read_str(num_s, Endian::Little)?;
                    self.data.push(parsed)
                }
                _ => return Err(arr_err(r)),
            };
            item_ct += 1;
        }
        if self.dim_sizes[curr_dim] == 0 {
            self.dim_sizes[curr_dim] = item_ct
        } else if self.dim_sizes[curr_dim] != item_ct {
            return Err(arr_err(r));
        }
        if matches!(curr_type, Some(ValueType::String)) {
            self.is_quoted = true;
        }
        r.end_array()?;
        Ok(())
    }
}

/// Attempt to read only the array portion (i.e. the 'value') part of a fud2 data entry.
///
/// assumes that reader is at the start of the array, and the key has already been consumed.
fn try_read_arr<R: Read>(
    r: &mut JsonStreamReader<R>,
    ty: &TypeSpec,
) -> Result<ArrayReader, JsonErr> {
    let mut a = ArrayReader::default();
    a.read_arr_helper(r, ty, 0)?;
    debug_assert_eq!(a.dim_sizes.iter().product::<usize>(), a.data.len());
    Ok(a)
}

/// Attempt to write a [SingleMem] as a object containing both ``format`` and ``data``.
fn try_write_obj<W: Write>(
    m: SingleMem,
    w: &mut JsonStreamWriter<W>,
    is_hex: bool,
) -> Result<(), JsonErr> {
    let is_bin = m.ty().class == TypeClass::Bits;

    w.begin_object()?;
    w.name("data")?;

    debug_assert_eq!(m.size(), m.dimensions.iter().product::<usize>());

    // when writing out json data, only do so in a 1D array.
    w.begin_array()?;

    for el in m.iter_data() {
        let s_to_write = if is_hex {
            m.ty().write_hexstring(el, Endian::Little)
        } else {
            m.ty().write_string(el, Endian::Little)
        };
        if is_bin || is_hex {
            w.string_value(&s_to_write)?;
        } else {
            w.number_value_from_string(&s_to_write)?;
        }
    }
    w.end_array()?;

    w.name("format")?;
    let json_t = FormatInfo::try_from(m.ty())?;

    w.serialize_value(&json_t)?;
    w.end_object()?;

    Ok(())
}

// if more options exist in the future, we could split JsonReader / JsonWriter.
pub struct JsonHandler {
    /// whether to output results as hexstrings, regardless of specified type.
    pub out_hex: bool,
}

impl FileStore for JsonHandler {
    type Err = JsonErr;

    /// this implementation is particularly slow, avoid if performance is necessary.
    /// because we use a streaming JSON library, and need to read a file two times, [JsonHandler::read_stream] copies ``stdin`` to a buffer.  this naturally introduces overhead.
    fn read_stream<R: BufRead>(
        &self,
        mut handle: R,
    ) -> Result<MemsMap, Self::Err> {
        let mut stdin_buf: Vec<u8> = Vec::new();
        let mut linebuf = String::with_capacity(20);
        while handle.read_line(&mut linebuf)? != 0 {
            let cleaned = linebuf.trim();
            stdin_buf.extend_from_slice(cleaned.as_bytes());
            linebuf.clear();
        }
        let typemap = read_types(&mut stdin_buf.as_slice())?;
        read_data(stdin_buf.as_slice(), typemap)
    }

    fn read_filelike<R: BufRead + Seek>(
        &self,
        src: R,
    ) -> Result<MemsMap, Self::Err> {
        let mut handle = src;
        let typemap = read_types(&mut handle)?;
        handle.rewind()?;
        read_data(handle, typemap)
    }

    fn write<W: Write>(&self, inp: MemsMap, dest: W) -> Result<(), Self::Err> {
        use struson::writer::WriterSettings;
        let mut sw = JsonStreamWriter::new_custom(
            dest,
            WriterSettings {
                pretty_print: true,
                ..Default::default()
            },
        );
        sw.begin_object()?;
        for (mem_name, mem) in inp.mems.into_iter() {
            sw.name(&mem_name)?;
            try_write_obj(mem, &mut sw, self.out_hex)?;
        }
        sw.end_object()?;

        Ok(())
    }
}

// TODO: rather than seeks, can we do skips?
/// Read the "format" keys and values within a ``fud2`` .data file.
///
/// Because JSON does not require any key ordering, we read the whole file twice. Types are read first. [read_data] then uses the type information to convert numerical data into [SingleMem], without first copying the strings out.
pub fn read_types<R: Read>(
    inp: R,
) -> Result<HashMap<String, TypeSpec>, JsonErr> {
    let mut sr = JsonStreamReader::new(inp);
    let mut typemap: HashMap<String, TypeSpec> = HashMap::new();
    sr.begin_object()?;

    // read types first
    let type_p = struson::json_path!["format"];
    while sr.has_next()? {
        let mem_name = sr.next_name_owned()?;
        sr.seek_to(&type_p)?;
        let v: FormatInfo = sr.deserialize_next()?;
        sr.seek_back(&type_p)?;
        typemap.insert(mem_name, TypeSpec::try_from(&v)?);
    }
    sr.end_object()?;
    Ok(typemap)
}

/// Read the "data" keys and values within a ``fud2`` .data file.
///
/// See [read_types] for details on usage.
pub fn read_data<R: Read>(
    inp: R,
    mut t: HashMap<String, TypeSpec>,
) -> Result<MemsMap, JsonErr> {
    let mut new_mems = MemsMap::default();

    let mut sr = JsonStreamReader::new(inp);

    let data_p = struson::json_path!["data"];
    sr.begin_object()?;
    while sr.has_next()? {
        let mem_name = sr.next_name_owned()?;
        sr.seek_to(&data_p)?;
        let mem_ty = t.remove(&mem_name).unwrap();
        let v = try_read_arr(&mut sr, &mem_ty)?;
        sr.seek_back(&data_p)?;

        new_mems.mems.insert(
            mem_name,
            SingleMem::new(v.data, v.dim_sizes, mem_ty, Endian::Little),
        );
    }
    sr.end_object()?;

    Ok(new_mems)
}
