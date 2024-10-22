use crate::ir::IntermediateRepresentation;
use num_bigint::{BigUint, BigInt};
use num_traits::One;


// let byte: u8 = 171; // Decimal
// println!("u8 value in decimal: {}", byte); // Prints 171
// println!("u8 value in binary: {:08b}", byte); // Prints 10101011
// println!("u8 value in hex: {:X}", byte); // Prints AB

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



pub fn u8_to_ir_fixed(vector: Result<Vec<u8>, String>, exponent: i64, twos_comp: bool) -> IntermediateRepresentation {
    match vector {
        Ok(vec) => {
            // Check if the MSB of the first byte is 1
            let mut is_negative = false; 

            if twos_comp{
                is_negative = (vec[0] & 0b10000000) != 0; 
            }

            let mantissa = if is_negative {
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



pub fn u8_to_ir_float(vector: Result<Vec<u8>, String>, exponent_len: i64, mantissa_len: i64, twos_comp: bool) -> IntermediateRepresentation {
    match vector {
        Ok(vec) => {

            let mut is_negative = false; 

            if twos_comp {
                // Check if the MSB of the first byte is 1
                is_negative = (vec[0] & 0b10000000) != 0; 
            }
            
            let mut mantissa = BigUint::from(0u8);
            let bit_offset = 1 + exponent_len; // Start after the sign (1 bit) and exponent

            for i in 0..mantissa_len {
                let byte_index = ((bit_offset + i) / 8) as usize;
                let bit_index = ((bit_offset + i) % 8) as usize;
                let bit = (vec[byte_index] >> (7 - bit_index)) & 1;  // Get the i-th bit

                // Shift the mantissa left and add the new bit
                mantissa = mantissa << 1;  // Shift left by 1
                if bit == 1 { // If bit is 1, add 1 to the mantissa
                    mantissa = mantissa | BigUint::one();  
                }
            }

            // Extract the exponent
            let mut exponent = 0i64;
            for i in 0..exponent_len {
                let byte_index = ((1 + i) / 8) as usize;  // Starting just after the sign bit
                let bit_index = ((1 + i) % 8) as usize;
                let bit = (vec[byte_index] >> (7 - bit_index)) & 1;

                // Shift exponent left and add the bit
                exponent = (exponent << 1) | bit as i64;
            }

            // Apply the bias to the exponent
            let bias = (1 << (exponent_len - 1)) - 1;  // Bias: 2^(exponent_len-1) - 1
            exponent = exponent - bias;  // Subtract the bias to get the actual exponent value


            IntermediateRepresentation {
                sign: is_negative,
                mantissa,
                exponent,
            }
        }
        Err(e) => {
            // Handle the error case, for example by panicking or returning a default value.
            panic!("Error unpacking vector: {}", e);
        }
    }
}