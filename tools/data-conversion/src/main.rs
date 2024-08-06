//use std::env;
use argh::FromArgs;
// use core::num;
use num_bigint::BigInt;
use num_bigint::BigUint;
// use num_traits::sign;
use num_traits::Num;
use std::error::Error;
use std::fmt;
use std::fs::read_to_string;
use std::fs::File;
use std::io::stdout;
use std::io::{self, Write};
use std::str::FromStr;
//cargo run -- --from $PATH1 --to $PATH2 --ftype "from" --totype "to"

// Threshold for using fast-track functions
const FAST_TRACK_THRESHOLD: u32 = 1 << 24; // Example threshold value, adjust as needed

struct IntermediateRepresentation {
    sign: bool,
    mantissa: BigUint,
    exponent: i32,
}

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

    /// optional file to convery to
    #[argh(option)]
    to: Option<String>,

    /// type to convert from
    #[argh(option)]
    ftype: NumType,

    /// type to convert to
    #[argh(option)]
    totype: NumType,

    /// optional exponent for fixed_to_binary -> default is -1
    #[argh(option, default = "-1")]
    exp: i32,

    /// optional for fixed_to_binary using bit slicing. If choosen, will use bit slicing.
    #[argh(switch, short = 'b')]
    bits: bool,
    // optional switch for
}

fn main() {
    let args: Arguments = argh::from_env();

    convert(
        &args.from,
        &args.to,
        args.ftype,
        args.totype,
        args.exp,
        args.bits,
    );
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
    filepath_send: &Option<String>,
    convert_from: NumType,
    convert_to: NumType,
    exponent: i32,
    bits: bool,
) {
    // Create the output file if filepath_send is Some
    let mut converted: Option<File> = filepath_send
        .as_ref()
        .map(|path| File::create(path).expect("creation failed"));

    match (convert_from, convert_to) {
        (NumType::Hex, NumType::Binary) => {
            for line in read_to_string(filepath_get).unwrap().lines() {
                if line.parse::<f32>().unwrap() <= FAST_TRACK_THRESHOLD as f32 {
                    hex_to_binary(line, &mut converted)
                        .expect("Failed to write binary to file");
                } else {
                    intermediate_to_binary(
                        hex_to_intermediate(line),
                        &mut converted,
                    )
                    .expect("Failed to write binary to file");
                }
            }
        }
        (NumType::Float, NumType::Binary) => {
            for line in read_to_string(filepath_get).unwrap().lines() {
                if line.parse::<f32>().unwrap() <= FAST_TRACK_THRESHOLD as f32 {
                    float_to_binary(line, &mut converted)
                        .expect("Failed to write binary to file");
                } else {
                    intermediate_to_binary(
                        float_to_intermediate(line),
                        &mut converted,
                    )
                    .expect("Failed to write binary to file");
                }
            }
        }
        (NumType::Fixed, NumType::Binary) => {
            for line in read_to_string(filepath_get).unwrap().lines() {
                if line.parse::<f32>().unwrap() <= FAST_TRACK_THRESHOLD as f32 {
                    fixed_to_binary(line, &mut converted, exponent)
                        .expect("Failed to write binary to file");
                } else {
                    intermediate_to_binary(
                        fixed_to_intermediate(line, exponent),
                        &mut converted,
                    )
                    .expect("Failed to write binary to file");
                }
            }
        }
        (NumType::Binary, NumType::Hex) => {
            for line in read_to_string(filepath_get).unwrap().lines() {
                if line.parse::<f32>().unwrap() <= FAST_TRACK_THRESHOLD as f32 {
                    binary_to_hex(line, &mut converted)
                        .expect("Failed to write hex to file");
                } else {
                    intermediate_to_hex(
                        binary_to_intermediate(line),
                        &mut converted,
                    )
                    .expect("Failed to write binary to file");
                }
            }
        }
        (NumType::Binary, NumType::Float) => {
            for line in read_to_string(filepath_get).unwrap().lines() {
                if line.parse::<f32>().unwrap() <= FAST_TRACK_THRESHOLD as f32 {
                    binary_to_float(line, &mut converted)
                        .expect("Failed to write float to file");
                } else {
                    intermediate_to_float(
                        binary_to_intermediate(line),
                        &mut converted,
                    )
                    .expect("Failed to write binary to file");
                }
            }
        }
        (NumType::Binary, NumType::Fixed) => {
            if !bits {
                for line in read_to_string(filepath_get).unwrap().lines() {
                    if line.parse::<f32>().unwrap()
                        <= FAST_TRACK_THRESHOLD as f32
                    {
                        binary_to_fixed(line, &mut converted, exponent)
                            .expect("Failed to write fixed-point to file");
                    } else {
                        intermediate_to_fixed(
                            binary_to_intermediate(line),
                            &mut converted,
                        )
                        .expect("Failed to write binary to file");
                    }
                }
            } else {
                for line in read_to_string(filepath_get).unwrap().lines() {
                    if line.parse::<f32>().unwrap()
                        <= FAST_TRACK_THRESHOLD as f32
                    {
                        binary_to_fixed_bit_slice(
                            line,
                            &mut converted,
                            exponent,
                        )
                        .expect("Failed to write fixed-point to file");
                    } else {
                        intermediate_to_fixed(
                            binary_to_intermediate(line),
                            &mut converted,
                        )
                        .expect("Failed to write binary to file");
                    }
                }
            }
        }
        _ => panic!(
            "Conversion from {} to {} is not supported",
            convert_from.to_string(),
            convert_to.to_string()
        ),
    }
    if let Some(filepath) = filepath_send {
        eprintln!(
            "Successfully converted from {} to {} in {}",
            convert_from.to_string(),
            convert_to.to_string(),
            filepath
        );
    } else {
        eprintln!(
            "Successfully converted from {} to {}",
            convert_from.to_string(),
            convert_to.to_string(),
        );
    }
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
    filepath_send: &mut Option<File>,
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

    if let Some(file) = filepath_send.as_mut() {
        file.write_all(formatted_binary_str.as_bytes())?;
        file.write_all(b"\n")?;
    } else {
        stdout().write_all(formatted_binary_str.as_bytes())?;
        stdout().write_all(b"\n")?;
    }

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
fn hex_to_binary(
    hex_string: &str,
    filepath_send: &mut Option<File>,
) -> io::Result<()> {
    // Convert hex to binary
    let binary_of_hex = u32::from_str_radix(hex_string, 16)
        .expect("Failed to parse hex string");

    // Format nicely
    let formatted_binary_str = format!("{:b}", binary_of_hex);

    // Write binary string to the file

    if let Some(file) = filepath_send.as_mut() {
        // Write binary string to the file
        file.write_all(formatted_binary_str.as_bytes())?;
        file.write_all(b"\n")?;
    } else {
        stdout().write_all(formatted_binary_str.as_bytes())?;
        stdout().write_all(b"\n")?;
    }

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
    filepath_send: &mut Option<File>,
) -> io::Result<()> {
    let hex_of_binary = u32::from_str_radix(binary_string, 2)
        .expect("Failed to parse binary string");

    let formatted_hex_str = format_hex(hex_of_binary);

    if let Some(file) = filepath_send.as_mut() {
        // Write binary string to the file
        file.write_all(formatted_hex_str.as_bytes())?;
        file.write_all(b"\n")?;
    } else {
        stdout().write_all(formatted_hex_str.as_bytes())?;
        stdout().write_all(b"\n")?;
    }

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
    filepath_send: &mut Option<File>,
) -> io::Result<()> {
    let binary_value = u32::from_str_radix(binary_string, 2)
        .expect("Failed to parse binary string");

    // Interpret the integer as the binary representation of a floating-point number
    let float_value = f32::from_bits(binary_value);

    let formated_float_str = format!("{:?}", float_value);

    if let Some(file) = filepath_send.as_mut() {
        // Write binary string to the file
        file.write_all(formated_float_str.as_bytes())?;
        file.write_all(b"\n")?;
    } else {
        stdout().write_all(formated_float_str.as_bytes())?;
        stdout().write_all(b"\n")?;
    }

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
    filepath_send: &mut Option<File>,
    exp_int: i32,
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

    if let Some(file) = filepath_send.as_mut() {
        // Write binary string to the file
        file.write_all(binary_of_fixed.as_bytes())?;
        file.write_all(b"\n")?;
    } else {
        stdout().write_all(binary_of_fixed.as_bytes())?;
        stdout().write_all(b"\n")?;
    }

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
    filepath_send: &mut Option<File>,
    exp_int: i32,
) -> io::Result<()> {
    // Convert binary value from string to int
    let binary_value = match u32::from_str_radix(binary_string, 2) {
        Ok(parsed_num) => parsed_num,
        Err(_) => panic!("Bad binary value input"),
    };

    // Convert to fixed
    let int_of_binary = binary_value as f32;

    //exponent int to float so we can multiply
    let exponent = exp_int as f32;

    // Exponent math
    let divided: f32 = int_of_binary / 2_f32.powf(-exponent);

    let string_of_divided = format!("{:+.8e}", divided);

    if let Some(file) = filepath_send.as_mut() {
        // Write binary string to the file
        file.write_all(string_of_divided.as_bytes())?;
        file.write_all(b"\n")?;
    } else {
        stdout().write_all(string_of_divided.as_bytes())?;
        stdout().write_all(b"\n")?;
    }

    Ok(())
}

fn binary_to_fixed_bit_slice(
    binary_string: &str,
    filepath_send: &mut Option<File>,
    exp_int: i32,
) -> io::Result<()> {
    // Convert binary string to an integer (assuming binary_string is a valid binary representation)
    let binary_int = u32::from_str_radix(binary_string, 2).unwrap();

    // Adjust the binary point based on the exponent
    let mut result = binary_int;
    if exp_int < 0 {
        // If exponent is negative, shift right (multiply by 2^(-exp_int))
        result >>= -exp_int as u32;
    } else {
        // If exponent is positive, shift left (multiply by 2^(exp_int))
        result <<= exp_int as u32;
    }

    // Convert result to a fixed-point decimal representation
    let fixed_value = result as f32;

    let string_of_fixed = format!("{:.8e}", fixed_value);

    if let Some(file) = filepath_send.as_mut() {
        // Write binary string to the file
        file.write_all(string_of_fixed.as_bytes())?;
        file.write_all(b"\n")?;
    } else {
        stdout().write_all(string_of_fixed.as_bytes())?;
        stdout().write_all(b"\n")?;
    }

    Ok(())
}

fn binary_to_intermediate(binary_string: &str) -> IntermediateRepresentation {
    let bit_width = binary_string.len();

    let sign = binary_string.chars().next() == Some('0');

    let binary_value = BigUint::from_str_radix(binary_string, 2)
        .expect("Invalid binary string");

    let mantissa = if sign {
        binary_value
    } else {
        // Calculate the two's complement for negative values
        let max_value = BigUint::from(1u32) << bit_width;
        &max_value - &binary_value
    };

    IntermediateRepresentation {
        sign,
        mantissa,
        exponent: 0,
    }
}

fn intermediate_to_binary(
    inter_rep: IntermediateRepresentation,
    filepath_send: &mut Option<File>,
) -> io::Result<()> {
    let inter_value = if inter_rep.sign {
        BigInt::from(inter_rep.mantissa)
    } else {
        -BigInt::from(inter_rep.mantissa)
    };

    let binary_str = inter_value.to_str_radix(2);

    if let Some(file) = filepath_send {
        file.write_all(binary_str.as_bytes())?;
        file.write_all(b"\n")?;
    } else {
        std::io::stdout().write_all(binary_str.as_bytes())?;
        std::io::stdout().write_all(b"\n")?;
    }

    Ok(())
}

fn float_to_intermediate(float_string: &str) -> IntermediateRepresentation {
    let sign = !float_string.starts_with("-");
    let float_trimmed = float_string.trim_start_matches("-");

    let parts: Vec<&str> = float_trimmed.split('.').collect();
    let integer_part = parts[0];
    let fractional_part = if parts.len() > 1 { parts[1] } else { "0" };
    // Prob not the best way to do this
    let mantissa_string = format!("{integer_part}{fractional_part}");

    IntermediateRepresentation {
        sign,
        mantissa: BigUint::from_str(&mantissa_string).expect("Invalid number"),
        exponent: -(fractional_part.len() as i32),
    }
}

fn intermediate_to_float(
    inter_rep: IntermediateRepresentation,
    filepath_send: &mut Option<File>,
) -> io::Result<()> {
    let mut mantissa_str = inter_rep.mantissa.to_string();

    // Determine the position to insert the decimal point
    let mut decimal_pos = mantissa_str.len() as i32 - inter_rep.exponent;

    // Handle cases where the decimal position is before the first digit
    if decimal_pos <= 0 {
        let zero_padding = "0".repeat(-decimal_pos as usize);
        mantissa_str = format!("{}{}", zero_padding, mantissa_str);
        decimal_pos = 1; // Decimal point will be at the first digit position
    }

    // Convert to &str for split_at
    let mantissa_str = mantissa_str.as_str();

    // Insert the decimal point
    let decimal_position = decimal_pos as usize;
    let (integer_part, fractional_part) = if decimal_position > 0 {
        mantissa_str.split_at(decimal_position)
    } else {
        ("0", mantissa_str)
    };

    let result = if inter_rep.sign {
        format!("{}.{}", integer_part, fractional_part)
    } else {
        format!("-{}.{}", integer_part, fractional_part)
    };

    if let Some(file) = filepath_send.as_mut() {
        // Write string to the file
        file.write_all(result.as_bytes())?;
        file.write_all(b"\n")?;
    } else {
        io::stdout().write_all(result.as_bytes())?;
        io::stdout().write_all(b"\n")?;
    }

    Ok(())
}

fn fixed_to_intermediate(
    fixed_string: &str,
    exp_int: i32,
) -> IntermediateRepresentation {
    let sign = !fixed_string.starts_with("-");
    let fixed_trimmed = fixed_string.trim_start_matches("-");

    let mantissa_string = &format!("{fixed_trimmed}");

    IntermediateRepresentation {
        sign,
        mantissa: BigUint::from_str(mantissa_string).expect("Invalid number"),
        exponent: exp_int,
    }
}

fn intermediate_to_fixed(
    inter_rep: IntermediateRepresentation,
    filepath_send: &mut Option<File>,
) -> io::Result<()> {
    // Negate exp
    let neg_exponent = -inter_rep.exponent;

    // 10^-exp
    let scale_factor = BigInt::from(10).pow(neg_exponent as u32);

    // Convert mantissa to BigInt
    let mantissa_bigint = BigInt::from(inter_rep.mantissa);

    let mantissa_mult = mantissa_bigint * scale_factor;

    // Apply the sign
    let signed_value = if inter_rep.sign {
        mantissa_mult
    } else {
        -mantissa_mult
    };

    // Handle placement of decimal point
    let mantissa_str = signed_value.to_string();
    let mantissa_len = mantissa_str.len();
    let adjusted_exponent = inter_rep.exponent + mantissa_len as i32;

    let string = if adjusted_exponent <= 0 {
        // Handle case where the exponent indicates a number less than 1
        let zero_padding = "0".repeat(-adjusted_exponent as usize);
        format!("0.{}{}", zero_padding, mantissa_str)
    } else if adjusted_exponent as usize >= mantissa_len {
        // Handle case where the exponent is larger than the length of the mantissa
        format!(
            "{}{}",
            mantissa_str,
            "0".repeat(adjusted_exponent as usize - mantissa_len)
        )
    } else {
        // Normal case
        let integer_part = &mantissa_str[..adjusted_exponent as usize];
        let fractional_part = &mantissa_str[adjusted_exponent as usize..];
        format!("{}.{}", integer_part, fractional_part)
    };

    // Write the result to the file or stdout
    if let Some(file) = filepath_send.as_mut() {
        file.write_all(string.as_bytes())?;
        file.write_all(b"\n")?;
    } else {
        io::stdout().write_all(string.as_bytes())?;
        io::stdout().write_all(b"\n")?;
    }

    Ok(())
}

fn hex_to_intermediate(hex_string: &str) -> IntermediateRepresentation {
    // Get sign value before converting string
    let sign = hex_string.chars().next() != Some('-');

    // Remove the '-' sign if present
    let cleaned_hex_string = if sign { hex_string } else { &hex_string[1..] };

    // Convert the cleaned hexadecimal string to BigUint
    let hex_value = BigUint::from_str_radix(cleaned_hex_string, 16)
        .expect("Invalid hexadecimal string");

    IntermediateRepresentation {
        sign,
        mantissa: hex_value,
        exponent: 0,
    }
}

fn intermediate_to_hex(
    inter_rep: IntermediateRepresentation,
    filepath_send: &mut Option<File>,
) -> io::Result<()> {
    // Apply the sign
    let hex_value = if inter_rep.sign {
        inter_rep.mantissa.to_str_radix(16)
    } else {
        format!("-{}", inter_rep.mantissa.to_str_radix(16))
    };

    // Write the result to the file or stdout
    if let Some(file) = filepath_send.as_mut() {
        file.write_all(hex_value.as_bytes())?;
        file.write_all(b"\n")?;
    } else {
        io::stdout().write_all(hex_value.as_bytes())?;
        io::stdout().write_all(b"\n")?;
    }

    Ok(())
}

// ChatGPT stuff below

// fn format_scientific_notation(num_str: &str, exponent: i32) -> String {
//     // Check for an empty string
//     if num_str.is_empty() {
//         return "0.0e0".to_string();
//     }

//     // Find the position of the decimal point
//     let (integer_part, fractional_part) = if num_str.contains('.') {
//         let parts: Vec<&str> = num_str.split('.').collect();
//         (parts[0], parts.get(1).unwrap_or(&""))
//     } else {
//         (num_str, "")
//     };

//     // Calculate the new exponent after including the fractional part
//     let new_exponent = exponent + integer_part.len() as i32;

//     // Format integer part and fractional part
//     let integer_part = integer_part.to_string();
//     let mut fractional_part = fractional_part.to_string();
//     let mut result = String::new();

//     if !integer_part.is_empty() {
//         result.push_str(&integer_part);
//     }

//     if !fractional_part.is_empty() {
//         result.push('.');
//         result.push_str(&fractional_part);
//     }

//     format!("{:+.8e}", result.to_f64().unwrap_or(0.0)) // Converting to f64 for formatting
// }
