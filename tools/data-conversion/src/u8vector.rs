use crate::ir::IntermediateRepresentation;
use num_bigint::{BigUint, BigInt};

pub fn binary_to_u8_vec(binary: &str) -> Result<Vec<u8>, String> {
    let mut padded_binary = binary.to_string();

    // If the binary string length is not a multiple of 8, pad with the most significant bit
    let padding = 8 - (padded_binary.len() % 8);
    if padding != 8 {
        let msb = &padded_binary[0..1]; // Get the most significant bit
        padded_binary = msb.repeat(padding) + &padded_binary; // Pad with the MSB
    }

    let mut vec = Vec::new();

    for i in (0..padded_binary.len()).step_by(8) {
        let byte_str = &padded_binary[i..i + 8];
        match u8::from_str_radix(byte_str, 2) {
            Ok(byte) => vec.push(byte),
            Err(_) => return Err(String::from("Invalid binary string")),
        }
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



pub fn u8_to_ir(vector: Result<Vec<u8>, String>, exponent: i64) -> IntermediateRepresentation {
    match vector {
        Ok(vec) => {
            // Check if the MSB of the first byte is 1
            let is_negative = (vec[0] & 0b10000000) != 0; 

            let mantissa = if is_negative {
                // Convert the Vec<u8> to a two's complement BigInt, then get its absolute value
                let bigint = BigInt::from_signed_bytes_be(&vec);
                bigint.magnitude().clone() // absolute value 
            } else {
                BigUint::from_bytes_be(&vec)
            };

            IntermediateRepresentation {
                sign: is_negative,
                mantissa,
                exponent,
            }
        }
        Err(e) => {
            // Handle the error case, for example by panicking or returning a default value.
            panic!("Failed to convert: {}", e);
        }
    }
}