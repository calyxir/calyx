#[cfg(test)]
mod val_test {
    use baa::*;
    #[test]
    fn basic_print_test() {
        let v1 = BitVecValue::from_u64(12, 5);
        println!("12 with bit width 5: {:?}", v1);
        assert_eq!(v1.to_u64().unwrap(), 12);
    }
    #[test]
    fn basic_print_test2() {
        let v1 = BitVecValue::from_u64(33, 6);
        println!("33 with bit width 6: {:?}", v1);
        assert_eq!(v1.to_u64().unwrap(), 33);
    }

    #[test]
    fn ext() {
        let v_15_4 = BitVecValue::from_u64(15, 4);
        assert_eq!(
            v_15_4.to_u64().unwrap(),
            v_15_4.zero_extend(4).to_u64().unwrap()
        );
    }
}

#[cfg(test)]
mod unsigned_fixed_point_tests {
    use baa::*;
    use fraction::Fraction;

    #[test]
    fn test_zero() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0, /*width=*/ 4)
                .to_unsigned_fixed_point(/*fractional_width=*/ 2)
                .unwrap(),
            Fraction::new(0u32, 1u32)
        );
    }
    #[test]
    fn test_zero_fractional_width() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b1110, /*width=*/ 4)
                .to_unsigned_fixed_point(/*fractional_width=*/ 0)
                .unwrap(),
            Fraction::new(14u32, 1u32)
        );
    }
    #[test]
    fn test_high_bits_set() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b1110, /*width=*/ 4)
                .to_unsigned_fixed_point(/*fractional_width=*/ 2)
                .unwrap(),
            Fraction::new(7u32, 2u32)
        );
    }
    #[test]
    fn test_middle_bits_set() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b0110, /*width=*/ 4)
                .to_unsigned_fixed_point(/*fractional_width=*/ 2)
                .unwrap(),
            Fraction::new(3u32, 2u32)
        );
    }
    #[test]
    fn test_low_bits_set() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b0111, /*width=*/ 4)
                .to_unsigned_fixed_point(/*fractional_width=*/ 2)
                .unwrap(),
            Fraction::new(7u32, 4u32)
        );
    }
    #[test]
    fn test_low_high_bits_set() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b1001, /*width=*/ 4)
                .to_unsigned_fixed_point(/*fractional_width=*/ 2)
                .unwrap(),
            Fraction::new(9u32, 4u32)
        );
    }
    #[test]
    fn test_32bit_fractional_value() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 1, /*width=*/ 32)
                .to_unsigned_fixed_point(/*fractional_width=*/ 31)
                .unwrap(),
            Fraction::new(1u32, 2147483648u32)
        );
    }
    #[test]
    fn test_64bit_fractional_value() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 1u64, /*width=*/ 64)
                .to_unsigned_fixed_point(/*fractional_width=*/ 63)
                .unwrap(),
            Fraction::new(1u64, 9223372036854775808u64)
        );
    }
    #[test]
    fn test_alternating_ones() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b10101, /*width=*/ 5)
                .to_unsigned_fixed_point(/*fractional_width=*/ 3)
                .unwrap(),
            Fraction::new(21u32, 8u32)
        );
    }
    #[test]
    fn test_all_ones() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b111, /*width=*/ 3)
                .to_unsigned_fixed_point(/*fractional_width=*/ 1)
                .unwrap(),
            Fraction::new(7u32, 2u32)
        );
    }
}

#[cfg(test)]
mod signed_fixed_point_tests {
    use baa::*;
    use fraction::Fraction;

    #[test]
    fn test_zero() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0, /*width=*/ 4)
                .to_signed_fixed_point(/*fractional_width=*/ 2)
                .unwrap(),
            Fraction::new(0u32, 1u32)
        );
    }
    #[test]
    fn test_zero_fractional_width() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b1110, /*width=*/ 4)
                .to_signed_fixed_point(/*fractional_width=*/ 0)
                .unwrap(),
            -Fraction::new(2u32, 1u32)
        );
    }
    #[test]
    fn test_high_bits_set() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b1110, /*width=*/ 4)
                .to_signed_fixed_point(/*fractional_width=*/ 2)
                .unwrap(),
            -Fraction::new(1u32, 2u32)
        );
    }
    #[test]
    fn test_middle_bits_set() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b0110, /*width=*/ 4)
                .to_signed_fixed_point(/*fractional_width=*/ 2)
                .unwrap(),
            Fraction::new(3u32, 2u32)
        );
    }
    #[test]
    fn test_low_bits_set() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b0111, /*width=*/ 4)
                .to_signed_fixed_point(/*fractional_width=*/ 2)
                .unwrap(),
            Fraction::new(7u32, 4u32)
        );
    }
    #[test]
    fn test_mixed_bits_set() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b10110101, /*width=*/ 8)
                .to_signed_fixed_point(/*fractional_width=*/ 3)
                .unwrap(),
            -Fraction::new(75u32, 8u32)
        );
    }
    #[test]
    fn test_mixed_bits_set2() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b10100011, /*width=*/ 8)
                .to_signed_fixed_point(/*fractional_width=*/ 4)
                .unwrap(),
            -Fraction::new(93u32, 16u32)
        );
    }
    #[test]
    fn test_mixed_bits_set3() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b11111101, /*width=*/ 8)
                .to_signed_fixed_point(/*fractional_width=*/ 4)
                .unwrap(),
            -Fraction::new(3u32, 16u32)
        );
    }
    #[test]
    fn test_low_high_bits_set() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b1001, /*width=*/ 4)
                .to_signed_fixed_point(/*fractional_width=*/ 2)
                .unwrap(),
            -Fraction::new(7u32, 4u32)
        );
    }
    #[test]
    fn test_single_bit_set() {
        assert_eq!(
            BitVecValue::from_u64(
                /*value=*/ 0b10000000000000000000000000000000,
                /*width=*/ 32,
            )
            .to_signed_fixed_point(/*fractional_width=*/ 31)
            .unwrap(),
            -Fraction::new(1u32, 1u32)
        );
    }
    #[test]
    fn test_small_negative_value() {
        assert_eq!(
            BitVecValue::from_u64(
                /*value=*/ 0b10000000000000000000000000000001,
                /*width=*/ 32,
            )
            .to_signed_fixed_point(/*fractional_width=*/ 31)
            .unwrap(),
            -Fraction::new(2147483647u32, 2147483648u32)
        );
    }
    #[test]
    fn test_alternating_ones() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b10101, /*width=*/ 5)
                .to_signed_fixed_point(/*fractional_width=*/ 3)
                .unwrap(),
            -Fraction::new(11u32, 8u32)
        );
    }
    #[test]
    fn test_all_ones() {
        assert_eq!(
            BitVecValue::from_u64(/*value=*/ 0b111, /*width=*/ 3)
                .to_signed_fixed_point(/*fractional_width=*/ 1)
                .unwrap(),
            -Fraction::new(1u32, 2u32)
        );
    }
}

#[cfg(test)]
mod property_tests {
    use baa::*;
    use num_bigint::{BigInt, BigUint};
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]


        #[test]
        fn u8_round_trip(input: u8) {
            prop_assert_eq!(input as u64, BitVecValue::from_u64(input as u64, 8).to_u64().unwrap())
        }

        #[test]
        fn u16_round_trip(input: u16) {
            prop_assert_eq!(input as u64, BitVecValue::from_u64(input as u64, 16).to_u64().unwrap())
        }

        #[test]
        fn u32_round_trip(input: u32) {
            prop_assert_eq!(input as u64, BitVecValue::from_u64(input as u64, 32).to_u64().unwrap())
        }

        #[test]
        fn u64_round_trip(input: u64) {
            prop_assert_eq!(input, BitVecValue::from_u64(input, 64).to_u64().unwrap())
        }

        #[test]
        fn i64_round_trip(input: i64) {
            prop_assert_eq!(input, BitVecValue::from_u64(input as u64, 64).to_i64().unwrap())
        }

        #[test]
        fn u32_to_ubig(input: u32) {
            let val = BitVecValue::from_u64(input as u64, 32);
            prop_assert_eq!(val.to_big_uint(), input.into())
        }

        #[test]
        fn i32_to_ibig(input: i32) {
            let val = BitVecValue::from_i64(input as i64, 32);
            prop_assert_eq!(val.to_big_int(), input.into())
        }
          #[test]
        fn ibig_to_ibig_neg(input in -350_i32..350_i32) {
            dbg!(input);
            let val = BitVecValue::from_big_int(&BigInt::from(input), 32);
            prop_assert_eq!(val.to_big_int(), BigInt::from(input))
        }

        #[test]
        fn ubig_roundtrip(input: u128, mul: u128) {
            let in_big: BigUint = input.into();
            let mul_big: BigUint = mul.into();
            let target: BigUint = in_big * mul_big;
            let val = BitVecValue::from_big_uint(&target, target.bits() as WidthInt);
            prop_assert_eq!(val.to_big_uint(), target)
        }

        #[test]
        fn ibig_roundtrip(input: i128, mul: i128) {
            let in_big: BigInt = input.into();
            let mul_big: BigInt = mul.into();
            let target: BigInt = in_big * mul_big;
            let val = BitVecValue::from_big_int(&target, target.magnitude().bits() as WidthInt + 1);
            println!("{:?}", val);
            prop_assert_eq!(val.to_big_int(), target)
        }
    }
}
