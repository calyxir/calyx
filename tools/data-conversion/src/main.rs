use argh::FromArgs;
use num_bigint::BigUint;
use std::error::Error;
use std::fmt;
use std::fs::read_to_string;
use std::fs::File;
use std::io::stdout;
use std::io::{self, Write};
use std::str::FromStr;
mod u8vector;
mod ir;
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
                            ir::to_binary(ir::from_hex(line, width), &mut converted, width)
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
                            ir::to_binary(ir::from_float(line), &mut converted, width)
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
                            ir::to_hex(ir::from_binary(line, width), &mut converted)
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
                            ir::to_float(ir::from_binary(line, width), &mut converted)
                                .expect("Failed to write binary to file");
                        }
                    }
                }
                (FileType:: Hex, FileType::Decimal)=>{
                    for line in read_to_string(filepath_get).unwrap().lines() {
                        ir::to_float(ir::from_hex(line, width), &mut converted)
                                .expect("Failed to write binary to file");
                    }
                }
                (FileType:: Decimal, FileType::Hex)=>{
                    for line in read_to_string(filepath_get).unwrap().lines() {
                        ir::to_hex(ir::from_float(line), &mut converted)
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
