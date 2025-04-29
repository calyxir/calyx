use std::fs::File;
use std::io::stdout;
use std::io::{self, Write};


/// Formats [to_format] properly for float values
pub fn format_binary(to_format: u64) -> String {
    let binary_str = format!("{:064b}", to_format);
    format!(
        "{} {} {}",
        &binary_str[0..1], // Sign bit
        &binary_str[1..9], // Exponent
        &binary_str[9..]   // Significand
    )
}

pub fn format_hex(to_format: u64) -> String {
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
pub fn float_to_binary(
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
pub fn hex_to_binary(
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
pub fn binary_to_hex(
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
pub fn binary_to_float(
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
pub fn fixed_to_binary(
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
pub fn binary_to_fixed(
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

pub fn binary_to_fixed_bit_slice(
    binary_string: &str,
    filepath_send: &mut Option<File>,
    exp_int: i64,
) -> io::Result<()> {
    // Convert binary string to an integer
    let binary_int = match u64::from_str_radix(binary_string, 2) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("Invalid binary string: {}", binary_string);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid binary string"));
        }
    };

    // Adjust the binary point based on the exponent
    let fixed_value = if exp_int < 0 {
        // If exponent is negative, divide by 2^(-exp_int)
        binary_int as f64 / (1u64 << -exp_int) as f64
    } else {
        // If exponent is positive, multiply by 2^(exp_int)
        binary_int as f64 * (1u64 << exp_int) as f64
    };

    // Format the fixed-point value as a plain decimal
    let string_of_fixed = format!("{:.8}", fixed_value);

    if let Some(file) = filepath_send.as_mut() {
        // Write the fixed-point value to the file
        file.write_all(string_of_fixed.as_bytes())?;
        file.write_all(b"\n")?;
    } else {
        // Write the fixed-point value to standard output
        io::stdout().write_all(string_of_fixed.as_bytes())?;
        io::stdout().write_all(b"\n")?;
    }

    Ok(())
}
