use num_bigint::BigInt;
use num_bigint::BigUint;
use num_traits::Num;
use num_traits::Zero;
use std::fs::File;
use std::io::{self, Write};
use std::str::FromStr;

/// Enum representing special cases for binary numbers.
#[derive(Debug, PartialEq, Eq)]
pub enum SpecialCase {
    None,
    Zero,
    Infinity,
    NaN,
}

/// * 'sign' - `true` indicates that the value is negative; `false` indicates that it is positive.
/// * 'mantissa' - The absolute value represented as an integer without a decimal point.
/// * 'exponent' - The exponent to apply to the mantissa, where the actual value is calculated as `mantissa * 2^exponent`. The exponent can be negative.
/// * 'special_case' - Represents any special cases like zero, infinity, NaN, etc.
pub struct IntermediateRepresentation {
    pub sign: bool,
    pub mantissa: BigUint,
    pub exponent: i64,
    pub special_case: SpecialCase,
}

impl IntermediateRepresentation {
    /// Determines the special case for the current representation.
    pub fn determine_special_case(&mut self, bit_width: usize) {
        let max_exponent = (1 << (bit_width - 1)) - 1; // All exponent bits set to 1
        let is_exponent_all_ones = self.exponent == max_exponent as i64;
        let is_mantissa_zero = self.mantissa.is_zero();

        self.special_case = if is_exponent_all_ones && is_mantissa_zero {
            SpecialCase::Infinity
        } else if is_exponent_all_ones && !is_mantissa_zero {
            SpecialCase::NaN
        } else if self.mantissa.is_zero() {
            SpecialCase::Zero
        } else {
            SpecialCase::None
        };
    }
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

pub fn from_binary(
    binary_string: &str,
    bit_width: usize,
    twos_comp: bool,
) -> IntermediateRepresentation {
    let sign = if !twos_comp {
        false
    } else {
        binary_string.starts_with('1')
    };

    let binary_value = BigUint::from_str_radix(binary_string, 2)
        .expect("Invalid binary string");

    let mantissa = if sign {
        binary_value
    } else {
        // Calculate the two's complement for negative values
        let max_value = BigUint::from(1u64) << bit_width;
        &max_value - &binary_value
    };

    let mut ir = IntermediateRepresentation {
        sign,
        mantissa,
        exponent: 0,
        special_case: SpecialCase::None,
    };

    // Determine if the IR is a special case
    ir.determine_special_case(bit_width);

    ir
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

pub fn to_binary(
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
pub fn from_float(float_string: &str) -> IntermediateRepresentation {
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
        special_case: SpecialCase::None,
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

pub fn to_dec_float(
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
// pub fn from_fixed(
//     fixed_string: &str,
//     exp_int: i64,
// ) -> IntermediateRepresentation {
//     let sign = !fixed_string.starts_with('-');
//     let fixed_trimmed = fixed_string.trim_start_matches('-');

//     let mantissa_string = fixed_trimmed.to_string();

//     IntermediateRepresentation {
//         sign,
//         mantissa: BigUint::from_str(&mantissa_string).expect("Invalid number"),
//         exponent: exp_int,
//     }
// }

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
pub fn to_dec_fixed(
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
    write_to_output(&string, filepath_send)
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

pub fn from_hex(hex_string: &str, width: usize) -> IntermediateRepresentation {
    let hex_value = BigUint::from_str_radix(hex_string, 16)
        .expect("Invalid hexadecimal string");

    let sign_bit = BigUint::from(1u64) << (width - 1);
    let sign = &hex_value & &sign_bit == BigUint::from(0u64);

    let mantissa = if sign {
        hex_value
    } else {
        // Calculate the two's complement for negative values
        let max_value = BigUint::from(1u64) << width;
        &max_value - &hex_value
    };

    let mut ir = IntermediateRepresentation {
        sign,
        mantissa,
        exponent: 0,
        special_case: SpecialCase::None,
    };

    // Determine if the IR is a special case
    ir.determine_special_case(width);

    ir
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

pub fn to_hex(
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

fn write_to_output(
    string: &str,
    filepath_send: &mut Option<File>,
) -> io::Result<()> {
    if let Some(file) = filepath_send.as_mut() {
        file.write_all(string.as_bytes())?;
        file.write_all(b"\n")?;
    } else {
        io::stdout().write_all(string.as_bytes())?;
        io::stdout().write_all(b"\n")?;
    }
    Ok(())
}
