use itertools::Itertools;
use num_bigint::{BigInt, BigUint, ToBigInt};
use num_rational::BigRational;
use num_traits::{sign::Signed, Num, ToPrimitive};
use serde_json::Number;
use std::{collections::HashMap, iter::repeat, str::FromStr};

use super::json_data::*;
use interp::serialization::*;

fn msb(width: u32) -> u8 {
    let rem = width % 8;
    1u8 << (if rem != 0 { rem - 1 } else { 7 }) // shift to the right by between 0 and 7
}

fn sign_extend_vec(mut vec: Vec<u8>, width: u32, signed: bool) -> Vec<u8> {
    let byte_count = width.div_ceil(8) as usize;
    let msb = if vec.len() < byte_count {
        0b1000_0000u8
    } else {
        msb(width)
    };

    if signed && vec.last().unwrap() & msb != 0 {
        match vec.len().cmp(&(byte_count)) {
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

pub fn convert_to_data_dump(json: &JsonData, round_float: bool) -> DataDump {
    let mut data_dump = DataDump::new_empty();

    for (name, entry) in json.0.iter() {
        let data_vec = entry.data.parse(&entry.format).unwrap();
        let format = entry.format.as_data_dump_format();
        let dec: MemoryDeclaration = MemoryDeclaration::new(
            name.clone(),
            data_vec.dimensions(),
            format.clone(),
        );

        let width = dec.width();
        let signed = dec.signed();

        let data: Box<dyn Iterator<Item = u8>> = match &data_vec {
            DataVec::Id1(v1) => Box::new(v1.iter().flat_map(|val| {
                // chopping off the upper bits
                unroll_bigint(val, width, signed)
            })),
            DataVec::Id2(v1) => Box::new(v1.iter().flat_map(|v2| {
                v2.iter().flat_map(|val| unroll_bigint(val, width, signed))
            })),
            DataVec::Id3(v1) => Box::new(v1.iter().flat_map(|v2| {
                v2.iter().flat_map(|v3| {
                    v3.iter().flat_map(|val| unroll_bigint(val, width, signed))
                })
            })),
            DataVec::Id4(v1) => Box::new(v1.iter().flat_map(|v2| {
                v2.iter().flat_map(|v3| {
                    v3.iter().flat_map(|v4| {
                        v4.iter()
                            .flat_map(|val| unroll_bigint(val, width, signed))
                    })
                })
            })),
            DataVec::Fd1(v1) => Box::new(
                v1.iter()
                    .flat_map(|val| unroll_float(*val, &format, round_float)),
            ),
            DataVec::Fd2(v1) => Box::new(v1.iter().flat_map(|v2| {
                v2.iter()
                    .flat_map(|val| unroll_float(*val, &format, round_float))
            })),
            DataVec::Fd3(v1) => Box::new(v1.iter().flat_map(|v2| {
                v2.iter().flat_map(|v3| {
                    v3.iter().flat_map(|val| {
                        unroll_float(*val, &format, round_float)
                    })
                })
            })),
            DataVec::Fd4(v1) => Box::new(v1.iter().flat_map(|v2| {
                v2.iter().flat_map(|v3| {
                    v3.iter().flat_map(|v4| {
                        v4.iter().flat_map(|val| {
                            unroll_float(*val, &format, round_float)
                        })
                    })
                })
            })),
        };

        data_dump.push_memory(dec, data)
    }

    data_dump
}

#[inline]
fn unroll_bigint(
    val: &BigInt,
    width: u32,
    signed: bool,
) -> std::iter::Take<std::vec::IntoIter<u8>> {
    sign_extend_vec(val.to_signed_bytes_le(), width, signed)
        .into_iter()
        .take(width.div_ceil(8) as usize)
}

/// This is so so so stupid. unfortunately, using the `BigRational` type's
/// to_f64 method results in some rounding behavior which creates very confusing
/// errors (demanding more precision) so this is a workaround.
fn float_to_rational(float: f64) -> BigRational {
    let string = format!("{:.}", float);
    let string = string.split('.').collect_vec();

    if string.len() == 1 {
        return BigRational::from_integer(
            BigInt::from_str_radix(string[0], 10).unwrap(),
        );
    }

    let is_neg = string[0].starts_with('-');

    let int = BigInt::from_str_radix(
        string[0].strip_prefix('-').unwrap_or(string[0]),
        10,
    )
    .unwrap();
    let frac = BigInt::from_str_radix(string[1], 10).unwrap();
    let denom = BigInt::from(10).pow(string[1].len() as u32);

    let result = BigRational::from_integer(int) + BigRational::new(frac, denom);
    if is_neg {
        -result
    } else {
        result
    }
}

fn unroll_float(
    val: f64,
    format: &interp::serialization::FormatInfo,
    round_float: bool,
) -> impl Iterator<Item = u8> {
    if let &interp::serialization::FormatInfo::Fixed {
        signed,
        int_width,
        frac_width,
    } = format
    {
        let rational = float_to_rational(val);

        let frac_part = rational.fract().abs();
        let frac_log = log2_exact(&frac_part.denom().to_biguint().unwrap());

        let number = if frac_log.is_none() && round_float {
            let w = BigInt::from(1) << frac_width;
            let new = (val * w.to_f64().unwrap()).round();
            new.to_bigint().unwrap()
        } else if frac_log.is_none() {
            panic!("Number {val} cannot be represented as a fixed-point number. If you want to approximate the number, set the `round_float` flag to true.");
        } else {
            let int_part = rational.to_integer();

            let frac_log = frac_log.unwrap_or_else(|| panic!("unable to round the given value to a value representable with {frac_width} fractional bits"));
            if frac_log > frac_width {
                panic!("cannot represent value with {frac_width} fractional bits, requires at least {frac_log} bits");
            }

            let mut int_log =
                log2_round_down(&int_part.abs().to_biguint().unwrap());
            if (BigInt::from(1) << int_log) <= int_part.abs() {
                int_log += 1;
            }
            if signed {
                int_log += 1;
            }

            if int_log > int_width {
                let signed_str = if signed { "signed " } else { "" };

                panic!("cannot represent {signed_str}value of {val} with {int_width} integer bits, requires at least {int_log} bits");
            }

            rational.numer() << (frac_width - frac_log)
        };

        let bit_count = number.bits() + if signed { 1 } else { 0 };

        if bit_count > (frac_width + int_width) as u64 {
            let difference = bit_count - frac_width as u64;
            panic!("The approximation of the number {val} cannot be represented with {frac_width} fractional bits and {int_width} integer bits. Requires at least {difference} integer bits.");
        }

        sign_extend_vec(
            number.to_signed_bytes_le(),
            frac_width + int_width,
            signed,
        )
        .into_iter()
        .take((frac_width + int_width).div_ceil(8) as usize)
    } else {
        panic!("Called unroll_float on a non-fixed point type");
    }
}

fn parse_bytes(bytes: &[u8], width: u32, signed: bool) -> BigInt {
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

fn parse_bytes_fixed(
    bytes: &[u8],
    int_width: u32,
    frac_width: u32,
    signed: bool,
) -> BigRational {
    let int = parse_bytes(bytes, int_width + frac_width, signed);

    BigRational::new(int.clone(), BigInt::from(1) << frac_width)
}

fn format_data(declaration: &MemoryDeclaration, data: &[u8]) -> ParseVec {
    let width = declaration.width();

    let chunk_stream =
        data.chunks_exact(width.div_ceil(8) as usize).map(|chunk| {
            match declaration.format {
                interp::serialization::FormatInfo::Bitnum {
                    signed, ..
                } => {
                    let int = parse_bytes(chunk, width, signed);
                    Number::from_str(&int.to_str_radix(10)).unwrap()
                }
                interp::serialization::FormatInfo::Fixed {
                    signed,
                    int_width,
                    frac_width,
                } => {
                    let int =
                        parse_bytes_fixed(chunk, int_width, frac_width, signed);
                    let float = int.to_f64().unwrap();

                    Number::from_f64(float).unwrap()
                }
            }
        });
    // sanity check
    assert!(data.len() % (width.div_ceil(8) as usize) == 0);

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

pub fn convert_from_data_dump(
    dump: &DataDump,
    use_quotes: bool,
) -> JsonPrintDump {
    let mut map = HashMap::new();
    for declaration in &dump.header.memories {
        let data = dump.get_data(&declaration.name).unwrap();
        let formatted_data = format_data(declaration, data);

        map.insert(declaration.name.clone(), formatted_data);
    }

    if use_quotes {
        let map: HashMap<String, PrintVec> =
            map.into_iter().map(|(k, v)| (k, v.into())).collect();
        map.into()
    } else {
        map.into()
    }
}

/// This is catastrophically stupid.
fn log2_round_down(x: &BigUint) -> u32 {
    if *x == BigUint::ZERO {
        return 0;
    }

    let mut count = 0_u32;
    while *x > BigUint::from(2_u32).pow(count) {
        count += 1;
    }

    if BigUint::from(2_u32).pow(count) == *x {
        count
    } else {
        count - 1
    }
}

fn log2_exact(x: &BigUint) -> Option<u32> {
    let log_round_down = log2_round_down(x);
    if *x == BigUint::from(2_u32).pow(log_round_down) {
        Some(log_round_down)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_unroll_float() {
        let float = -0.5;

        let signed = true;
        let int_width = 16;
        let frac_width = 16;

        let format = interp::serialization::FormatInfo::Fixed {
            signed,
            int_width,
            frac_width,
        };

        let result = unroll_float(float, &format, true);
        let result = result.collect_vec();
        BigInt::from_signed_bytes_le(&result);
        let parsed_res =
            parse_bytes_fixed(&result, int_width, frac_width, signed);
        println!(
            " exact {}\n approx {}",
            parsed_res,
            parsed_res.to_f64().unwrap()
        );

        assert_eq!(parsed_res.to_f64().unwrap(), float)
    }

    prop_compose! {
        fn arb_format_info_bitnum()(width in 1_u32..=128, signed in any::<bool>()) -> crate::json_data::FormatInfo {
            crate::json_data::FormatInfo {
                width: Some(width),
                is_signed: signed,
                numeric_type: NumericType::Bitnum,
                int_width: None,
                frac_width: None,
            }
        }
    }

    prop_compose! {
        fn arb_format_info_fixed()(int_width in 1_u32..=128, frac_width in 1_u32..=128, signed in any::<bool>()) -> crate::json_data::FormatInfo {
            crate::json_data::FormatInfo {
                width: None,
                is_signed: signed,
                numeric_type: NumericType::Fixed,
                int_width: Some(int_width),
                frac_width: Some(frac_width),
            }
        }
    }

    fn format_info_generator(
    ) -> impl Strategy<Value = crate::json_data::FormatInfo> {
        prop_oneof![arb_format_info_bitnum(), arb_format_info_fixed()]
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
        fn arb_bigint(width: u32, signed: bool)(mut data in prop::collection::vec(any::<u8>(), width.div_ceil(8) as usize)) -> BigInt {
            let last = data.last_mut().unwrap();
            let mask = 0b1111_1111u8
                >> (if width % 8 == 0 { 0 } else { 8 - width % 8 });
            *last &= mask;

            if signed {
                parse_bytes(data.as_slice(), width, signed)

            } else {
                BigInt::from_bytes_le(num_bigint::Sign::Plus, data.as_slice())
            }

        }
    }

    prop_compose! {
        fn arb_data(format: crate::json_data::FormatInfo, dimensions: Dimensions, signed: bool)(data in prop::collection::vec(arb_bigint(format.get_width(), signed), dimensions.size())) -> ParseVec {
            let data = data.into_iter().map(|x| {
                if format.is_fixedpt() {
                    let rat = BigRational::new(x.clone(), BigInt::from(1) << format.frac_width().unwrap());
                    Number::from_f64(rat.to_f64().unwrap()).unwrap()
                } else {
                    Number::from_str(&x.to_str_radix(10)).unwrap()
                }
            });

            match dimensions {
                Dimensions::D1(_) => data.collect_vec().into(),
                Dimensions::D2(_d0, d1) => data.into_iter().chunks(d1).into_iter().map(|v| v.collect_vec()).collect_vec().into(),
                Dimensions::D3(_d0, d1, d2) => data.into_iter().chunks(d1 * d2).into_iter().map(|v1| v1.chunks(d2).into_iter().map(|v2| v2.collect_vec()).collect_vec()).collect_vec().into(),
                Dimensions::D4(_d0, d1, d2, d3) => data.into_iter().chunks(d1 * d2 * d3).into_iter().map(|v1| v1.chunks(d2 * d3).into_iter().map(|v2| v2.chunks(d3).into_iter().map(|v3| v3.collect_vec()).collect_vec()).collect_vec()).collect_vec().into(),
            }
        }
    }

    fn arb_json_entry() -> impl Strategy<Value = JsonDataEntry> {
        let arb_format_info = format_info_generator();
        let dim = dim_generator();
        (arb_format_info, dim).prop_flat_map(|(format, dimensions)| {
            arb_data(format.clone(), dimensions, format.is_signed).prop_map(
                move |x| JsonDataEntry {
                    data: x,
                    format: format.clone(),
                },
            )
        })
    }

    fn arb_bigint_with_info() -> impl Strategy<Value = (BigInt, u32, bool)> {
        let width = prop_oneof![1..=128_u32];
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

            let dump = convert_to_data_dump(&json_data, true);

            let json_print_dump = convert_from_data_dump(&dump, false);

            for (name, entry) in &json_data.0 {
                prop_assert_eq!(&entry.data, json_print_dump.as_normal().unwrap().get(name).unwrap())
            }
        }

        #[test]
        fn sign_extend(data in arb_bigint_with_info()) {
            let (data, width, signed) = data;
            let vec = sign_extend_vec(data.to_signed_bytes_le(), width, signed);

            let parsed_back = parse_bytes(&vec, width, signed);

            prop_assert_eq!(data, parsed_back);
        }

    }
}
