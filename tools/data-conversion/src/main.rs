//use std::env;
use argh::FromArgs;
use std::error::Error;
use std::fmt;
use std::fs::read_to_string;
use std::fs::File;
use std::io::{self, Write};
use std::str::FromStr;

//cargo run -- --from $PATH1 --to $PATH2 --ftype "binary" --totype "hex"

#[derive(Debug)]
struct ParseNumTypeError;

impl fmt::Display for ParseNumTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid number type")
    }
}

impl Error for ParseNumTypeError {}

#[derive(Debug, PartialEq, Clone, Copy)] // Add PartialEq derivation here - What is this?
enum NumType {
    Binary,
    Float,
    Hex,
    Fixed,
}

impl ToString for NumType {
    fn to_string(&self) -> String {
        match self {
            NumType::Binary => "binary".to_string(),
            NumType::Float => "float".to_string(),
            NumType::Hex => "hex".to_string(),
            NumType::Fixed => "fixed".to_string(),
        }
    }
}

impl FromStr for NumType {
    type Err = ParseNumTypeError;

    fn from_str(input: &str) -> Result<NumType, Self::Err> {
        match input {
            "binary" => Ok(NumType::Binary),
            "float" => Ok(NumType::Float),
            "hex" => Ok(NumType::Hex),
            "fixed" => Ok(NumType::Fixed),
            _ => Err(ParseNumTypeError),
        }
    }
}

#[derive(FromArgs)]
/// get arguments to convert
struct Arguments {
    /// file to convert from
    #[argh(option)]
    from: String,

    /// file to convery to
    #[argh(option)]
    to: String,

    /// type to convert from
    #[argh(option)]
    ftype: NumType,

    /// type to convert to
    #[argh(option)]
    totype: NumType,

    /// optional exponent for fixed_to_binary -> default is -1
    #[argh(option, default = "-1")]
    exp: i32,
}

fn main() {
    let args: Arguments = argh::from_env();

    convert(&args.from, &args.to, args.ftype, args.totype, args.exp);
}

/// Converts [filepath_get] from type [convert_from] to type
/// [convert_to] in [filepath_send]

/// # Arguments
///
/// * `filepath_get` - A reference to a `String` representing the path to the input file
///   containing data to be converted.
/// * `filepath_send` - A reference to a `String` representing the path to the output file
///   where the converted data will be written.
/// * `convert_from` - A reference to a `NumType` enum indicating the type of the input data.
/// * `convert_to` - A reference to a `NumType` enum indicating the type of the output data.
/// * `exponent` - An `i32` value used as the exponent for conversions involving fixed-point numbers.
///
/// # Returns
///
/// Returns `Ok(())` if the conversion and file writing operations are successful,
/// or an `Err` if an I/O error occurs during the process.
fn convert(
    filepath_get: &String,
    filepath_send: &String,
    convert_from: NumType,
    convert_to: NumType,
    exponent: i32,
) {
    // Create the output file
    let mut converted = File::create(filepath_send).expect("creation failed");

    match (convert_from, convert_to) {
        (NumType::Hex, NumType::Binary) => {
            for line in read_to_string(filepath_get).unwrap().lines() {
                hex_to_binary(line, &mut converted)
                    .expect("Failed to write binary to file");
            }
        }
        (NumType::Float, NumType::Binary) => {
            for line in read_to_string(filepath_get).unwrap().lines() {
                float_to_binary(line, &mut converted)
                    .expect("Failed to write binary to file");
            }
        }
        (NumType::Fixed, NumType::Binary) => {
            for line in read_to_string(filepath_get).unwrap().lines() {
                fixed_to_binary(line, &mut converted, exponent)
                    .expect("Failed to write binary to file");
            }
        }
        (NumType::Binary, NumType::Hex) => {
            for line in read_to_string(filepath_get).unwrap().lines() {
                binary_to_hex(line, &mut converted)
                    .expect("Failed to write hex to file");
            }
        }
        (NumType::Binary, NumType::Float) => {
            for line in read_to_string(filepath_get).unwrap().lines() {
                binary_to_float(line, &mut converted)
                    .expect("Failed to write float to file");
            }
        }
        (NumType::Binary, NumType::Fixed) => {
            for line in read_to_string(filepath_get).unwrap().lines() {
                binary_to_fixed(line, &mut converted, exponent)
                    .expect("Failed to write fixed-point to file");
            }
        }
        _ => panic!(
            "Conversion from {} to {} is not supported",
            convert_from.to_string(),
            convert_to.to_string()
        ),
    }

    eprintln!(
        "Successfully converted from {} to {} in {}",
        convert_from.to_string(),
        convert_to.to_string(),
        filepath_send
    );
}

/// Formats [to_format] properly for float values
fn format_binary(to_format: u32) -> String {
    let binary_str = format!("{:032b}", to_format);
    format!(
        "{} {} {}",
        &binary_str[0..1], // Sign bit
        &binary_str[1..9], // Exponent
        &binary_str[9..]   // Significand
    )
}

fn format_hex(to_format: u32) -> String {
    format!("0x{:X}", to_format)
}

/// Converts a string representation of a floating-point number to its binary
/// format and appends the result to the specified file.
///
/// This function takes a string slice representing a floating-point number,
/// converts it to a 32-bit floating-point number (`f32`), then converts this
/// number to its binary representation. The binary representation is formatted
/// as a string and written to the specified file, followed by a newline.
///
/// # Arguments
///
/// * `float_string` - A string slice containing the floating-point number to be converted.
/// * `filepath_send` - A mutable reference to a `File` where the binary representation
///   will be appended.
///
/// # Returns
///
/// This function returns a `std::io::Result<()>` which is `Ok` if the operation
/// is successful, or an `Err` if an I/O error occurs while writing to the file.
///
/// # Panics
///
/// This function will panic if the input string cannot be parsed as a floating-point number.
fn float_to_binary(
    float_string: &str,
    filepath_send: &mut File,
) -> std::io::Result<()> {
    let float_of_string: f32;
    // Convert string to float
    match float_string.parse::<f32>() {
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

/// Converts a string representation of a hexadecimal number to its binary
/// format and appends the result to the specified file.
///
/// This function takes a string slice representing a hexadecimal number,
/// converts it to a 32-bit integer (`u32`), then converts this number to its
/// binary representation. The binary representation is formatted as a string
/// and written to the specified file, followed by a newline.
///
/// # Arguments
///
/// * `hex_string` - A string slice containing the hexadecimal number to be converted.
/// * `filepath_send` - A mutable reference to a `File` where the binary representation
///   will be appended.
///
/// # Returns
///
/// This function returns a `std::io::Result<()>` which is `Ok` if the operation
/// is successful, or an `Err` if an I/O error occurs while writing to the file.
///
/// # Error
///
/// This function will panic if the input string cannot be parsed as a hexadecimal number.
fn hex_to_binary(hex_string: &str, filepath_send: &mut File) -> io::Result<()> {
    // Convert hex to binary
    let binary_of_hex = u32::from_str_radix(hex_string, 16)
        .expect("Failed to parse hex string");

    // Format nicely
    let formatted_binary_str = format!("{:b}", binary_of_hex);

    // Write binary string to the file
    filepath_send.write_all(formatted_binary_str.as_bytes())?;
    filepath_send.write_all(b"\n")?;

    Ok(())
}

/// Converts a string representation of a binary number to its hexadecimal
/// format and appends the result to the specified file.
///
/// This function takes a string slice representing a binary number,
/// converts it to a 32-bit integer (`u32`), then converts this number to its
/// hexadecimal representation. The hexadecimal representation is formatted
/// as a string and written to the specified file, followed by a newline.
///
/// # Arguments
///
/// * `binary_string` - A string slice containing the binary number to be converted.
/// * `filepath_send` - A mutable reference to a `File` where the hexadecimal representation
///   will be appended.
///
/// # Returns
///
/// This function returns a `std::io::Result<()>` which is `Ok` if the operation
/// is successful, or an `Err` if an I/O error occurs while writing to the file.
///
/// # Panics
///
/// This function will panic if the input string cannot be parsed as a binary number.
fn binary_to_hex(
    binary_string: &str,
    filepath_send: &mut File,
) -> io::Result<()> {
    let hex_of_binary = u32::from_str_radix(binary_string, 2)
        .expect("Failed to parse binary string");

    let formatted_hex_str = format_hex(hex_of_binary);

    filepath_send.write_all(formatted_hex_str.as_bytes())?;
    filepath_send.write_all(b"\n")?;

    Ok(())
}

/// Converts a string representation of a binary number to its floating-point
/// format and appends the result to the specified file.
///
/// This function takes a string slice representing a binary number,
/// converts it to a 32-bit integer (`u32`), then interprets this integer as
/// the binary representation of a 32-bit floating-point number (`f32`).
/// The floating-point representation is formatted as a string and written
/// to the specified file, followed by a newline.
///
/// # Arguments
///
/// * `binary_string` - A string slice containing the binary number to be converted.
/// * `filepath_send` - A mutable reference to a `File` where the floating-point representation
///   will be appended.
///
/// # Returns
///
/// This function returns a `std::io::Result<()>` which is `Ok` if the operation
/// is successful, or an `Err` if an I/O error occurs while writing to the file.
///
/// # Panics
///
/// This function will panic if the input string cannot be parsed as a binary number.
fn binary_to_float(
    binary_string: &str,
    filepath_send: &mut File,
) -> io::Result<()> {
    let binary_value = u32::from_str_radix(binary_string, 2)
        .expect("Failed to parse binary string");

    // Interpret the integer as the binary representation of a floating-point number
    let float_value = f32::from_bits(binary_value);

    let formated_float_str = format!("{:?}", float_value);

    filepath_send.write_all(formated_float_str.as_bytes())?;
    filepath_send.write_all(b"\n")?;

    Ok(())
}

/// Converts a string representation of a fixed-point number to its binary
/// format and appends the result to the specified file.
///
/// This function takes a string slice representing a fixed-point number,
/// multiplies it by 2 raised to the power of the negative exponent, converts the result
/// to a 32-bit integer, and then to its binary representation. The binary representation
/// is formatted as a string and written to the specified file, followed by a newline.
///
/// # Arguments
///
/// * `fixed_string` - A string slice containing the fixed-point number to be converted.
/// * `filepath_send` - A mutable reference to a `File` where the binary representation
///   will be appended.
/// * `exponent` - A floating-point number representing the exponent to be applied in the
///   conversion process.
///
/// # Returns
///
/// This function returns a `std::io::Result<()>` which is `Ok` if the operation
/// is successful, or an `Err` if an I/O error occurs while writing to the file.
///
/// # Panics
///
/// This function will panic if the input string cannot be parsed as a fixed-point number.
fn fixed_to_binary(
    fixed_string: &str,
    filepath_send: &mut File,
    exp_int: i32,
    // scale: usize,
) -> io::Result<()> {
    // Convert fixed value from string to int
    let fixed_value: f32;
    match fixed_string.parse::<f32>() {
        Ok(parsed_num) => fixed_value = parsed_num,
        Err(_) => {
            panic!("Bad fixed value input")
        }
    }

    //exponent int to float so we can multiply
    let exponent = exp_int as f32;

    // Exponent math
    let multiplied_fixed = fixed_value * 2_f32.powf(-exponent);

    // Convert to a 32-bit integer
    let multiplied_fixed_as_i32 = multiplied_fixed as i32;

    // Convert to a binary string with 32 bits
    let binary_of_fixed = format!("{:032b}", multiplied_fixed_as_i32);

    // Write binary string to the file
    filepath_send.write_all(binary_of_fixed.as_bytes())?;
    filepath_send.write_all(b"\n")?;

    Ok(())
}

/// Converts a string representation of a binary number to its fixed-point
/// format and appends the result to the specified file.
///
/// This function takes a string slice representing a binary number,
/// converts it to a 32-bit unsigned integer, interprets this integer as
/// a floating-point number, divides it by 2 raised to the power of the negative exponent,
/// and converts the result to its fixed-point representation. The fixed-point
/// representation is formatted as a string and written to the specified file,
/// followed by a newline.
///
/// # Arguments
///
/// * `binary_string` - A string slice containing the binary number to be converted.
/// * `filepath_send` - A mutable reference to a `File` where the fixed-point representation
///   will be appended.
/// * `exponent` - A floating-point number representing the exponent to be applied in the
///   conversion process.
///
/// # Returns
///
/// This function returns a `std::io::Result<()>` which is `Ok` if the operation
/// is successful, or an `Err` if an I/O error occurs while writing to the file.
///
/// # Panics
///
/// This function will panic if the input string cannot be parsed as a binary number.
fn binary_to_fixed(
    binary_string: &str,
    filepath_send: &mut File,
    exp_int: i32,
) -> io::Result<()> {
    // Create an array with the elements of fixed_string
    let words: Vec<&str> = binary_string.split_whitespace().collect();
    let binary_str: &&str =
        words.first().unwrap_or(&"There is not a binary number");

    // Convert binary value from string to int
    let binary_value = match u32::from_str_radix(binary_str, 2) {
        Ok(parsed_num) => parsed_num,
        Err(_) => panic!("Bad binary value input"),
    };

    // Convert to fixed
    let int_of_binary = binary_value as f32;

    //exponent int to float so we can multiply
    let exponent = exp_int as f32;

    // Exponent math
    let divided: f32 = int_of_binary / 2_f32.powf(-exponent);

    let string_of_divided = divided.to_string();

    // filepath_send.write_all(divided)?;
    filepath_send.write_all(string_of_divided.as_bytes())?;
    filepath_send.write_all(b"\n")?;

    Ok(())
}
