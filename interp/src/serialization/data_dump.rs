use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Deserialize, PartialEq, Clone)]
pub struct MemoryDeclaration {
    pub name: String,
    pub width: usize,
    pub size: usize,
}

impl MemoryDeclaration {
    pub fn new(name: String, width: usize, size: usize) -> Self {
        Self { name, width, size }
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
        let mut size = 0;

        for mem in &self.memories {
            let width_bytes = mem.width.div_ceil(8);
            size += width_bytes * mem.size;
        }

        size
    }
}

#[derive(Debug, PartialEq)]
pub struct DataDump {
    pub header: DataHeader,
    pub data: Vec<u8>,
}

impl DataDump {
    // TODO Griffin: handle the errors properly
    pub fn serialize(&self, writer: &mut dyn std::io::Write) {
        let header_str = serde_json::to_string(&self.header).unwrap();
        let len_bytes = header_str.len();
        let written = writer.write(&len_bytes.to_le_bytes()).unwrap();
        assert_eq!(written, 8);
        write!(writer, "{}", header_str).unwrap();

        let written = writer.write(&self.data).unwrap();
        assert_eq!(written, self.data.len());
    }

    /// TODO Griffin: handle the errors properly
    pub fn deserialize(reader: &mut dyn std::io::Read) -> Self {
        let mut raw_header_len = [0u8; 8];
        reader.read_exact(&mut raw_header_len).unwrap();
        let header_len = usize::from_le_bytes(raw_header_len);

        let mut raw_header = vec![0u8; header_len];
        reader.read_exact(&mut raw_header).unwrap();
        let header_str = String::from_utf8(raw_header).unwrap();
        let header: DataHeader = serde_json::from_str(&header_str).unwrap();
        let mut data: Vec<u8> = Vec::with_capacity(header.data_size());

        // we could do a read_exact here instead but I opted for read_to_end
        // instead to avoid allowing incorrect/malformed data files
        let amount_read = reader.read_to_end(&mut data).unwrap();
        assert_eq!(amount_read, header.data_size());

        DataDump { header, data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_dump() {
        let header = DataHeader {
            top_level: "test".to_string(),
            memories: vec![
                MemoryDeclaration::new("mem0".to_string(), 32, 16), // 64 bytes
                MemoryDeclaration::new("mem1".to_string(), 4, 17),  // 17 bytes
                MemoryDeclaration::new("mem2".to_string(), 3, 2),   // 2 bytes
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

        dump.serialize(&mut buf);
        let reparsed_dump = DataDump::deserialize(&mut buf.as_slice());
        assert_eq!(reparsed_dump, dump);
    }

    use proptest::prelude::*;

    prop_compose! {
        fn arb_memory_declaration()(name in any::<String>(), width in 1_usize..=128, size in 1_usize..=1024) -> MemoryDeclaration {
            MemoryDeclaration::new(name.to_string(), width, size)
        }
    }

    prop_compose! {
        fn arb_data_header()(
            top_level in any::<String>(),
            memories in prop::collection::vec(arb_memory_declaration(), 1..5)
        ) -> DataHeader {
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

        data.prop_map(|(header, data)| DataDump { header, data })
    }

    proptest! {
        #[test]
        fn prop_roundtrip(dump in arb_data_dump()) {
            let mut buf = Vec::new();
            dump.serialize(&mut buf);

            let reparsed_dump = DataDump::deserialize(&mut buf.as_slice());
            prop_assert_eq!(dump, reparsed_dump)

        }
    }
}
