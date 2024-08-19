use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Debug, Deserialize, PartialEq, Clone)]
pub enum Dimensions {
    D1(usize),
    D2(usize, usize),
    D3(usize, usize, usize),
    D4(usize, usize, usize, usize),
}

impl Dimensions {
    pub fn size(&self) -> usize {
        match self {
            Dimensions::D1(d0) => *d0,
            Dimensions::D2(d0, d1) => d0 * d1,
            Dimensions::D3(d0, d1, d2) => d0 * d1 * d2,
            Dimensions::D4(d0, d1, d2, d3) => d0 * d1 * d2 * d3,
        }
    }
}

impl From<usize> for Dimensions {
    fn from(v: usize) -> Self {
        Self::D1(v)
    }
}

impl From<(usize, usize)> for Dimensions {
    fn from(v: (usize, usize)) -> Self {
        Self::D2(v.0, v.1)
    }
}

impl From<(usize, usize, usize)> for Dimensions {
    fn from(v: (usize, usize, usize)) -> Self {
        Self::D3(v.0, v.1, v.2)
    }
}

impl From<(usize, usize, usize, usize)> for Dimensions {
    fn from(v: (usize, usize, usize, usize)) -> Self {
        Self::D4(v.0, v.1, v.2, v.3)
    }
}

#[derive(Serialize, Debug, Deserialize, PartialEq, Clone)]
pub enum FormatInfo {
    Bitnum {
        signed: bool,
        width: u32,
    },
    Fixed {
        signed: bool,
        int_width: u32,
        frac_width: u32,
    },
}

impl FormatInfo {
    pub fn signed(&self) -> bool {
        match self {
            FormatInfo::Bitnum { signed, .. } => *signed,
            FormatInfo::Fixed { signed, .. } => *signed,
        }
    }

    pub fn width(&self) -> u32 {
        match self {
            FormatInfo::Bitnum { width, .. } => *width,
            FormatInfo::Fixed {
                int_width,
                frac_width,
                ..
            } => *int_width + *frac_width,
        }
    }
}

#[derive(Serialize, Debug, Deserialize, PartialEq, Clone)]
pub struct MemoryDeclaration {
    pub name: String,
    pub dimensions: Dimensions,
    pub format: FormatInfo,
}

impl MemoryDeclaration {
    pub fn new_bitnum(
        name: String,
        width: u32,
        dimensions: Dimensions,
        signed: bool,
    ) -> Self {
        Self {
            name,
            dimensions,
            format: FormatInfo::Bitnum { signed, width },
        }
    }

    pub fn new_fixed(
        name: String,
        dimensions: Dimensions,
        signed: bool,
        int_width: u32,
        frac_width: u32,
    ) -> Self {
        assert!(int_width + frac_width > 0, "width must be greater than 0");

        Self {
            name,
            dimensions,
            format: FormatInfo::Fixed {
                signed,
                int_width,
                frac_width,
            },
        }
    }

    pub fn new(
        name: String,
        dimensions: Dimensions,
        format: FormatInfo,
    ) -> Self {
        Self {
            name,
            dimensions,
            format,
        }
    }

    pub fn size(&self) -> usize {
        self.dimensions.size()
    }

    pub fn byte_count(&self) -> usize {
        self.format.width().div_ceil(8) as usize * self.dimensions.size()
    }

    pub fn width(&self) -> u32 {
        self.format.width()
    }

    pub fn signed(&self) -> bool {
        self.format.signed()
    }
}

#[derive(Serialize, Debug, Deserialize, PartialEq, Clone)]
pub struct DataHeader {
    pub top_level: String,
    pub memories: Vec<MemoryDeclaration>,
}

impl DataHeader {
    pub fn new(top_level: String, memories: Vec<MemoryDeclaration>) -> Self {
        Self {
            top_level,
            memories,
        }
    }

    pub fn data_size(&self) -> usize {
        self.memories
            .iter()
            .fold(0, |acc, mem| acc + mem.byte_count())
    }
}

#[derive(Debug, PartialEq)]
pub struct DataDump {
    pub header: DataHeader,
    pub data: Vec<u8>,
}

impl DataDump {
    /// Magic number to identify a data dump file
    const MAGIC_NUMBER: [u8; 4] = [216, 194, 228, 20];

    /// returns an empty data dump with a top level name
    pub fn new_empty_with_top_level(top_level: String) -> Self {
        Self {
            header: DataHeader {
                top_level,
                memories: vec![],
            },
            data: vec![],
        }
    }

    /// returns an empty data dump
    pub fn new_empty() -> Self {
        Self::new_empty_with_top_level("".to_string())
    }

    /// pushes a new memory into the data dump. This does not do any fancy
    /// conversion so the data must already be configured into a byte iterator.
    pub fn push_memory<T: IntoIterator<Item = u8>>(
        &mut self,
        declaration: MemoryDeclaration,
        data: T,
    ) {
        self.header.memories.push(declaration);
        self.data.extend(data);
    }

    pub fn push_reg<T: IntoIterator<Item = u8>>(
        &mut self,
        name: String,
        width: u32,
        data: T,
    ) {
        let declaration = MemoryDeclaration::new_bitnum(
            name,
            width,
            Dimensions::D1(1),
            false,
        );
        self.push_memory(declaration, data)
    }

    // TODO Griffin: handle the errors properly
    pub fn serialize(
        &self,
        writer: &mut dyn std::io::Write,
    ) -> Result<(), SerializationError> {
        let mut header_str = Vec::new();
        ciborium::ser::into_writer(&self.header, &mut header_str)?;
        writer.write_all(&Self::MAGIC_NUMBER)?;

        let len_bytes: u32 = header_str
            .len()
            .try_into()
            .expect("Header length cannot fit in u32");
        writer.write_all(&len_bytes.to_le_bytes())?;
        writer.write_all(&header_str)?;
        writer.write_all(&self.data)?;
        writer.flush()?;
        Ok(())
    }

    // TODO Griffin: handle the errors properly
    pub fn deserialize(
        reader: &mut dyn std::io::Read,
    ) -> Result<Self, SerializationError> {
        let mut magic_number = [0u8; 4];
        reader.read_exact(&mut magic_number).map_err(|e| {
            if let std::io::ErrorKind::UnexpectedEof = e.kind() {
                SerializationError::InvalidMagicNumber
            } else {
                SerializationError::IoError(e)
            }
        })?;
        if magic_number != Self::MAGIC_NUMBER {
            return Err(SerializationError::InvalidMagicNumber);
        }

        let mut raw_header_len = [0u8; 4];
        reader.read_exact(&mut raw_header_len).map_err(|e| {
            if let std::io::ErrorKind::UnexpectedEof = e.kind() {
                SerializationError::MissingHeaderLength
            } else {
                SerializationError::IoError(e)
            }
        })?;
        let header_len = u32::from_le_bytes(raw_header_len);

        let mut raw_header = vec![0u8; header_len as usize];
        reader.read_exact(&mut raw_header).map_err(|e| {
            if let std::io::ErrorKind::UnexpectedEof = e.kind() {
                SerializationError::MalformedHeader
            } else {
                SerializationError::IoError(e)
            }
        })?;
        let header: DataHeader = ciborium::from_reader(raw_header.as_slice())?;

        let mut data: Vec<u8> = Vec::with_capacity(header.data_size());

        // we could do a read_exact here instead but I opted for read_to_end
        // instead to avoid allowing incorrect/malformed data files
        let amount_read = reader.read_to_end(&mut data)?;
        if amount_read != header.data_size() {
            return Err(SerializationError::MalformedData);
        }

        Ok(DataDump { header, data })
    }

    pub fn get_data(&self, mem_name: &str) -> Option<&[u8]> {
        let mut current_base = 0_usize;
        for mem in &self.header.memories {
            if mem.name == mem_name {
                let end = current_base + mem.byte_count();
                return Some(&self.data[current_base..end]);
            } else {
                current_base += mem.byte_count();
            }
        }
        None
    }
}

/// An error struct to handle any errors generated during the deserialization process
#[derive(Debug, Error)]
pub enum SerializationError {
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error("failed to parse data header: {0}")]
    CborDeError(#[from] ciborium::de::Error<std::io::Error>),

    #[error("failed to serialize data header: {0}")]
    CborSerError(#[from] ciborium::ser::Error<std::io::Error>),

    #[error("Malformed data dump, missing header length")]
    MissingHeaderLength,

    #[error("Malformed data dump, file is too short for given header length")]
    MalformedHeader,

    #[error(
        "Malformed data dump, data section does not match header description"
    )]
    MalformedData,

    #[error("Input is not a valid data dump")]
    InvalidMagicNumber,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_dump() -> Result<(), SerializationError> {
        let header = DataHeader {
            top_level: "test".to_string(),
            memories: vec![
                MemoryDeclaration::new_bitnum(
                    "mem0".to_string(),
                    32,
                    Dimensions::D1(16),
                    false,
                ), // 64 bytes
                MemoryDeclaration::new_bitnum(
                    "mem1".to_string(),
                    4,
                    Dimensions::D1(17),
                    false,
                ), // 17 bytes
                MemoryDeclaration::new_bitnum(
                    "mem2".to_string(),
                    3,
                    Dimensions::D1(2),
                    false,
                ), // 2 bytes
                   // 83 bytes
            ],
        };

        // This was generated from random.org
        let data = vec![
            230, 165, 232, 82, 9, 111, 146, 146, 243, 18, 26, 100, 23, 45, 22,
            34, 229, 70, 32, 185, 21, 160, 237, 107, 227, 253, 174, 96, 238,
            118, 182, 23, 167, 67, 5, 76, 82, 223, 205, 190, 109, 177, 75, 15,
            216, 40, 93, 111, 231, 205, 136, 231, 193, 155, 217, 192, 120, 235,
            81, 15, 214, 225, 113, 246, 98, 212, 51, 120, 17, 112, 83, 126,
            218, 136, 0, 16, 116, 139, 213, 255, 83, 107, 112,
        ];

        let dump = DataDump { header, data };

        let mut buf = Vec::new();

        dump.serialize(&mut buf)?;
        let reparsed_dump = DataDump::deserialize(&mut buf.as_slice())?;
        assert_eq!(reparsed_dump, dump);
        Ok(())
    }

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

                assert!(header_data[cursor..cursor + mem.byte_count()]
                    .chunks_exact(bytes_per_val)
                    .remainder()
                    .is_empty());
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

    use crate::flatten::{
        flat_ir::prelude::GlobalPortIdx,
        primitives::stateful::{CombMemD1, SeqMemD1},
        structures::index_trait::IndexRef,
    };

    proptest! {
        #[test]
        fn comb_roundtrip(dump in arb_data_dump()) {
            for mem in &dump.header.memories {
                let memory_prim = CombMemD1::new_with_init(GlobalPortIdx::new(0), mem.width(), false, mem.size(), dump.get_data(&mem.name).unwrap());
                let data = memory_prim.dump_data();
                prop_assert_eq!(dump.get_data(&mem.name).unwrap(), data);
            }
        }

        #[test]
        fn seq_roundtrip(dump in arb_data_dump()) {
            for mem in &dump.header.memories {
                let memory_prim = SeqMemD1::new_with_init(GlobalPortIdx::new(0), mem.width(), false, mem.size(), dump.get_data(&mem.name).unwrap());
                let data = memory_prim.dump_data();
                prop_assert_eq!(dump.get_data(&mem.name).unwrap(), data);
            }
        }
    }
}
