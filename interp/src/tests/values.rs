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
    use ibig::{ops::UnsignedAbs, IBig, UBig};
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

        #[test]
        fn i128_to_ibig(input: i128) {
            let val = Value::from(input, 128);
            assert_eq!(val.as_signed(), input.into())
        }

        #[test]
        fn u128_to_ubig(input: u128) {
            let val = Value::from(input, 128);
            assert_eq!(val.as_unsigned(), input.into())
        }

        #[test]
        fn ubig_roundtrip(input: u128, mul: u128) {
            let in_big: UBig = input.into();
            let mul_big: UBig = mul.into();
            let target: UBig = in_big * mul_big;
            let val = Value::from(target.clone(), target.bit_len());
            assert_eq!(val.as_unsigned(), target)
        }

        #[test]
        fn ibig_roundtrip(input: i128, mul: i128) {
            let in_big: IBig = input.into();
            let mul_big: IBig = mul.into();
            let target: IBig = in_big * mul_big;
            let val = Value::from(target.clone(), (&target).unsigned_abs().bit_len()+1);
            println!("{}", val);
            assert_eq!(val.as_signed(), target)
        }
    }
}
