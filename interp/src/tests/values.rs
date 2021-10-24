#[cfg(test)]
mod val_test {
    use crate::values::Value;
    #[test]
    fn basic_print_test() {
        let v1 = Value::from(12, 5);
        println!("12 with bit width 5: {}", v1);
        assert_eq!(v1.as_u64(), 12);
    }
    #[test]
    fn basic_print_test2() {
        let v1 = Value::from(33, 6);
        println!("33 with bit width 6: {}", v1);
        assert_eq!(v1.as_u64(), 33);
    }
    // TODO(cgyurgyik): Add more corner case tests / property tests.
    #[test]
    fn basic_print_fp_test() {
        use fraction::Fraction;
        let v1 = Value::from(/*value=*/ 0b0110, /*width=*/ 4);
        println!("3/2 with bit width 4 and fractional width 2: {}", v1);
        assert_eq!(
            v1.as_ufp(/*fractional_width=*/ 2),
            Fraction::new(3u32, 2u32)
        );
    }
    #[test]
    fn basic_print_fp_test2() {
        use fraction::Fraction;
        let v1 = Value::from(
            /*value=*/ 0b00000000000000000000000000000001,
            /*width=*/ 32,
        );
        println!(
            "1/2147483648 with bit width 32 and fractional width 31: {}",
            v1
        );
        assert_eq!(
            v1.as_ufp(/*fractional_width=*/ 31),
            Fraction::new(1u32, 2147483648u32)
        );
    }
    #[test]
    fn basic_print_fp_test3() {
        use fraction::Fraction;
        let v1 = Value::from(/*value=*/ 0b1110, /*width=*/ 4);
        println!("3/2 with bit width 4 and fractional width 2: {}", v1);
        assert_eq!(
            v1.as_ufp(/*fractional_width=*/ 2),
            Fraction::new(7u32, 2u32)
        );
    }
    #[test]
    fn basic_print_fp_test4() {
        use fraction::Fraction;
        let v1 = Value::from(/*value=*/ 0b111, /*width=*/ 3);
        println!("-1/2 with bit width 2 and fractional width 1: {}", v1);
        assert_eq!(v1.as_sfp(/*fractional_width=*/ 1), Fraction::from(-0.5));
    }
    #[test]
    fn too_few_bits() {
        let v_16_4 = Value::from(16, 4);
        println!("16 with bit width 4: {}", v_16_4);
        assert_eq!(v_16_4.as_u64(), 0);
        let v_31_4 = Value::from(31, 4);
        println!("31 with bit width 4: {}", v_31_4);
        let v_15_4 = Value::from(15, 4);
        println!("15 with bit width 4: {}", v_15_4);
        assert_eq!(v_31_4.as_u64(), v_15_4.as_u64());
    }
    #[test]
    fn clear() {
        let v_15_4 = Value::from(15, 4);
        let v_15_4 = v_15_4.clear();
        println!("15 with bit width 4 AFTER clear: {}", v_15_4);
        assert_eq!(v_15_4.as_u64(), 0);
    }
    #[test]
    fn ext() {
        let v_15_4 = Value::from(15, 4);
        assert_eq!(v_15_4.as_u64(), v_15_4.ext(8).as_u64());
    }
}

#[cfg(test)]
mod property_tests {
    use crate::values::Value;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn u8_round_trip(input: u8) {
            assert_eq!(input as u64, Value::from(input, 8).as_u64())
        }

        #[test]
        fn u16_round_trip(input: u16) {
            assert_eq!(input as u64, Value::from(input, 16).as_u64())
        }

        #[test]
        fn u32_round_trip(input: u32) {
            assert_eq!(input as u64, Value::from(input, 32).as_u64())
        }

        #[test]
        fn u64_round_trip(input: u64) {
            assert_eq!(input, Value::from(input, 64).as_u64())
        }

        #[test]
        fn u128_round_trip(input: u128) {
            assert_eq!(input, Value::from(input, 128).as_u128())
        }

        #[test]
        fn i8_round_trip(input: i8) {
            assert_eq!(input as i64, Value::from(input, 8).as_i64())
        }

        #[test]
        fn i16_round_trip(input: i16) {
            assert_eq!(input as i64, Value::from(input, 16).as_i64())
        }

        #[test]
        fn i32_round_trip(input: i32) {
            assert_eq!(input as i64, Value::from(input, 32).as_i64())
        }

        #[test]
        fn i64_round_trip(input: i64) {
            assert_eq!(input, Value::from(input, 64).as_i64())
        }

        #[test]
        fn i128_round_trip(input: i128) {
            assert_eq!(input, Value::from(input, 128).as_i128())
        }
    }
}
