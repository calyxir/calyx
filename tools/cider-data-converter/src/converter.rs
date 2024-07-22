use itertools::Itertools;
use num_bigint::BigInt;
use serde_json::Number;
use std::{collections::HashMap, iter::repeat, str::FromStr};

use super::json_data::*;
use interp::serialization::*;

fn msb(width: usize) -> u8 {
    let rem = width % 8;
    1u8 << (if rem != 0 { rem - 1 } else { 7 }) // shift to the right by between 0 and 7
}

fn sign_extend_vec(mut vec: Vec<u8>, width: usize, signed: bool) -> Vec<u8> {
    let byte_count = width.div_ceil(8);
    let msb = if vec.len() < byte_count {
        0b1000_0000u8
    } else {
        msb(width)
    };

    if signed && vec.last().unwrap() & msb != 0 {
        match vec.len().cmp(&byte_count) {
            std::cmp::Ordering::Less => {
                vec.extend(
                    repeat(0b1111_1111).take(byte_count - vec.len() - 1),
                );
                vec.push(
                    0b1111_1111
                        >> (if width % 8 == 0 { 0 } else { 8 - width % 8 }),
                );
            }
            std::cmp::Ordering::Equal => {
                // chopping off the upper bits
                let mask = 0b1111_1111u8
                    >> (if width % 8 == 0 { 0 } else { 8 - width % 8 });
                *vec.last_mut().unwrap() &= mask;
            }
            std::cmp::Ordering::Greater => unreachable!(),
        }
    } else if vec.len() > byte_count {
        assert_eq!(vec.len(), byte_count + 1);
        assert_eq!(*vec.last().unwrap(), 0);
        vec.pop();
    } else {
        vec.extend(repeat(0u8).take(byte_count - vec.len()));
    }

    vec
}

pub fn convert_to_data_dump(json: &JsonData) -> DataDump {
    let mut data_dump = DataDump::new_empty();

    for (name, entry) in json.0.iter() {
        let data_vec = entry.data.parse(&entry.format).unwrap();
        let dec: MemoryDeclaration = MemoryDeclaration::new(
            name.clone(),
            data_vec.dimensions(),
            entry.format.as_data_dump_format(),
        );

        let width = dec.width();
        let signed = dec.signed();

        let data: Box<dyn Iterator<Item = u8>> = match &data_vec {
            DataVec::Id1(v1) => Box::new(v1.iter().flat_map(|val| {
                // chopping off the upper bits
                sign_extend_vec(
                    val.to_signed_bytes_le(),
                    width,
                    signed && val < &BigInt::ZERO,
                )
                .into_iter()
                .take(width.div_ceil(8))
            })),
            DataVec::Id2(v1) => Box::new(v1.iter().flat_map(|v2| {
                v2.iter().flat_map(|val| {
                    sign_extend_vec(
                        val.to_signed_bytes_le(),
                        width,
                        signed && val < &BigInt::ZERO,
                    )
                    .into_iter()
                    .take(width.div_ceil(8))
                })
            })),
            DataVec::Id3(v1) => Box::new(v1.iter().flat_map(|v2| {
                v2.iter().flat_map(|v3| {
                    v3.iter().flat_map(|val| {
                        sign_extend_vec(
                            val.to_signed_bytes_le(),
                            width,
                            signed && val < &BigInt::ZERO,
                        )
                        .into_iter()
                        .take(width.div_ceil(8))
                    })
                })
            })),
            DataVec::Id4(v1) => Box::new(v1.iter().flat_map(|v2| {
                v2.iter().flat_map(|v3| {
                    v3.iter().flat_map(|v4| {
                        v4.iter().flat_map(|val| {
                            sign_extend_vec(
                                val.to_signed_bytes_le(),
                                width,
                                signed && val < &BigInt::ZERO,
                            )
                            .into_iter()
                            .take(width.div_ceil(8))
                        })
                    })
                })
            })),
            DataVec::Fd1(_) => todo!("implement fixed-point"),
            DataVec::Fd2(_) => todo!("implement fixed-point"),
            DataVec::Fd3(_) => todo!("implement fixed-point"),
            DataVec::Fd4(_) => todo!("implement fixed-point"),
        };

        data_dump.push_memory(dec, data)
    }

    data_dump
}

fn parse_bytes(bytes: &[u8], width: usize, signed: bool) -> BigInt {
    if signed {
        let msb = msb(width);

        let mut bytes = bytes.to_vec();
        if bytes.last().unwrap() & msb != 0 {
            let rem = width % 8;
            if rem != 0 {
                let mask = 255u8 >> (8 - rem);

                *bytes.last_mut().unwrap() |= !mask;
            }
        }
        BigInt::from_signed_bytes_le(bytes.as_slice())
    } else {
        BigInt::from_bytes_le(num_bigint::Sign::Plus, bytes)
    }
}

fn format_data(declaration: &MemoryDeclaration, data: &[u8]) -> ParseVec {
    let width = declaration.width();

    let chunk_stream = data.chunks_exact(width.div_ceil(8)).map(|chunk| {
        match declaration.format {
            interp::serialization::FormatInfo::Bitnum { signed, .. } => {
                let int = parse_bytes(chunk, width, signed);
                Number::from_str(&int.to_str_radix(10)).unwrap()
            }
            interp::serialization::FormatInfo::Fixed { .. } => todo!(),
        }
    });
    // sanity check
    assert!(data.len() % width.div_ceil(8) == 0);

    match &declaration.dimensions {
        Dimensions::D1(_) => chunk_stream.collect_vec().into(),
        Dimensions::D2(_d0, d1) => chunk_stream
            .chunks(*d1)
            .into_iter()
            .map(|v| v.collect_vec())
            .collect_vec()
            .into(),
        Dimensions::D3(_d0, d1, d2) => chunk_stream
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
        Dimensions::D4(_d0, d1, d2, d3) => chunk_stream
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
        let formatted_data = format_data(declaration, data);

        map.insert(declaration.name.clone(), formatted_data);
    }

    JsonPrintDump(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn arb_format_info()(width in 1_u64..=128, signed in any::<bool>()) -> crate::json_data::FormatInfo {
            crate::json_data::FormatInfo {
                width: Some(width),
                is_signed: signed,
                numeric_type: NumericType::Bitnum,
                int_width: None,
                frac_width: None,
            }
        }
    }

    fn dim_generator() -> impl Strategy<Value = Dimensions> {
        prop_oneof![
            (1_usize..=32).prop_map(Dimensions::D1),
            (1_usize..=32, 1_usize..=32)
                .prop_map(|(d1, d2)| Dimensions::D2(d1, d2)),
            (1_usize..=16, 1_usize..=16, 1_usize..=32)
                .prop_map(|(d1, d2, d3)| Dimensions::D3(d1, d2, d3)),
            (1_usize..=16, 1_usize..=16, 1_usize..=16, 1_usize..=16)
                .prop_map(|(d1, d2, d3, d4)| Dimensions::D4(d1, d2, d3, d4)),
        ]
    }

    prop_compose! {
        fn arb_bigint(width: u64, signed: bool)(mut data in prop::collection::vec(any::<u8>(), width.div_ceil(8) as usize)) -> BigInt {
            let last = data.last_mut().unwrap();
            let mask = 0b1111_1111u8
                >> (if width % 8 == 0 { 0 } else { 8 - width % 8 });
            *last &= mask;

            if signed {
                parse_bytes(data.as_slice(), width as usize, signed)

            } else {
                BigInt::from_bytes_le(num_bigint::Sign::Plus, data.as_slice())
            }

        }
    }

    prop_compose! {
        fn arb_data(width: u64, dimensions: Dimensions, signed: bool)(data in prop::collection::vec(arb_bigint(width, signed), dimensions.size())) -> ParseVec {
            let data = data.into_iter().map(|x| Number::from_str(&x.to_str_radix(10)).unwrap());

            match dimensions {
                Dimensions::D1(_) => data.collect_vec().into(),
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
            arb_data(format.get_width(), dimensions, format.is_signed).prop_map(
                move |x| JsonDataEntry {
                    data: x,
                    format: format.clone(),
                },
            )
        })
    }

    fn arb_bigint_with_info() -> impl Strategy<Value = (BigInt, u64, bool)> {
        let width = prop_oneof![1..=128_u64];
        let signed = any::<bool>();

        (width, signed).prop_flat_map(|(width, signed)| {
            let data = arb_bigint(width, signed);
            data.prop_map(move |x| (x, width, signed))
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

        #[test]
        fn sign_extend(data in arb_bigint_with_info()) {
            let (data, width, signed) = data;
            let vec = sign_extend_vec(data.to_signed_bytes_le(), width as usize, signed);

            let parsed_back = parse_bytes(&vec, width as usize, signed);

            prop_assert_eq!(data, parsed_back);
        }

    }
}
