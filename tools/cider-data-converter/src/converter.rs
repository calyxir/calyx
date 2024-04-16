use super::json_data::*;
use interp::serialization::data_dump::*;

pub fn convert_to_data_dump(json: JsonData) -> DataDump {
    let mut data_dump = DataDump::new_empty();

    for (name, entry) in json.0.iter() {
        let width = &entry.format.width;
        let data: Box<dyn Iterator<Item = u8>> = match &entry.data {
            DataVec::Id1(v1) => Box::new(v1.iter().flat_map(|val| {
                // chopping off the upper bits
                val.to_le_bytes()
                    .into_iter()
                    .take(width.div_ceil(8) as usize)
            })),
            DataVec::Id2(v1) => Box::new(v1.iter().flat_map(|v2| {
                v2.iter().flat_map(|val| {
                    val.to_le_bytes()
                        .into_iter()
                        .take(width.div_ceil(8) as usize)
                })
            })),
            DataVec::Id3(v1) => Box::new(v1.iter().flat_map(|v2| {
                v2.iter().flat_map(|v3| {
                    v3.iter().flat_map(|val| {
                        val.to_le_bytes()
                            .into_iter()
                            .take(width.div_ceil(8) as usize)
                    })
                })
            })),
            DataVec::Id4(v1) => Box::new(v1.iter().flat_map(|v2| {
                v2.iter().flat_map(|v3| {
                    v3.iter().flat_map(|v4| {
                        v4.iter().flat_map(|val| {
                            val.to_le_bytes()
                                .into_iter()
                                .take(width.div_ceil(8) as usize)
                        })
                    })
                })
            })),
            DataVec::Fd1(_) => todo!("implement fixed-point"),
            DataVec::Fd2(_) => todo!("implement fixed-point"),
            DataVec::Fd3(_) => todo!("implement fixed-point"),
            DataVec::Fd4(_) => todo!("implement fixed-point"),
        };

        data_dump.push_memory(
            name.clone(),
            *width as usize,
            entry.data.size(),
            data,
        )
    }

    todo!()
}
