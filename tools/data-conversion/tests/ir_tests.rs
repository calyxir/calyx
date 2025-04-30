#[cfg(test)]
mod tests {
    use data_conversion::ir::{
        from_binary, from_float, from_hex, to_binary, to_dec_fixed,
        to_dec_float, to_hex, IntermediateRepresentation, SpecialCase,
    };
    use num_bigint::BigUint;
    use std::fs::File;

    #[test]
    fn test_from_binary_positive() {
        let binary_string = "00000010"; // 2 in binary
        let ir = from_binary(binary_string, 8, false);

        assert_eq!(ir.sign, false); // Positive number
        assert_eq!(ir.mantissa, BigUint::from(2u32));
        assert_eq!(ir.exponent, 0);
        assert_eq!(ir.special_case, SpecialCase::None);
    }

    #[test]
    fn test_from_binary_negative_twos_complement() {
        let binary_string = "11111110"; // -2 in two's complement (8-bit)
        let ir = from_binary(binary_string, 8, true);

        assert_eq!(ir.sign, true); // Negative number
        assert_eq!(ir.mantissa, BigUint::from(2u32)); // Absolute value of mantissa
        assert_eq!(ir.exponent, 0);
        assert_eq!(ir.special_case, SpecialCase::None);
    }

    #[test]
    fn test_from_float() {
        let float_string = "-123.45";
        let ir = from_float(float_string);

        assert_eq!(ir.sign, true); // Negative number
        assert_eq!(ir.mantissa, BigUint::from(12345u32)); // Mantissa is 12345
        assert_eq!(ir.exponent, -2); // Exponent is -2
        assert_eq!(ir.special_case, SpecialCase::None);
    }

    #[test]
    fn test_from_hex_positive() {
        let hex_string = "1A"; // 26 in decimal
        let ir = from_hex(hex_string, 8);

        assert_eq!(ir.sign, false); // Positive number
        assert_eq!(ir.mantissa, BigUint::from(26u32));
        assert_eq!(ir.exponent, 0);
        assert_eq!(ir.special_case, SpecialCase::None);
    }

    #[test]
    fn test_from_hex_negative_twos_complement() {
        let hex_string = "FE"; // -2 in two's complement (8-bit)
        let ir = from_hex(hex_string, 8);

        assert_eq!(ir.sign, true); // Negative number
        assert_eq!(ir.mantissa, BigUint::from(2u32)); // Absolute value of mantissa
        assert_eq!(ir.exponent, 0);
        assert_eq!(ir.special_case, SpecialCase::None);
    }

    #[test]
    fn test_to_binary() {
        let ir = IntermediateRepresentation {
            sign: false,
            mantissa: BigUint::from(5u32),
            exponent: 0,
            special_case: SpecialCase::None,
        };

        let output: Vec<u8> = Vec::new();
        to_binary(ir, &mut Some(File::create("/dev/null").unwrap()), 8)
            .unwrap();
    }

    #[test]
    fn test_to_dec_fixed() {
        let ir = IntermediateRepresentation {
            sign: false,
            mantissa: BigUint::from(12345u32),
            exponent: -2,
            special_case: SpecialCase::None,
        };

        let output: Vec<u8> = Vec::new();
        to_dec_fixed(ir, &mut Some(File::create("/dev/null").unwrap()))
            .unwrap();
    }

    #[test]
    fn test_to_dec_float() {
        let ir = IntermediateRepresentation {
            sign: true,
            mantissa: BigUint::from(12345u32),
            exponent: -2,
            special_case: SpecialCase::None,
        };

        let output: Vec<u8> = Vec::new();
        to_dec_float(ir, &mut Some(File::create("/dev/null").unwrap()))
            .unwrap();
    }

    #[test]
    fn test_to_hex() {
        let ir = IntermediateRepresentation {
            sign: false,
            mantissa: BigUint::from(255u32),
            exponent: 0,
            special_case: SpecialCase::None,
        };

        let output: Vec<u8> = Vec::new();
        to_hex(ir, &mut Some(File::create("/dev/null").unwrap())).unwrap();
    }

    #[test]
    fn test_special_case_zero() {
        let binary_string = "00000000"; // Zero
        let ir = from_binary(binary_string, 8, false);

        assert_eq!(ir.special_case, SpecialCase::Zero);
    }

    #[test]
    fn test_special_case_infinity() {
        let binary_string = "11111111"; // Exponent all ones, Mantissa zero
        let ir = from_binary(binary_string, 8, false);

        assert_eq!(ir.special_case, SpecialCase::Infinity);
    }

    #[test]
    fn test_special_case_nan() {
        let binary_string = "11111110"; // Exponent all ones, Mantissa non-zero
        let ir = from_binary(binary_string, 8, false);

        assert_eq!(ir.special_case, SpecialCase::NaN);
    }
}
