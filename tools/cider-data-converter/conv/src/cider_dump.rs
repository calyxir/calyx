use std::collections::HashMap;

use cider::serialization as cs;
use cider::serialization::MemoryDeclaration;

use crate::filerep as fr;
use crate::filerep::FileFmtErr;
use crate::numrep as nr;

fn as_cider_dims(inp: &nr::SingleMem) -> cs::Dimensions {
    use cs::Dimensions as Dim;
    match inp.num_dimensions {
        1 => Dim::D1(inp.dimensions[0]),
        2 => Dim::D2(inp.dimensions[0], inp.dimensions[1]),
        3 => Dim::D3(inp.dimensions[0], inp.dimensions[1], inp.dimensions[2]),
        4 => Dim::D4(
            inp.dimensions[0],
            inp.dimensions[1],
            inp.dimensions[2],
            inp.dimensions[3],
        ),
        _ => panic!("invalid number of dims"),
    }
}

impl TryFrom<nr::TypeSpec> for cs::FormatInfo {
    type Error = fr::FileFmtErr;
    fn try_from(value: nr::TypeSpec) -> Result<Self, Self::Error> {
        use cs::FormatInfo as cider_t;

        let res = match value.class {
            nr::TypeClass::Bits => cider_t::Bitnum {
                signed: false,
                width: value.width as u32,
            },
            nr::TypeClass::Int => cider_t::Bitnum {
                signed: value.signed,
                width: value.width as u32,
            },
            nr::TypeClass::Float => cider_t::IEEFloat {
                signed: value.signed,
                width: value.width as u32,
            },
            nr::TypeClass::Fixed { exp_width } => cider_t::Fixed {
                signed: value.signed,
                int_width: exp_width as u32,
                frac_width: ((value.width) - exp_width) as u32,
            },
            _ => return Err(FileFmtErr::from("bad format")),
        };
        Ok(res)
    }
}

impl TryFrom<&cs::FormatInfo> for nr::TypeSpec {
    type Error = fr::FileFmtErr;
    fn try_from(value: &cs::FormatInfo) -> Result<Self, Self::Error> {
        use cs::FormatInfo;
        let res = match *value {
            FormatInfo::Bitnum { signed, width } => Self {
                width: width as usize,
                signed,
                class: nr::TypeClass::Int,
            },
            FormatInfo::IEEFloat { signed, width } => Self {
                width: width as usize,
                signed,
                class: nr::TypeClass::Int,
            },
            FormatInfo::Fixed {
                signed,
                int_width,
                frac_width,
            } => Self {
                width: (frac_width + int_width) as usize,
                signed,
                class: nr::TypeClass::Fixed {
                    exp_width: int_width as usize,
                },
            },
        };
        Ok(res)
    }
}

impl fr::TryFromIR for cs::DataDump {
    fn try_from_ir(
        inp: &fr::FileMems,
        _typeprops: &crate::numimpl::TypePropsMap,
    ) -> Result<cs::DataDump, fr::FileFmtErr> {
        let mut out_res = cs::DataDump::new_empty();
        for (k, v) in inp.mems.iter() {
            let t = v.ty();
            let omask = crate::util::mask_n_bits(t.width);

            let meminfo = MemoryDeclaration::new(
                k.clone(),
                as_cider_dims(v),
                t.try_into()?,
            );

            out_res.push_memory(
                meminfo,
                v.iter_data().flat_map(|e| (e & omask).to_le_bytes()),
            );
        }

        Ok(out_res)
    }
}

impl fr::TryToIR for cs::DataDump {
    fn try_to_ir(
        self,
        types: &HashMap<String, nr::TypeSpec>,
        _typeprops: &crate::numimpl::TypePropsMap,
    ) -> Result<fr::FileMems, fr::FileFmtErr> {
        let mut res = fr::FileMems {
            mems: HashMap::new(),
        };
        for mem in self.header.memories.iter() {
            let byte_data = self.get_data(&mem.name).unwrap();
            let assoc_type = types.get(&mem.name).unwrap();
            assert!(byte_data.len() % assoc_type.num_bytes() == 0);
            let c: Result<Vec<nr::BinRep>, _> = byte_data
                .chunks(assoc_type.num_bytes())
                .into_iter()
                .map(|e| {
                    crate::numimpl::try_from_bytes(
                        e,
                        assoc_type.num_bytes(),
                        assoc_type.width,
                        nr::Endian::Little,
                    )
                })
                .collect();
            let data = c?;
            let (dimensions, num_dimensions) = match mem.dimensions {
                cs::Dimensions::D1(d1) => ([d1, 0, 0, 0], 1),
                cs::Dimensions::D2(d1, d2) => ([d1, d2, 0, 0], 2),
                cs::Dimensions::D3(d1, d2, d3) => ([d1, d2, d3, 0], 3),
                cs::Dimensions::D4(d1, d2, d3, d4) => ([d1, d2, d3, d4], 4),
            };
            res.mems.insert(
                mem.name.clone(),
                nr::SingleMem::new(
                    data,
                    dimensions,
                    num_dimensions,
                    assoc_type.clone(),
                    nr::Endian::Little,
                ),
            );
        }

        Ok(res)
    }
}

impl From<cs::SerializationError> for fr::FileFmtErr {
    fn from(value: cs::SerializationError) -> Self {
        fr::FileFmtErr::from(value.to_string())
    }
}

impl fr::FileIO for cs::DataDump {
    fn read_into(
        src: Box<dyn std::io::prelude::Read>,
    ) -> Result<Self, FileFmtErr> {
        let res = cs::DataDump::deserialize(src)?;
        Ok(res)
    }
    fn write_out(
        &self,
        dest: Box<dyn std::io::prelude::Write>,
    ) -> Result<(), FileFmtErr> {
        self.serialize(dest)?;
        Ok(())
    }
}

impl fr::ExtractType for cs::DataDump {
    fn extract_types(
        &self,
    ) -> Result<HashMap<String, nr::TypeSpec>, fr::FileFmtErr> {
        let mut res = HashMap::new();
        for mem in self.header.memories.iter() {
            let new_spec = nr::TypeSpec::try_from(&mem.format)?;
            res.insert(mem.name.clone(), new_spec);
        }
        Ok(res)
    }
}

impl fr::HintedTryToIR for cs::DataDump {}
