use baa::BitVecOps;
use cider_serde as cs;

use crate::filerep::*;
use num_ir::memrep as nr;

use num_ir::typing::*;
use smallvec::smallvec;

#[derive(Debug, thiserror::Error)]
pub enum CiderErr {
    #[error("lib cider_serde: {0}")]
    Serde(#[from] cs::SerializationError),

    #[error("can't represent format {0}")]
    Format(String),

    #[error("numparse: {0}")]
    NumParse(#[from] NumParseErr),
}

pub(crate) fn as_cider_dims(inp: &nr::SingleMem) -> cs::Dimensions {
    use cs::Dimensions as Dim;
    match inp.dimensions.len() {
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

pub(crate) fn try_type_to_cider(
    value: &TypeSpec,
) -> Result<cs::FormatInfo, CiderErr> {
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
            return Err(CiderErr::Format(format!("{:?}", value)));
        }
    };
    Ok(res)
}

pub(crate) fn try_type_from_cider(
    value: &cs::FormatInfo,
) -> Result<TypeSpec, CiderErr> {
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

pub struct CiderHandler;

impl FileStore for CiderHandler {
    type Err = CiderErr;

    fn write<W: std::io::Write>(
        &self,
        inp: MemsMap,
        dest: W,
    ) -> Result<(), CiderErr> {
        let mut out_res = cs::DataDump::new_empty();
        for (k, v) in inp.mems.into_iter() {
            let t = v.ty();

            let meminfo = cs::MemoryDeclaration::new(
                k.to_string(),
                as_cider_dims(&v),
                try_type_to_cider(t)?,
            );

            out_res.push_memory(
                meminfo,
                v.iter_data().flat_map(|e| e.to_bytes_le()),
            );
        }
        out_res.serialize(dest)?;
        Ok(())
    }
    fn read_stream<R: std::io::Read>(
        &self,
        src: R,
    ) -> Result<MemsMap, CiderErr> {
        let dump = cs::DataDump::deserialize(src)?;
        let mut res = MemsMap::default();
        for mem in dump.header.memories.iter() {
            let byte_data = dump.get_data(&mem.name).unwrap();
            let assoc_type = try_type_from_cider(&mem.format)?;

            debug_assert!(
                byte_data.len().is_multiple_of(assoc_type.num_bytes())
            );
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
            let dimensions = match mem.dimensions {
                cs::Dimensions::D1(d1) => smallvec![d1],
                cs::Dimensions::D2(d1, d2) => smallvec![d1, d2],
                cs::Dimensions::D3(d1, d2, d3) => smallvec![d1, d2, d3],
                cs::Dimensions::D4(d1, d2, d3, d4) => smallvec![d1, d2, d3, d4],
            };
            res.mems.insert(
                mem.name.clone(),
                nr::SingleMem::new(
                    data,
                    dimensions,
                    assoc_type.clone(),
                    Endian::Little,
                ),
            );
        }

        Ok(res)
    }
}
