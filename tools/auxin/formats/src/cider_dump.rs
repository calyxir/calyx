use std::collections::HashMap;

use baa::BitVecOps;
use cider::serialization as cs;

use crate::filerep as fr;
use crate::filerep::FileFmtErr;
use num_ir::memrep as nr;

use num_ir::typing::*;

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

fn try_type_to_cider(value: &TypeSpec) -> Result<cs::FormatInfo, FileFmtErr> {
    use cs::FormatInfo as cider_t;

    let res = match value.class {
        TypeClass::Bits => cider_t::Bitnum {
            signed: false,
            width: value.width as u32,
        },
        TypeClass::Int => cider_t::Bitnum {
            signed: value.signed,
            width: value.width as u32,
        },
        TypeClass::Float => cider_t::IEEFloat {
            signed: value.signed,
            width: value.width as u32,
        },
        TypeClass::Fixed { exp_mag } => {
            let frac_width = (if exp_mag <= 0 { 0 } else { exp_mag }) as u32;
            cider_t::Fixed {
                signed: value.signed,
                int_width: (value.width as u32) - frac_width,
                frac_width,
            }
        }
        _ => {
            return Err(FileFmtErr::FileSpecific(format!(
                "could not write {:?} in cider",
                value
            )));
        }
    };
    Ok(res)
}

fn try_type_from_cider(value: &cs::FormatInfo) -> Result<TypeSpec, FileFmtErr> {
    use cs::FormatInfo;
    let res = match *value {
        FormatInfo::Bitnum { signed, width } => TypeSpec {
            width: width as usize,
            signed,
            class: TypeClass::Int,
        },
        FormatInfo::IEEFloat { signed, width } => TypeSpec {
            width: width as usize,
            signed,
            class: TypeClass::Float,
        },
        FormatInfo::Fixed {
            signed,
            int_width,
            frac_width,
        } => TypeSpec {
            width: (frac_width + int_width) as usize,
            signed,
            class: TypeClass::Fixed {
                exp_mag: frac_width as i32,
            },
        },
    };
    Ok(res)
}

impl fr::TryFromIR for cs::DataDump {
    fn try_from_ir(inp: fr::FileMems) -> Result<cs::DataDump, fr::FileFmtErr> {
        let mut out_res = cs::DataDump::new_empty();
        for (k, v) in inp.mems.iter() {
            let t = v.ty();

            let meminfo = cs::MemoryDeclaration::new(
                k.to_string(),
                as_cider_dims(v),
                try_type_to_cider(t)?,
            );

            // below is exceptionally evil
            out_res.push_memory(
                meminfo,
                v.iter_data().flat_map(|e| {
                    let whole = &e.to_bytes_le();
                    whole.to_vec()
                }),
            );
        }

        Ok(out_res)
    }
}

impl fr::TryToIR for cs::DataDump {
    fn try_to_ir(
        self,
        types: &HashMap<String, TypeSpec>,
    ) -> Result<fr::FileMems, fr::FileFmtErr> {
        let mut res = fr::FileMems {
            mems: HashMap::new(),
        };
        for mem in self.header.memories.iter() {
            let byte_data = self.get_data(&mem.name).unwrap();
            let assoc_type = types.get(&mem.name).unwrap();
            assert!(byte_data.len().is_multiple_of(assoc_type.num_bytes()));
            let c: Result<Vec<baa::BitVecValue>, _> = byte_data
                .chunks(assoc_type.num_bytes())
                .map(|e| {
                    num_ir::numimpl::try_from_bytes(
                        e,
                        assoc_type.width,
                        Endian::Little,
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
                    Endian::Little,
                ),
            );
        }

        Ok(res)
    }
}

impl From<cs::SerializationError> for FileFmtErr {
    fn from(value: cs::SerializationError) -> Self {
        Self::FileSpecific(value.to_string())
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
    ) -> Result<HashMap<String, TypeSpec>, fr::FileFmtErr> {
        let mut res = HashMap::new();
        for mem in self.header.memories.iter() {
            let new_spec = try_type_from_cider(&mem.format)?;
            res.insert(mem.name.clone(), new_spec);
        }
        Ok(res)
    }
}

impl fr::HintedTryToIR for cs::DataDump {}
