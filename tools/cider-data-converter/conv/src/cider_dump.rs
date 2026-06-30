use std::marker::PhantomData;

use cider::serialization as cs;
use cider::serialization::MemoryDeclaration;

use crate::filerep as fr;
use crate::numrep as nr;

fn as_cider_dims<T: nr::ReprType>(inp: &nr::DataSet<T>) -> cs::Dimensions {
    match inp.num_dimensions {
        1 => cs::Dimensions::D1(inp.dimensions[0]),
        2 => cs::Dimensions::D2(inp.dimensions[0], inp.dimensions[1]),
        3 => cs::Dimensions::D3(
            inp.dimensions[0],
            inp.dimensions[1],
            inp.dimensions[2],
        ),
        4 => cs::Dimensions::D4(
            inp.dimensions[0],
            inp.dimensions[1],
            inp.dimensions[2],
            inp.dimensions[3],
        ),
        _ => panic!("invalid number of dims"),
    }
}

fn as_cider_formatinfo(v: nr::ReprAs, width: u32) -> cs::FormatInfo {
    match v {
        nr::ReprAs::Bits => cs::FormatInfo::Bitnum {
            signed: false,
            width,
        },
        nr::ReprAs::Int { signed } => cs::FormatInfo::Bitnum { signed, width },
        nr::ReprAs::Float => cs::FormatInfo::IEEFloat {
            signed: true,
            width,
        },
        nr::ReprAs::Fixed { signed, exp_width } => cs::FormatInfo::Fixed {
            signed,
            int_width: exp_width,
            frac_width: width - exp_width,
        },
        _ => panic!("unknown"),
    }
}

impl<T: nr::ReprType> fr::TryFromIR<T, cs::DataDump> for cs::DataDump {
    fn try_from_ir(
        inp: &nr::DataSet<T>,
    ) -> Result<cs::DataDump, fr::FileFmtErr> {
        let mut out_res = cs::DataDump::new_empty();
        let meminfo = MemoryDeclaration::new(
            String::from("place"),
            as_cider_dims(inp),
            as_cider_formatinfo(T::repr_as(), T::WIDTH as u32),
        );
        // TODO: below should trim values to size
        out_res.push_memory(
            meminfo,
            inp.data.iter().flat_map(|e| e.to_le_bytes()),
        );
        Ok(out_res)
    }
}

impl fr::TryToIR for cs::DataDump {
    fn try_to_ir<T: nr::ReprType>(
        inp: Self,
    ) -> Result<nr::DataSet<T>, fr::FileFmtErr> {
        let first = inp.header.memories.first().unwrap();
        let byte_data = inp.get_data(&first.name).unwrap();
        assert!(byte_data.len() % T::NUM_BYTES == 0);
        let c: Result<Vec<nr::BinRep>, _> = byte_data
            .chunks(T::NUM_BYTES)
            .into_iter()
            .map(|e| T::try_from_bytes(e, T::NUM_BYTES, nr::Endian::Little))
            .collect();
        let data = c?;
        let (dimensions, num_dimensions) = match first.dimensions {
            cs::Dimensions::D1(d1) => ([d1, 0, 0, 0], 1),
            cs::Dimensions::D2(d1, d2) => ([d1, d2, 0, 0], 2),
            cs::Dimensions::D3(d1, d2, d3) => ([d1, d2, d3, 0], 3),
            cs::Dimensions::D4(d1, d2, d3, d4) => ([d1, d2, d3, d4], 4),
        };

        Ok(nr::DataSet::<T> {
            data,
            dimensions,
            num_dimensions,
            dtype: PhantomData::<T>,
            end: nr::Endian::Little,
        })
    }
}
