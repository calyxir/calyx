#[cfg(test)]
mod tests {
    use data_conversion::u8vector::{binary_to_u8_vec, hex_to_u8_vec};

    #[test]
    fn test_binary_to_u8_vec_padding() {
        // For input "101", the function pads with the MSB ('1') to get an 8-bit string:
        // "101" becomes "11111101" (binary) which is 253 in decimal.
        let input = "101";
        let result = binary_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![253]);
    }

    #[test]
    fn test_binary_to_u8_vec_exact() {
        // An already 8-bit binary string should be converted without padding.
        let input = "01010101";
        let result = binary_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![85]);
    }

    // #[test]
    // fn test_binary_to_u8_vec_empty() {
    //     let input = "";
    //     let result = binary_to_u8_vec(input).unwrap();
    //     assert_eq!(result, vec![0]); // Expect a single zero byte.
    // }

    #[test]
    fn test_binary_to_u8_vec_invalid_characters() {
        let input = "10201"; // Contains invalid '2'
        let result = binary_to_u8_vec(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_to_u8_vec_padding() {
        // For the hex input "1", the function pads with the MSB ("1") to form "11",
        // which corresponds to the value 0x11 (or 17 in decimal).
        let input = "1";
        let result = hex_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![0x11]);
    }

    #[test]
    fn test_hex_to_u8_vec_even() {
        // For an even-length hex string, no padding is needed.
        let input = "A1";
        let result = hex_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![0xA1]);
    }

    #[test]
    fn test_hex_to_u8_vec_invalid() {
        // An invalid hex string should return an error.
        let input = "GHI";
        let result = hex_to_u8_vec(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_binary_to_u8_vec_long_string() {
        let input = "10101010101010101010101010101010"; // 32 bits
        let result = binary_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![170, 170, 170, 170]); // 0b10101010 = 170
    }

    // #[test]
    // fn test_hex_to_u8_vec_empty() {
    //     let input = "";
    //     let result = hex_to_u8_vec(input).unwrap();
    //     assert_eq!(result, vec![0]); // Expect a single zero byte.
    // }

    #[test]
    fn test_hex_to_u8_vec_odd_length() {
        let input = "A"; // Single nibble
        let result = hex_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![0xAA]); // Padded to "AA"
    }

    #[test]
    fn test_hex_to_u8_vec_long_string() {
        let input = "A1B2C3D4E5F6"; // 12 characters
        let result = hex_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![0xA1, 0xB2, 0xC3, 0xD4, 0xE5, 0xF6]);
    }

    #[test]
    fn test_binary_to_u8_vec_all_zeros() {
        let input = "00000000";
        let result = binary_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn test_hex_to_u8_vec_all_zeros() {
        let input = "00";
        let result = hex_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn test_binary_to_u8_vec_all_ones() {
        let input = "11111111";
        let result = binary_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![255]); // 0b11111111 = 255
    }

    #[test]
    fn test_hex_to_u8_vec_all_ones() {
        let input = "FF";
        let result = hex_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![255]); // 0xFF = 255
    }
}

#[cfg(test)]
mod tests2 {
    use data_conversion::u8vector::{binary_to_u8_vec, hex_to_u8_vec};

    #[test]
    fn test_binary_to_u8_vec_padding() {
        // For input "101", the function pads with the MSB ('1') to get an 8-bit string:
        // "101" becomes "11111101" (binary) which is 253 in decimal.
        let input = "101";
        let result = binary_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![253]);
    }

    #[test]
    fn test_binary_to_u8_vec_exact() {
        // An already 8-bit binary string should be converted without padding.
        let input = "01010101";
        let result = binary_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![85]);
    }

    // #[test]
    // fn test_binary_to_u8_vec_empty() {
    //     let input = "";
    //     let result = binary_to_u8_vec(input).unwrap();
    //     assert_eq!(result, vec![0]); // Expect a single zero byte.
    // }

    #[test]
    fn test_binary_to_u8_vec_invalid_characters() {
        let input = "10201"; // Contains invalid '2'
        let result = binary_to_u8_vec(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_to_u8_vec_padding() {
        // For the hex input "1", the function pads with the MSB ("1") to form "11",
        // which corresponds to the value 0x11 (or 17 in decimal).
        let input = "1";
        let result = hex_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![0x11]);
    }

    #[test]
    fn test_hex_to_u8_vec_even() {
        // For an even-length hex string, no padding is needed.
        let input = "A1";
        let result = hex_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![0xA1]);
    }

    #[test]
    fn test_hex_to_u8_vec_invalid() {
        // An invalid hex string should return an error.
        let input = "GHI";
        let result = hex_to_u8_vec(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_binary_to_u8_vec_long_string() {
        let input = "10101010101010101010101010101010"; // 32 bits
        let result = binary_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![170, 170, 170, 170]); // 0b10101010 = 170
    }

    // #[test]
    // fn test_hex_to_u8_vec_empty() {
    //     let input = "";
    //     let result = hex_to_u8_vec(input).unwrap();
    //     assert_eq!(result, vec![0]); // Expect a single zero byte.
    // }

    #[test]
    fn test_hex_to_u8_vec_odd_length() {
        let input = "A"; // Single nibble
        let result = hex_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![0xAA]); // Padded to "AA"
    }

    #[test]
    fn test_hex_to_u8_vec_long_string() {
        let input = "A1B2C3D4E5F6"; // 12 characters
        let result = hex_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![0xA1, 0xB2, 0xC3, 0xD4, 0xE5, 0xF6]);
    }

    #[test]
    fn test_binary_to_u8_vec_all_zeros() {
        let input = "00000000";
        let result = binary_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn test_hex_to_u8_vec_all_zeros() {
        let input = "00";
        let result = hex_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn test_binary_to_u8_vec_all_ones() {
        let input = "11111111";
        let result = binary_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![255]); // 0b11111111 = 255
    }

    #[test]
    fn test_hex_to_u8_vec_all_ones() {
        let input = "FF";
        let result = hex_to_u8_vec(input).unwrap();
        assert_eq!(result, vec![255]); // 0xFF = 255
    }

    #[test]
    fn test_hex_to_u8_vec_invalid_characters() {
        let input = "Z1"; // Contains invalid 'Z'
        let result = hex_to_u8_vec(input);
        assert!(result.is_err());
    }
}
