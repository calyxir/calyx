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

// Thresholds for using fast-track functions
const FAST_TRACK_THRESHOLD_BINARY_TO_FIXED: usize = 53; //52 bits for the significand (plus 1 implicit bit)b
const FAST_TRACK_THRESHOLD_FLOAT_TO_BINARY: usize = 53;
const FAST_TRACK_THRESHOLD_BINARY_TO_FLOAT: usize = 53;
const FAST_TRACK_THRESHOLD_FIXED_TO_BINARY: usize = 53;
const FAST_TRACK_THRESHOLD_HEX_TO_BINARY: usize = 64;
const FAST_TRACK_THRESHOLD_BINARY_TO_HEX: usize = 64;

/// * 'sign' - `true` indicates that the value is negative; `false` indicates that it is positive.
/// * 'mantissa' - The absolute value represented as an integer without a decimal point.
/// * 'exponent' - The exponent to apply to the mantissa, where the actual value is calculated as `mantissa * 2^exponent`. The exponent can be negative.
struct IntermediateRepresentation {
    sign: bool,
    mantissa: BigUint,
    exponent: i64,
}

#[derive(Debug)]
struct ParseNumTypeError;

impl fmt::Display for ParseNumTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid numeric type")
    }
}

impl Error for ParseNumTypeError {}

#[derive(Debug, PartialEq, Clone, Copy)] 
enum NumType {
    Float,
    Fixed,
}

impl FromStr for NumType {
    type Err = ParseNumTypeError;

    fn from_str(input: &str) -> Result<NumType, Self::Err> {
        match input {
            "float" => Ok(NumType::Float),
            "fixed" => Ok(NumType::Fixed),
            _ => Err(ParseNumTypeError),
        }
    }
}

impl ToString for NumType {
    fn to_string(&self) -> String {
        match self {
            NumType::Float => "float".to_string(),
            NumType::Fixed => "fixed".to_string(),
        }
    }
}


#[derive(Debug)]
struct ParseFileTypeError;

impl fmt::Display for ParseFileTypeError {
    fn fmt(&self, f: &mut fmt:: Formatter<'_>) -> fmt::Result {
        write!(f, "invalid file type")
    }
}

impl Error for ParseFileTypeError {}

#[derive(Debug, PartialEq, Clone, Copy)] 
enum FileType {
    Binary,
    Hex,
    Decimal,
}

impl std::str::FromStr for FileType {
    type Err = ParseNumTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "binary" => Ok(FileType::Binary),
            "hex" => Ok(FileType::Hex),
            "decfloat" => Ok(FileType::Decimal),
            _ => Err(ParseNumTypeError),
        }
    }
}

impl ToString for FileType {
    fn to_string(&self) -> String {
        match self {
            FileType::Hex => "hex".to_string(),
            FileType::Binary => "binary".to_string(),
            FileType::Decimal => "decimal".to_string(),
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

    /// num type to convert from
    #[argh(option)]
    fromnum: NumType,

    /// file type to convert from
    #[argh(option)]
    fromfile: FileType,

    /// num type to convert to
    #[argh(option)]
    tonum: NumType,

    /// file type to convert to
    #[argh(option)]
    tofile: FileType,

    /// optional exponent for fixed_to_binary -> default is -1
    #[argh(option, default = "-1")]
    exp: i64,

    /// optional for fixed_to_binary using bit slicing. If choosen, will use bit slicing.
    #[argh(switch, short = 'b')]
    bits: bool,

    /// optional for use with to_binary and from_binary
    #[argh(option, default = "0")]
    width: usize,

    /// optional flag to force the inter-rep path
    #[argh(switch, short = 'i')]
    inter: bool
}

fn main() {
    let args: Arguments = argh::from_env();

    convert(
        &args.from,
        &args.to,
        args.fromnum,
        args.fromfile,
        args.tonum,
        args.tofile,
        args.exp,
        args.bits,
        args.width,
        args.inter,
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
/// * `exponent` - An `i64` value used as the exponent for conversions involving fixed-point numbers.
///
/// # Returns
///
/// Returns `Ok(())` if the conversion and file writing operations are successful,
/// or an `Err` if an I/O error occurs during the process.
fn convert(
    filepath_get: &String,
    filepath_send: &Option<String>,
    fromnum: NumType,
    fromfile: FileType,
    tonum: NumType,
    tofile: FileType,
    exponent: i64,
    bits: bool,
    width: usize,
    inter: bool,
) {
    // Create the output file if filepath_send is Some
    let mut converted: Option<File> = filepath_send
        .as_ref()
        .map(|path| File::create(path).expect("creation failed"));

    match (fromnum, tonum) {
        (NumType:: Float, NumType::Float) => {
            match(fromfile, tofile){
                (FileType:: Hex, FileType::Binary) => {
                    for line in read_to_string(filepath_get).unwrap().lines() {
                        if line.len() <= FAST_TRACK_THRESHOLD_HEX_TO_BINARY && !inter{
                            hex_to_binary(line, &mut converted)
                                .expect("Failed to write binary to file");
                        } else {
                            to_binary(from_hex(line, width), &mut converted, width)
                                .expect("Failed to write binary to file");
                        }
                    }
                }
                (FileType:: Decimal, FileType::Binary)=>{
                    for line in read_to_string(filepath_get).unwrap().lines() {
                        if line.len() <= FAST_TRACK_THRESHOLD_FLOAT_TO_BINARY && !inter {
                            float_to_binary(line, &mut converted)
                                .expect("Failed to write binary to file");
                        } else {
                            to_binary(from_float(line), &mut converted, width)
                                .expect("Failed to write binary to file");
                        }
                    }
                }
                (FileType:: Binary, FileType::Hex)=>{
                    for line in read_to_string(filepath_get).unwrap().lines() {
                        print!("used fastpath");
                        if line.len() <= FAST_TRACK_THRESHOLD_BINARY_TO_HEX && !inter {
                            binary_to_hex(line, &mut converted)
                                .expect("Failed to write hex to file");
                        } else {
                            print!("used intermediate");
                            to_hex(from_binary(line, width), &mut converted)
                                .expect("Failed to write binary to file");
                        }
                    }
                }
                (FileType:: Binary, FileType::Decimal)=>{
                    for line in read_to_string(filepath_get).unwrap().lines() {
                        if line.len() <= FAST_TRACK_THRESHOLD_BINARY_TO_FLOAT && !inter {
                            binary_to_float(line, &mut converted)
                                .expect("Failed to write float to file");
                        } else {
                            to_float(from_binary(line, width), &mut converted)
                                .expect("Failed to write binary to file");
                        }
                    }
                }
                (FileType:: Hex, FileType::Decimal)=>{
                    for line in read_to_string(filepath_get).unwrap().lines() {
                        to_float(from_hex(line, width), &mut converted)
                                .expect("Failed to write binary to file");
                    }
                }
                (FileType:: Decimal, FileType::Hex)=>{
                    for line in read_to_string(filepath_get).unwrap().lines() {
                        to_hex(from_float(line), &mut converted)
                                .expect("Failed to write binary to file");
                    }
                }
                (_, _)=>{
                    panic!("Invalid Conversion of File Types") 
                }
            }
        }
        _ => panic!(
            "Conversion from {} to {} is not supported",
            fromnum.to_string(),
            tonum.to_string()
        ),
    }
    if let Some(filepath) = filepath_send {
        eprintln!(
            "Successfully converted from {} to {} in {}",
            fromnum.to_string(),
            tonum.to_string(),
            filepath
        );
    } else {
        eprintln!(
            "Successfully converted from {} to {}",
            fromnum.to_string(),
            tonum.to_string(),
        );
    }
}

/// Formats [to_format] properly for float values
fn format_binary(to_format: u64) -> String {
    let binary_str = format!("{:064b}", to_format);
    format!(
        "{} {} {}",
        &binary_str[0..1], // Sign bit
        &binary_str[1..9], // Exponent
        &binary_str[9..]   // Significand
    )
}

fn format_hex(to_format: u64) -> String {
    format!("0x{:X}", to_format)
}

/// Converts a string representation of a floating-point number to its binary
/// format and appends the result to the specified file.
///
/// This function takes a string slice representing a floating-point number,
/// converts it to a 64-bit floating-point number (`f64`), then converts this
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
    let float_of_string: f64;
    // Convert string to float
    match float_string.parse::<f64>() {
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
/// converts it to a 64-bit integer (`u64`), then converts this number to its
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
/// 
/// This does not differentiate between floating and fixed. It just treats any hex as an integer.
fn hex_to_binary(
    hex_string: &str,
    filepath_send: &mut Option<File>,
) -> io::Result<()> {
    // Convert hex to binary
    let binary_of_hex = u64::from_str_radix(hex_string, 16)
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
/// converts it to a 64-bit integer (`u64`), then converts this number to its
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
    let hex_of_binary = u64::from_str_radix(binary_string, 2)
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
/// converts it to a 64-bit integer (`u64`), then interprets this integer as
/// the binary representation of a 64-bit floating-point number (`f64`).
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
    let binary_value = u64::from_str_radix(binary_string, 2)
        .expect("Failed to parse binary string");

    let formated_float_str = format!("{:?}", binary_value);

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
/// to a 64-bit integer, and then to its binary representation. The binary representation
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
    exp_int: i64,
) -> io::Result<()> {
    // Convert fixed value from string to int
    let fixed_value: f64;
    match fixed_string.parse::<f64>() {
        Ok(parsed_num) => fixed_value = parsed_num,
        Err(_) => {
            panic!("Bad fixed value input")
        }
    }

    //exponent int to float so we can multiply
    let exponent = exp_int as f64;

    // Exponent math
    let multiplied_fixed = fixed_value * 2_f64.powf(-exponent);

    // Convert to a 64-bit integer
    let multiplied_fixed_as_i64 = multiplied_fixed as i64;

    // Convert to a binary string with 64 bits
    let binary_of_fixed = format!("{:064b}", multiplied_fixed_as_i64);

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
/// converts it to a 64-bit unsigned integer, interprets this integer as
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
    exp_int: i64,
) -> io::Result<()> {
    // Convert binary value from string to int
    let binary_value = match u64::from_str_radix(binary_string, 2) {
        Ok(parsed_num) => parsed_num,
        Err(_) => panic!("Bad binary value input"),
    };

    // Convert to fixed
    let int_of_binary = binary_value as f64;

    //exponent int to float so we can multiply
    let exponent = exp_int as f64;

    // Exponent math
    let divided: f64 = int_of_binary / 2_f64.powf(-exponent);

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
    exp_int: i64,
) -> io::Result<()> {
    // Convert binary string to an integer (assuming binary_string is a valid binary representation)
    let binary_int = u64::from_str_radix(binary_string, 2).unwrap();

    // Adjust the binary point based on the exponent
    let mut result = binary_int;
    if exp_int < 0 {
        // If exponent is negative, shift right (multiply by 2^(-exp_int))
        result >>= -exp_int as u64;
    } else {
        // If exponent is positive, shift left (multiply by 2^(exp_int))
        result <<= exp_int as u64;
    }

    // Convert result to a fixed-point decimal representation
    let fixed_value = result as f64;

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

/// Converts a string representation of a binary number the intermediate representation.
///
/// This function takes a string slice representing a binary number,
/// converts it to a `BigUint`, determines the sign, and constructs an
/// `IntermediateRepresentation` containing the sign, mantissa, and exponent.
///
/// # Arguments
///
/// * `binary_string` - A string slice containing the binary number to be converted.
/// * `bit_width` - A number representing the width of each binary number
///
/// # Returns
///
/// This function returns an `IntermediateRepresentation` containing the sign, mantissa,
/// and exponent of the binary number.
///
/// # Panics
///
/// This function will panic if the input string cannot be parsed as a binary number.

fn from_binary(
    binary_string: &str,
    bit_width: usize,
) -> IntermediateRepresentation {
    let sign = binary_string.starts_with('0');

    let binary_value = BigUint::from_str_radix(binary_string, 2)
        .expect("Invalid binary string");

    let mantissa = if sign {
        binary_value
    } else {
        // Calculate the two's complement for negative values
        let max_value = BigUint::from(1u64) << bit_width;
        &max_value - &binary_value
    };

    IntermediateRepresentation {
        sign,
        mantissa,
        exponent: 0,
    }
}

/// Converts the intermediate representation to a binary string and writes it to a file or stdout.
///
/// This function takes an `IntermediateRepresentation`, converts the mantissa to a `BigInt`
/// applying the sign, converts the resulting `BigInt` to a binary string, and writes
/// the binary string to the specified file or stdout.
///
/// # Arguments
///
/// * `inter_rep` - The intermediate representation to be converted.
/// * `filepath_send` - A mutable reference to an optional `File` where the binary string
///   will be written. If `None`, the result is written to stdout.
/// * `bit_width` - A number representing the width of each binary number
///
/// # Returns
///
/// This function returns a `std::io::Result<()>` which is `Ok` if the operation
/// is successful, or an `Err` if an I/O error occurs while writing to the file.

fn to_binary(
    inter_rep: IntermediateRepresentation,
    filepath_send: &mut Option<File>,
    bit_width: usize, // Bit width specified by the user
) -> io::Result<()> {
    let inter_value = if inter_rep.sign {
        BigInt::from(inter_rep.mantissa)
    } else {
        -BigInt::from(inter_rep.mantissa)
    };

    // Convert the value to a binary string
    let mut binary_str = inter_value.to_str_radix(2);

    // Handle two's complement for negative numbers
    if inter_value < BigInt::from(0) {
        let max_value = BigInt::from(1) << bit_width;
        let two_complement = max_value + inter_value;
        binary_str = two_complement.to_str_radix(2);
    }

    // At this point, binary_str should already be at the correct bit width
    // No padding or truncation is necessary

    // Write to file or stdout
    if let Some(file) = filepath_send {
        file.write_all(binary_str.as_bytes())?;
        file.write_all(b"\n")?;
    } else {
        std::io::stdout().write_all(binary_str.as_bytes())?;
        std::io::stdout().write_all(b"\n")?;
    }

    Ok(())
}

/// Converts a string representation of a floating-point number to the intermediate representation.
///
/// This function takes a string slice representing a floating-point number,
/// splits it into integer and fractional parts, constructs the mantissa,
/// sets the exponent based on the length of the fractional part, and
/// constructs an `IntermediateRepresentation` containing the sign, mantissa, and exponent.
///
/// # Arguments
///
/// * `float_string` - A string slice containing the floating-point number to be converted.
///
/// # Returns
///
/// This function returns an `IntermediateRepresentation` containing the sign, mantissa,
/// and exponent of the floating-point number.
///
/// # Panics
///
/// This function will panic if the input string cannot be parsed as a number.
fn from_float(float_string: &str) -> IntermediateRepresentation {
    let sign = !float_string.starts_with('-');
    let float_trimmed = float_string.trim_start_matches('-');

    let parts: Vec<&str> = float_trimmed.split('.').collect();
    let integer_part = parts[0];
    let fractional_part = if parts.len() > 1 { parts[1] } else { "0" };
    // Prob not the best way to do this
    let mantissa_string = format!("{integer_part}{fractional_part}");

    IntermediateRepresentation {
        sign,
        mantissa: BigUint::from_str(&mantissa_string).expect("Invalid number"),
        exponent: -(fractional_part.len() as i64),
    }
}

/// Converts the intermediate representation to a floating-point number string and writes it to a file or stdout.
///
/// This function takes an `IntermediateRepresentation`, converts the mantissa to a string,
/// inserts the decimal point at the correct position based on the exponent, constructs
/// the floating-point number string applying the sign, and writes the resulting string
/// to the specified file or stdout.
///
/// # Arguments
///
/// * `inter_rep` - The intermediate representation to be converted.
/// * `filepath_send` - A mutable reference to an optional `File` where the floating-point
///   number string will be written. If `None`, the result is written to stdout.
///
/// # Returns
///
/// This function returns a `std::io::Result<()>` which is `Ok` if the operation
/// is successful, or an `Err` if an I/O error occurs while writing to the file.

fn to_float(
    inter_rep: IntermediateRepresentation,
    filepath_send: &mut Option<File>,
) -> io::Result<()> {
    let mut mantissa_str = inter_rep.mantissa.to_string();

    // Determine the position to insert the decimal point
    let mut decimal_pos = mantissa_str.len() as i64 - inter_rep.exponent;

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

/// Converts a string representation of a fixed-point number to the intermediate representation.
///
/// This function takes a string slice representing a fixed-point number and an exponent value,
/// determines the sign, constructs the mantissa, sets the exponent to the given value, and
/// constructs an `IntermediateRepresentation` containing the sign, mantissa, and exponent.
///
/// # Arguments
///
/// * `fixed_string` - A string slice containing the fixed-point number to be converted.
/// * `exp_int` - The exponent value for the fixed-point number.
///
/// # Returns
///
/// This function returns an `IntermediateRepresentation` containing the sign, mantissa,
/// and exponent of the fixed-point number.
///
/// # Panics
///
/// This function will panic if the input string cannot be parsed as a number.
fn from_fixed(fixed_string: &str, exp_int: i64) -> IntermediateRepresentation {
    let sign = !fixed_string.starts_with('-');
    let fixed_trimmed = fixed_string.trim_start_matches('-');

    let mantissa_string = fixed_trimmed.to_string();

    IntermediateRepresentation {
        sign,
        mantissa: BigUint::from_str(&mantissa_string).expect("Invalid number"),
        exponent: exp_int,
    }
}

/// Converts the intermediate representation to a fixed-point number string and writes it to a file or stdout.
///
/// This function takes an `IntermediateRepresentation`, computes the scale factor based on
/// the negative exponent, converts the mantissa to a `BigInt` and multiplies by the scale factor,
/// constructs the fixed-point number string applying the sign, and writes the resulting string
/// to the specified file or stdout.
///
/// # Arguments
///
/// * `inter_rep` - The intermediate representation to be converted.
/// * `filepath_send` - A mutable reference to an optional `File` where the fixed-point
///   number string will be written. If `None`, the result is written to stdout.
///
/// # Returns
///
/// This function returns a `std::io::Result<()>` which is `Ok` if the operation
/// is successful, or an `Err` if an I/O error occurs while writing to the file.
fn to_fixed(
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
    let adjusted_exponent = inter_rep.exponent + mantissa_len as i64;

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

/// Converts a string representation of a hexadecimal number to the intermediate representation.
///
/// This function takes a string slice representing a hexadecimal number,
/// converts it to a `BigUint`, determines the sign, and constructs an
/// `IntermediateRepresentation` containing the sign, mantissa, and exponent.
///
/// # Arguments
///
/// * `hex_string` - A string slice containing the hexadecimal number to be converted.
///
/// # Returns
///
/// This function returns an `IntermediateRepresentation` containing the sign, mantissa,
/// and exponent of the hexadecimal number.
///
/// # Panics
///
/// This function will panic if the input string cannot be parsed as a hexadecimal number.

fn from_hex(hex_string: &str, width: usize, float_or_fixed: NumType) -> IntermediateRepresentation {
    // Convert the cleaned hexadecimal string to BigUint
    let hex_value = BigUint::from_str_radix(hex_string, 16)
        .expect("Invalid hexadecimal string");

    // Determine if the value is negative based on the MSB
    let sign_bit = BigUint::from(1u64) << (width - 1);
    let sign = &hex_value & &sign_bit == BigUint::from(0u64);

    let mantissa = if sign {
        hex_value
    } else {
        // Calculate the two's complement for negative values
        let max_value = BigUint::from(1u64) << width;
        &max_value - &hex_value
    };

    IntermediateRepresentation {
        sign,
        mantissa,
        exponent: 0,
    }
}

/// Converts the intermediate representation to a hexadecimal string and writes it to a file or stdout.
///
/// This function takes an `IntermediateRepresentation`, converts the mantissa to a hexadecimal string,
/// applies the sign, and writes the resulting string to the specified file or stdout.
///
/// # Arguments
///
/// * `inter_rep` - The intermediate representation to be converted.
/// * `filepath_send` - A mutable reference to an optional `File` where the hexadecimal string
///   will be written. If `None`, the result is written to stdout.
///
/// # Returns
///
/// This function returns a `std::io::Result<()>` which is `Ok` if the operation
/// is successful, or an `Err` if an I/O error occurs while writing to the file.

fn to_hex(
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
