pub fn binary_to_u8_vec(binary: &str) -> Result<Vec<u8>, String> {
    let mut padded_binary: String = binary.to_string();

    // If the binary string length is not a multiple of 8, pad it with leading zeros
    let padding = 8 - (padded_binary.len() % 8);
    if padding != 8 {
        padded_binary = "0".repeat(padding) + &padded_binary;
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

    let padding = 2 - (padded_hex.len() % 2);
    if padding != 2 {
        padded_hex = "0".repeat(padding) + &padded_hex;
    }

    let mut vec = Vec::new();

    for i in (0..padded_hex.len()).step_by(2) {
        let byte_str = &padded_hex[i..i + 8];
        match u8::from_str_radix(byte_str, 2){
            Ok(byte) => vec.push(byte),
            Err(_) => return Err(String::from("Invalid binary string")),
        }
    }
    Ok(vec)
}


pub fn decimal_to_u8_vec(decimal: &str) -> Result<Vec<u8>, String> {
    let mut vec = Vec::new();

    // Iterate over each character in the decimal string
    for c in decimal.chars() {
        // Check if the character is a digit
        if let Some(digit) = c.to_digit(10) {
            // Convert the digit (u32) to a u8 and push it to the vector
            vec.push(digit as u8);
        } else {
            return Err(format!("Invalid character '{}' in decimal string", c));
        }
    }
    Ok(vec)
}