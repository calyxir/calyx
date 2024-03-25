use std::fs::read_to_string;
use std::fs::File;
use std::io::{self, Write};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    convert(&args[1], &args[2], &args[3], &args[4]);
}


/// Converts [filepath_get] from type [convert_from] to [convert_to] in [filepath_send]
fn convert(filepath_get: &String, filepath_send: &String, 
                                convert_from: &String, convert_to: &String) {
    // Create a file called converted.txt
    let mut converted = File::create(filepath_send).expect("creation failed");
    
        if convert_to == "binary"{
            //Convert from hex to binary
            if convert_from == "hex"{
                for line in read_to_string(filepath_get).unwrap().lines() {
                    hex_to_binary(line, &mut converted)
                        .expect("Failed to write binary to file");
                }
            //Convert from float to binary
            } else if convert_from == "float"{
                for line in read_to_string(filepath_get).unwrap().lines() {
                    float_to_binary(line, &mut converted)
                        .expect("Failed to write binary to file");
                }
            } 
        } else if convert_to == "hex"{
            //Convert from binary to hex
            if convert_from == "binary"{
                for line in read_to_string(filepath_get).unwrap().lines() {
                    binary_to_hex(line, &mut converted)
                        .expect("Failed to write hex to file");
                }
            }
        } else if convert_to == "float"{
            //Convert from binary to float
            if convert_from == "binary"{
                for line in read_to_string(filepath_get).unwrap().lines() {
                    binary_to_float(line, &mut converted)
                        .expect("Failed to write float to file");
                }
            }
        }
    println!("Successfully converted to {} in {}", convert_to, filepath_send);
}

/// Formats [binary_of_float] properly 
fn format_binary(binary_of_float: u32) -> String{
    let binary_str = format!("{:032b}", binary_of_float);
    let formatted_binary_str = format!(
        "{} {} {}",
        &binary_str[0..1], // Sign bit
        &binary_str[1..9], // Exponent
        &binary_str[9..]   // Significand
    );
    formatted_binary_str
}
 
/// Converts [binary_string] to binary and appends to [filepath_send]
fn float_to_binary(binary_string: &str, filepath_send: &mut File) -> std::io::Result<()> {
    let float_of_string: f32;
            // Convert string to float
            match binary_string.parse::<f32>() {
                Ok(parsed_num) => float_of_string = parsed_num,
                Err(_) => {
                    panic!("Failed to parse float from string")
                }
            }

    // Convert float to binary
    let binary_of_float = float_of_string.to_bits();
    let formatted_binary_str = format_binary(binary_of_float);

    // Write binary string to the file
    filepath_send.write_all(formatted_binary_str.as_bytes())?;
    filepath_send.write_all(b"\n")?;

    Ok(())
}

/// Converts [hex_string] to binary and appends to [filepath_send]
fn hex_to_binary(hex_string: &str, filepath_send: &mut File) -> io::Result<()> {
    // Convert hex to binary
    let binary_of_hex = match u32::from_str_radix(hex_string, 16) {
        Ok(value) => value,
        Err(err) => return Err(io::Error::new(io::ErrorKind::InvalidData, err)),
    };

    // Format nicely
    let formatted_binary_str = format!("{:b}", binary_of_hex);

    // Write binary string to the file
    filepath_send.write_all(formatted_binary_str.as_bytes())?;
    filepath_send.write_all(b"\n")?;

    Ok(())
}

fn binary_to_hex(binary_string: &str, filepath_send: &mut File) -> io::Result<()> {
    let hex_of_binary = match u32::from_str_radix(binary_string, 2){
        Ok(value) => value,
        Err(err) => return Err(io::Error::new(io::ErrorKind::InvalidData, err)),
    };

    // Format the integer as a hexadecimal string
    let formatted_hex_str = format!("{:X}", hex_of_binary);

    filepath_send.write(formatted_hex_str.as_bytes())?;
    filepath_send.write_all(b"\n")?;

    Ok(())
}

fn binary_to_float(binary_string: &str, filepath_send: &mut File) -> io::Result<()>{
    let binary_value = match u32::from_str_radix(binary_string, 2) {
        Ok(value) => value,
        Err(err) => return Err(io::Error::new(io::ErrorKind::InvalidData, err)),
    };

    // Interpret the integer as the binary representation of a floating-point number
    let float_value = unsafe { std::mem::transmute::<u32, f32>(binary_value) };

    let formated_float_str = format!("{:?}", float_value);

    filepath_send.write_all(formated_float_str.as_bytes())?;
    filepath_send.write_all(b"\n")?;

    Ok(())
}