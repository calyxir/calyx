// use crate::ir::IntermediateRepresentation;
use crate::ir::{IntermediateRepresentation, SpecialCase};
use num_bigint::{BigInt, BigUint};
use num_traits::One;

pub fn binary_to_u8_vec(binary_string: &str) -> Result<Vec<u8>, String> {
    // Determine the necessary length to pad to the nearest multiple of 8 bits
    let total_length = ((binary_string.len() + 7) / 8) * 8;

    // Get the most significant bit; default to '0' if the string is empty
    let padding_char = binary_string.chars().next().unwrap_or('0');

    // Pad the binary string with the most significant bit
    let current_len = binary_string.len();
    let padded_binary_string = if current_len >= total_length {
        binary_string.to_string()
    } else {
        let padding = std::iter::repeat(padding_char)
            .take(total_length - current_len)
            .collect::<String>();
        format!("{}{}", padding, binary_string)
    };

    // Convert the padded binary string to a u8 vector
    let mut vec = Vec::new();
    for chunk in padded_binary_string.as_bytes().chunks(8) {
        let byte_str = std::str::from_utf8(chunk)
            .map_err(|_| "Invalid UTF-8 in binary string.")?;
        let byte = u8::from_str_radix(byte_str, 2)
            .map_err(|_| "Invalid binary number.")?;
        vec.push(byte);
    }

    Ok(vec)
}

pub fn hex_to_u8_vec(hex: &str) -> Result<Vec<u8>, String> {
    let mut padded_hex = hex.to_string();

    // If the hex string length is not a multiple of 2, pad with the most significant nibble
    let padding = 2 - (padded_hex.len() % 2);
    if padding != 2 {
        let msb = &padded_hex[0..1]; // Get the most significant nibble
        padded_hex = msb.repeat(padding) + &padded_hex; // Pad with the MSB
    }

    let mut vec = Vec::new();

    for i in (0..padded_hex.len()).step_by(2) {
        let byte_str = &padded_hex[i..i + 2]; // Fixed to extract two hex digits
        match u8::from_str_radix(byte_str, 16) {
            Ok(byte) => vec.push(byte),
            Err(_) => return Err(String::from("Invalid hex string")),
        }
    }
    Ok(vec)
}

pub enum NumericFormat {
    Fixed {
        exponent: i64,
    },
    Float {
        exponent_len: i64,
        mantissa_len: i64,
    },
}

pub fn u8_to_ir(
    vector: Result<Vec<u8>, String>,
    numeric_format: NumericFormat,
    twos_comp: bool,
) -> IntermediateRepresentation {
    match vector {
        Ok(vec) => {
            let mut is_negative = false;

            if twos_comp {
                is_negative = (vec[0] & 0b10000000) != 0;
            }

            let (mantissa, exponent) = match numeric_format {
                NumericFormat::Fixed { exponent } => {
                    let mantissa = if is_negative {
                        let bigint = BigInt::from_signed_bytes_be(&vec);
                        bigint.magnitude().clone() // absolute value
                    } else {
                        BigUint::from_bytes_be(&vec)
                    };
                    (mantissa, exponent)
                }
                NumericFormat::Float {
                    exponent_len,
                    mantissa_len,
                } => {
                    let mut mantissa = BigUint::from(0u8);
                    let bit_offset = 1 + exponent_len; // Start after the sign (1 bit) and exponent

                    for i in 0..mantissa_len {
                        let byte_index = ((bit_offset + i) / 8) as usize;
                        let bit_index = ((bit_offset + i) % 8) as usize;
                        let bit = (vec[byte_index] >> (7 - bit_index)) & 1; // Get the i-th bit

                        // Shift the mantissa left and add the new bit
                        mantissa <<= 1;
                        if bit == 1 {
                            mantissa |= BigUint::one();
                        }
                    }

                    let mut exponent = 0i64;
                    for i in 0..exponent_len {
                        let byte_index = ((1 + i) / 8) as usize; // Start after the sign bit
                        let bit_index = ((1 + i) % 8) as usize;
                        let bit = (vec[byte_index] >> (7 - bit_index)) & 1;

                        // Shift exponent left and add the bit
                        exponent = (exponent << 1) | bit as i64;
                    }

                    // Apply the bias to the exponent
                    let bias = (1 << (exponent_len - 1)) - 1; // Bias: 2^(exponent_len-1) - 1
                    exponent -= bias;

                    (mantissa, exponent)
                }
            };

            IntermediateRepresentation {
                sign: is_negative,
                mantissa,
                exponent,
                special_case: SpecialCase::None,
            }
        }
        Err(e) => {
            panic!("Error unpacking vector: {}", e);
        }
    }
}
