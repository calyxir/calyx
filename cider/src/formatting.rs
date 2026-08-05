use fraction::Fraction;
use itertools::Itertools;
use serde::Serialize;
use std::fmt::{Debug, Display, Write};

use crate::flatten::{
    flat_ir::cell_prototype::MemoryDimensions, text_utils::Color,
};

use cider_serde::Dimensions;

use baa::{BitVecOps, BitVecValue, WidthInt};

impl From<&MemoryDimensions> for Dimensions {
    fn from(value: &MemoryDimensions) -> Self {
        match value {
            MemoryDimensions::D1 { d0_size, .. } => {
                Dimensions::D1(*d0_size as usize)
            }
            MemoryDimensions::D2 {
                d0_size, d1_size, ..
            } => Dimensions::D2(*d0_size as usize, *d1_size as usize),
            MemoryDimensions::D3 {
                d0_size,
                d1_size,
                d2_size,
                ..
            } => Dimensions::D3(
                *d0_size as usize,
                *d1_size as usize,
                *d2_size as usize,
            ),
            MemoryDimensions::D4 {
                d0_size,
                d1_size,
                d2_size,
                d3_size,
                ..
            } => Dimensions::D4(
                *d0_size as usize,
                *d1_size as usize,
                *d2_size as usize,
                *d3_size as usize,
            ),
        }
    }
}

/// A wrapper enum used during serialization.
///
/// It represents either an unsigned integer, or a signed integer and is
/// serialized as the underlying integer. This also allows mixed serialization
/// of signed and unsigned values
#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum Entry {
    U(u64),
    I(i64),
    Frac(Fraction),
    Value(BitVecValue),
}

impl AsRef<Entry> for Entry {
    fn as_ref(&self) -> &Entry {
        self
    }
}

impl From<u64> for Entry {
    fn from(u: u64) -> Self {
        Self::U(u)
    }
}

impl From<i64> for Entry {
    fn from(i: i64) -> Self {
        Self::I(i)
    }
}

impl From<Fraction> for Entry {
    fn from(f: Fraction) -> Self {
        Self::Frac(f)
    }
}

impl Entry {
    pub fn from_val_code(val: &BitVecValue, code: &PrintCode) -> Self {
        match code {
            PrintCode::Unsigned => val.to_u64().unwrap().into(),
            PrintCode::Signed => val.to_i64().unwrap().into(),
            PrintCode::UFixed(f) => {
                val.to_unsigned_fixed_point(*f).unwrap().into()
            }
            PrintCode::SFixed(f) => {
                val.to_signed_fixed_point(*f).unwrap().into()
            }
            PrintCode::Binary => Entry::Value(val.clone()),
        }
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Entry::U(v) => write!(f, "{v}"),
            Entry::I(v) => write!(f, "{v}"),
            Entry::Frac(v) => write!(f, "{v}"),
            Entry::Value(v) => write!(f, "{}", v.to_bit_str()),
        }
    }
}

impl Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PrintCode {
    Binary,
    Unsigned,
    Signed,
    UFixed(WidthInt),
    SFixed(WidthInt),
}

impl Default for PrintCode {
    fn default() -> Self {
        Self::Binary
    }
}

impl Display for PrintCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PrintCode::Binary => "\\b".stylize_print_code().to_string(),
                PrintCode::Unsigned => "\\u".stylize_print_code().to_string(),
                PrintCode::Signed => "\\s".stylize_print_code().to_string(),
                PrintCode::UFixed(n) =>
                    format!("\\u.{n}").stylize_print_code().to_string(),
                PrintCode::SFixed(n) =>
                    format!("\\s.{n}").stylize_print_code().to_string(),
            }
        )
    }
}

pub struct LazySerializable<'a> {
    format: PrintCode,
    data: LazySerializeValue<'a>,
}

impl<'a> LazySerializable<'a> {
    fn new(format: PrintCode, data: LazySerializeValue<'a>) -> Self {
        Self { format, data }
    }

    pub fn new_empty() -> Self {
        Self::new(PrintCode::default(), LazySerializeValue::Empty)
    }

    pub fn new_val(format: PrintCode, data: &'a BitVecValue) -> Self {
        Self::new(format, LazySerializeValue::Val(data))
    }

    pub fn new_array(
        format: PrintCode,
        data: &'a [BitVecValue],
        shape: Dimensions,
    ) -> Self {
        Self::new(format, LazySerializeValue::Array(data, shape))
    }

    /// attempts to format a single value within a memory at the given address.
    /// If the address is invalid, it returns None.
    pub fn format_address(&self, address: &[usize]) -> Option<String> {
        if let Some((values, dims)) = self.data.as_array() {
            let addr = dims.compute_address(address);
            addr.and_then(|x| values.get(x)).map(|v| {
                let e = Entry::from_val_code(v, &self.format);
                format!("{e}")
            })
        } else {
            None
        }
    }
}

pub enum LazySerializeValue<'a> {
    Empty,
    Val(&'a BitVecValue),
    Array(&'a [BitVecValue], Dimensions),
}

impl<'a> LazySerializeValue<'a> {
    pub fn has_state(&self) -> bool {
        !matches!(self, Self::Empty)
    }

    pub fn as_array(&self) -> Option<(&'a [BitVecValue], &Dimensions)> {
        if let Self::Array(v, d) = &self {
            Some((*v, d))
        } else {
            None
        }
    }
}

impl<'a> std::fmt::Display for LazySerializable<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.data {
            LazySerializeValue::Empty => write!(f, ""),
            LazySerializeValue::Val(v) => {
                let v = Entry::from_val_code(v, &self.format);
                write!(f, "{v}")
            }
            LazySerializeValue::Array(arr, shape) => {
                write!(
                    f,
                    "{}",
                    format_array(
                        arr.iter()
                            .map(|x| Entry::from_val_code(x, &self.format)),
                        shape
                    )
                )
            }
        }
    }
}

impl<'a> Serialize for LazySerializable<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.data {
            LazySerializeValue::Empty => serializer.serialize_unit(),
            LazySerializeValue::Val(u) => u.serialize(serializer),
            LazySerializeValue::Array(arr, shape) => {
                let arr: Vec<Entry> = arr
                    .iter()
                    .map(|v| Entry::from_val_code(v, &self.format))
                    .collect();
                if shape.is_d1() {
                    return arr.serialize(serializer);
                }
                // there's probably a better way to write this
                match shape {
                    Dimensions::D2(_d0, d1) => {
                        let mem = arr
                            .iter()
                            .chunks(*d1)
                            .into_iter()
                            .map(|x| x.into_iter().collect::<Vec<_>>())
                            .collect::<Vec<_>>();
                        mem.serialize(serializer)
                    }
                    Dimensions::D3(_d0, d1, d2) => {
                        let mem = arr
                            .iter()
                            .chunks(d1 * d2)
                            .into_iter()
                            .map(|x| {
                                x.into_iter()
                                    .chunks(*d2)
                                    .into_iter()
                                    .map(|y| y.into_iter().collect::<Vec<_>>())
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>();
                        mem.serialize(serializer)
                    }
                    Dimensions::D4(_d0, d1, d2, d3) => {
                        let mem = arr
                            .iter()
                            .chunks(d2 * d1 * d3)
                            .into_iter()
                            .map(|x| {
                                x.into_iter()
                                    .chunks(d2 * d3)
                                    .into_iter()
                                    .map(|y| {
                                        y.into_iter()
                                            .chunks(*d3)
                                            .into_iter()
                                            .map(|z| {
                                                z.into_iter()
                                                    .collect::<Vec<_>>()
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>();
                        mem.serialize(serializer)
                    }
                    Dimensions::D1(_) => unreachable!(),
                }
            }
        }
    }
}

pub fn format_row<D>(
    out_str: &mut String,
    arr: impl Iterator<Item = D>,
    mut f: impl FnMut(&mut String, D),
) {
    out_str.push('[');
    let mut is_first = true;
    for item in arr {
        if !is_first {
            out_str.push_str(", ");
        } else {
            is_first = false;
        }
        f(out_str, item);
    }
    out_str.push(']');
}

fn format_d1(
    out_str: &mut String,
    arr: impl Iterator<Item = impl AsRef<Entry>>,
) {
    format_row(out_str, arr, |out, entry| {
        let entry = entry.as_ref();
        write!(out, "{entry}").unwrap();
    });
}

fn format_d2(
    out_str: &mut String,
    arr: impl Iterator<Item = impl AsRef<Entry>>,
    d1: usize,
) {
    format_row(out_str, arr.chunks(d1).into_iter(), |out, chunks| {
        format_d1(out, chunks.into_iter());
    });
}

fn format_d3(
    out_str: &mut String,
    arr: impl Iterator<Item = impl AsRef<Entry>>,
    d1: usize,
    d2: usize,
) {
    format_row(out_str, arr.chunks(d1 * d2).into_iter(), |out, chunks| {
        format_d2(out, chunks.into_iter(), d2)
    })
}

fn format_d4(
    out_str: &mut String,
    arr: impl Iterator<Item = impl AsRef<Entry>>,
    d1: usize,
    d2: usize,
    d3: usize,
) {
    format_row(
        out_str,
        arr.chunks(d1 * d2 * d3).into_iter(),
        |out, chunks| format_d3(out, chunks.into_iter(), d2, d3),
    )
}

fn format_array(
    arr: impl Iterator<Item = impl AsRef<Entry>>,
    shape: &Dimensions,
) -> String {
    // a somewhat arbitrary guess about how many characters each entry in the
    // array will need when printed out. Used for pre-allocating the output.
    let chars_per_entry: usize = 10;

    let mut out_str = String::with_capacity(shape.size() * chars_per_entry);

    match shape {
        Dimensions::D1(_) => {
            format_d1(&mut out_str, arr);
        }
        Dimensions::D2(_d0, d1) => {
            format_d2(&mut out_str, arr, *d1);
        }
        Dimensions::D3(_d0, d1, d2) => {
            format_d3(&mut out_str, arr, *d1, *d2);
        }
        Dimensions::D4(_d0, d1, d2, d3) => {
            format_d4(&mut out_str, arr, *d1, *d2, *d3);
        }
    }

    out_str
}

#[cfg(test)]
mod test {
    use crate::flatten::{
        flat_ir::{indexes::GlobalCellIdx, prelude::GlobalPortIdx},
        primitives::{
            Primitive,
            stateful::{CombMemD1, MemConfigInfo, SeqMemD1},
        },
        structures::environment::MemoryMap,
    };
    use cider_idx::IndexRef;

    use cider_serde::*;
    use proptest::prelude::*;

    prop_compose! {
        fn arb_memory_declaration()(name in any::<String>(), signed in any::<bool>(), width in 1_u32..=256, size in 1_usize..=500) -> MemoryDeclaration {
            MemoryDeclaration::new_bitnum(name.to_string(), width, Dimensions::D1(size), signed)
        }
    }

    prop_compose! {
        fn arb_data_header()(
            top_level in any::<String>(),
            mut memories in prop::collection::vec(arb_memory_declaration(), 1..3)
        ) -> DataHeader {
            // This is a silly hack to force unique names for the memories
            for (i, memory) in memories.iter_mut().enumerate() {
                memory.name = format!("{}_{i}", memory.name);
            }

            DataHeader { top_level, memories }
        }
    }

    prop_compose! {
        fn arb_data(size: usize)(
            data in prop::collection::vec(0u8..=255, size)
        )  -> Vec<u8> {
            data
        }
    }

    fn arb_data_dump() -> impl Strategy<Value = DataDump> {
        let data = arb_data_header().prop_flat_map(|header| {
            let data = arb_data(header.data_size());
            (Just(header), data)
        });

        data.prop_map(|(header, mut header_data)| {
            let mut cursor = 0_usize;
            // Need to go through the upper byte of each value in the memory to
            // remove any 1s in the padding region since that causes the memory
            // produced from the memory primitive to not match the one
            // serialized into it in the first place
            for mem in &header.memories {
                let bytes_per_val = mem.width().div_ceil(8) as usize;
                let rem = mem.width() % 8;
                let mask = if rem != 0 { 255u8 >> (8 - rem) } else { 255_u8 };

                for bytes in &mut header_data[cursor..cursor + mem.byte_count()]
                    .chunks_exact_mut(bytes_per_val)
                {
                    *bytes.last_mut().unwrap() &= mask;
                }

                assert!(
                    header_data[cursor..cursor + mem.byte_count()]
                        .chunks_exact(bytes_per_val)
                        .remainder()
                        .is_empty()
                );
                cursor += mem.byte_count();
            }

            DataDump {
                header,
                data: header_data,
            }
        })
    }

    proptest! {
        #[test]
        fn prop_roundtrip(dump in arb_data_dump()) {
            let mut buf = Vec::new();
            dump.serialize(&mut buf)?;

            let reparsed_dump = DataDump::deserialize(&mut buf.as_slice())?;
            prop_assert_eq!(dump, reparsed_dump)

        }
    }

    proptest! {
        #[test]
        fn comb_roundtrip(dump in arb_data_dump()) {
            let mut state_map = MemoryMap::new();
            for mem in &dump.header.memories {
                let memory_config = MemConfigInfo::new(GlobalPortIdx::new(0), GlobalCellIdx::new(0), mem.width(), false, mem.size());
                let memory_prim = CombMemD1::new_with_init(memory_config, dump.get_data(&mem.name).unwrap(), &mut None, &mut state_map);
                let data = memory_prim.serializer().unwrap().dump_data(&state_map);
                prop_assert_eq!(dump.get_data(&mem.name).unwrap(), data);
            }
        }

        #[test]
        fn seq_roundtrip(dump in arb_data_dump()) {
            let mut state_map = MemoryMap::new();

            for mem in &dump.header.memories {
                let memory_config = MemConfigInfo::new(GlobalPortIdx::new(0), GlobalCellIdx::new(0), mem.width(), false, mem.size());
                let memory_prim = SeqMemD1::new_with_init(memory_config, dump.get_data(&mem.name).unwrap(), &mut None, &mut state_map);
                let data = memory_prim.serializer().unwrap().dump_data(&state_map);
                prop_assert_eq!(dump.get_data(&mem.name).unwrap(), data);
            }
        }
    }
}
