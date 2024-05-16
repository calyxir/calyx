use itertools::Itertools;
use std::collections::HashMap;

use super::json_data::*;
use interp::serialization::data_dump::*;

pub fn convert_to_data_dump(json: &JsonData) -> DataDump {
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
            entry.data.dimensions(),
            data,
        )
    }

    data_dump
}

fn format_data(dimension: &Dimensions, data: &[u8], width: usize) -> DataVec {
    assert!(width.div_ceil(8) <= 8, "cannot fit in u64");
    let u64_stream = data.chunks_exact(width.div_ceil(8)).map(|chunk| {
        let mut array = [0u8; 8];
        array[0..chunk.len()].copy_from_slice(chunk);
        u64::from_le_bytes(array)
    });
    // sanity check
    assert!(data.len() % width.div_ceil(8) == 0);

    match dimension {
        Dimensions::D1(_) => u64_stream.collect_vec().into(),
        Dimensions::D2(_d0, d1) => u64_stream
            .chunks(*d1)
            .into_iter()
            .map(|v| v.collect_vec())
            .collect_vec()
            .into(),
        Dimensions::D3(_d0, d1, d2) => u64_stream
            .chunks(d1 * d2)
            .into_iter()
            .map(|v1| {
                v1.chunks(*d2)
                    .into_iter()
                    .map(|v2| v2.collect_vec())
                    .collect_vec()
            })
            .collect_vec()
            .into(),
        Dimensions::D4(_d0, d1, d2, d3) => u64_stream
            .chunks(d1 * d2 * d3)
            .into_iter()
            .map(|v1| {
                v1.chunks(d2 * d3)
                    .into_iter()
                    .map(|v2| {
                        v2.chunks(*d3)
                            .into_iter()
                            .map(|v3| v3.collect_vec())
                            .collect_vec()
                    })
                    .collect_vec()
            })
            .collect_vec()
            .into(),
    }
}

pub fn convert_from_data_dump(dump: &DataDump) -> JsonPrintDump {
    let mut map = HashMap::new();
    for declaration in &dump.header.memories {
        let data = dump.get_data(&declaration.name).unwrap();
        let formatted_data = format_data(
            &declaration.dimensions,
            data,
            declaration.width.into(),
        );

        map.insert(declaration.name.clone(), formatted_data);
    }

    JsonPrintDump(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn arb_format_info()(width in 1_u64..=64) -> FormatInfo {
            FormatInfo {
                width,
                is_signed: false,
                numeric_type: NumericType::Bitnum,
                int_width: None,
            }
        }
    }

    fn max_val(width: u64) -> u64 {
        assert!(width <= 64);
        2u64.saturating_pow(width.try_into().unwrap()) - 1
    }

    fn dim_generator() -> impl Strategy<Value = Dimensions> {
        prop_oneof![
            (1_usize..=32).prop_map(Dimensions::D1),
            (1_usize..=32, 1_usize..=32)
                .prop_map(|(d1, d2)| Dimensions::D2(d1, d2)),
            (1_usize..=32, 1_usize..=32, 1_usize..=32)
                .prop_map(|(d1, d2, d3)| Dimensions::D3(d1, d2, d3)),
            (1_usize..=32, 1_usize..=32, 1_usize..=32, 1_usize..=32)
                .prop_map(|(d1, d2, d3, d4)| Dimensions::D4(d1, d2, d3, d4)),
        ]
    }

    prop_compose! {
        fn arb_data(width: u64, dimensions: Dimensions)(data in prop::collection::vec(0u64..max_val(width), dimensions.size())) -> DataVec {
            match dimensions {
                Dimensions::D1(_) => data.into(),
                Dimensions::D2(_d0, d1) => data.into_iter().chunks(d1).into_iter().map(|v| v.collect_vec()).collect_vec().into(),
                Dimensions::D3(_d0, d1, d2) => data.into_iter().chunks(d1 * d2).into_iter().map(|v1| v1.chunks(d2).into_iter().map(|v2| v2.collect_vec()).collect_vec()).collect_vec().into(),
                Dimensions::D4(_d0, d1, d2, d3) => data.into_iter().chunks(d1 * d2 * d3).into_iter().map(|v1| v1.chunks(d2 * d3).into_iter().map(|v2| v2.chunks(d3).into_iter().map(|v3| v3.collect_vec()).collect_vec()).collect_vec()).collect_vec().into(),
            }
        }
    }

    fn arb_json_entry() -> impl Strategy<Value = JsonDataEntry> {
        let arb_format_info = arb_format_info();
        let dim = dim_generator();
        (arb_format_info, dim).prop_flat_map(|(format, dimensions)| {
            arb_data(format.width, dimensions).prop_map(move |x| {
                JsonDataEntry {
                    data: x,
                    format: format.clone(),
                }
            })
        })
    }

    proptest! {
        #[test]
        fn test_json_roundtrip(map in prop::collection::hash_map(any::<String>(), arb_json_entry(), 1..4)) {
            let json_data = JsonData(map);

            let dump = convert_to_data_dump(&json_data);

            let json_print_dump = convert_from_data_dump(&dump);

            for (name, entry) in &json_data.0 {
                prop_assert_eq!(&entry.data, json_print_dump.0.get(name).unwrap())
            }
        }

    }
}
