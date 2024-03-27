use std::fs::read_to_string;
use std::fs::File;
use std::io::{self, Write};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    convert(&args[1], &args[2], &args[3]);
}


//Converts [filepath_get] to binary in [filepath_send]
fn convert(filepath_get: &String, filepath_send: &String, to_convert: &String) {
    // Create a file called converted.txt
    let mut converted = File::create(filepath_send).expect("creation failed");

    // Read file line by line
    for line in read_to_string(filepath_get).unwrap().lines() {
        //If convert from hex
        if to_convert == "hex"{
            hex_to_binary(line, &mut converted)
                .expect("Failed to write binary to file");
        } else if to_convert == "binary"{
            float_to_binary(line, &mut converted)
                .expect("Failed to write binary to file");
         } 

    }
    println!("Successfully converted to {} in {}", to_convert, filepath_send);
}

fn format_binary(binary_of_float: u32) -> String{
    // Format output nicely
    let binary_str = format!("{:032b}", binary_of_float);
    let formatted_binary_str = format!(
        "{} {} {}",
        &binary_str[0..1], // Sign bit
        &binary_str[1..9], // Exponent
        &binary_str[9..]   // Significand
    );
    return formatted_binary_str;
}
 
fn float_to_binary(line: &str, filepath_send: &mut File) -> std::io::Result<()> {

    let float_of_string: f32;
            // Convert string to float
            match line.parse::<f32>() {
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

fn hex_to_binary(hex_string: &str, filepath_send: &mut File) -> io::Result<()> {
    // Convert hex to binary
    let binary_of_hex = match u32::from_str_radix(hex_string, 16) {
        Ok(value) => value,
        Err(err) => return Err(io::Error::new(io::ErrorKind::InvalidData, err)),
    };

    // Convert to binary string
    let formatted_binary_str = format!("{:b}", binary_of_hex);

    // Write binary string to the file
    filepath_send.write_all(formatted_binary_str.as_bytes())?;
    filepath_send.write_all(b"\n")?;

    Ok(())
}