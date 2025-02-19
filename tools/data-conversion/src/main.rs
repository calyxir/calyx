use argh::FromArgs;
use std::error::Error;
use std::fmt;
use std::fs::read_to_string;
use std::fs::File;
use std::str::FromStr;
mod fast_track;
mod ir;
mod u8vector;
//cargo run -- --from $PATH1 --to $PATH2 --ftype "from" --totype "to"

// Thresholds for using fast-track functions
// const FAST_TRACK_THRESHOLD_BINARY_TO_FIXED: usize = 53; //52 bits for the significand (plus 1 implicit bit)b
// const FAST_TRACK_THRESHOLD_FIXED_TO_BINARY: usize = 53;
const FAST_TRACK_THRESHOLD_FLOAT_TO_BINARY: usize = 53;
const FAST_TRACK_THRESHOLD_BINARY_TO_FLOAT: usize = 53;
const FAST_TRACK_THRESHOLD_HEX_TO_BINARY: usize = 64;
const FAST_TRACK_THRESHOLD_BINARY_TO_HEX: usize = 64;

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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

    /// optional exponent for fixed_to_binary -> default is -0
    #[argh(option, default = "0")]
    exp: i64,

    /// optional for fixed_to_binary using bit slicing. If choosen, will use bit slicing.
    #[argh(switch, short = 'b')]
    bits: bool,

    /// optional for use with to_binary and from_binary
    #[argh(option, default = "0")]
    width: usize,

    /// optional for use with to_binary and from_binary
    #[argh(option, default = "0")]
    exp_width: usize,

    /// optional for use with to_binary and from_binary
    #[argh(option, default = "0")]
    mant_width: usize,

    /// optional flag to force the inter-rep path
    #[argh(switch, short = 'i')]
    inter: bool,

    /// optional flag - when flagged, will not use two's complement for binary
    #[argh(switch, short = 't')]
    twos: bool,
}

fn main() {
    let args: Arguments = argh::from_env();

    convert(&args);
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
fn convert(args: &Arguments) {
    // Use `args` to access all the required fields
    let filepath_get = &args.from;
    let filepath_send = &args.to;
    let fromnum = args.fromnum;
    let fromfile = args.fromfile;
    let tonum = args.tonum;
    let tofile = args.tofile;
    let exponent = args.exp;
    let bits = args.bits;
    let width = args.width;
    let inter = args.inter;
    let twos = args.twos;
    let exp_width = args.exp_width;
    let mant_width = args.mant_width;
    // Create the output file if filepath_send is Some
    let mut converted: Option<File> = filepath_send
        .as_ref()
        .map(|path| File::create(path).expect("creation failed"));

    match (fromnum, tonum) {
        (NumType::Fixed, NumType::Fixed) => match (fromfile, tofile) {
            (FileType::Decimal, FileType::Binary) => {
                for line in read_to_string(filepath_get).unwrap().lines() {
                    if line.len() <= FAST_TRACK_THRESHOLD_FLOAT_TO_BINARY
                        && !inter
                    {
                        fast_track::fixed_to_binary(
                            line,
                            &mut converted,
                            exponent,
                        )
                        .expect("Failed to write binary to file");
                    } else {
                        ir::to_binary(
                            ir::from_float(line),
                            &mut converted,
                            width,
                        )
                        .expect("Failed to write binary to file");
                    }
                }
            }
            (FileType::Binary, FileType::Decimal) => {
                for line in read_to_string(filepath_get).unwrap().lines() {
                    if line.len() <= FAST_TRACK_THRESHOLD_FLOAT_TO_BINARY
                        && !inter
                    {
                        fast_track::binary_to_fixed(
                            line,
                            &mut converted,
                            exponent,
                        )
                        .expect("Failed to write binary to file");
                    } else if line.len() <= FAST_TRACK_THRESHOLD_FLOAT_TO_BINARY
                        && bits
                    {
                        fast_track::binary_to_fixed_bit_slice(
                            line,
                            &mut converted,
                            exponent,
                        )
                        .expect("Failed to write binary to file");
                    } else {
                        ir::to_float(
                            ir::from_binary(line, width, twos),
                            &mut converted,
                        )
                        .expect("Failed to write binary to file");
                    }
                }
            }
            (FileType::Binary, FileType::Hex) => {
                for line in read_to_string(filepath_get).unwrap().lines() {
                    let u8vec = u8vector::binary_to_u8_vec(line)
                        .expect("Failed to write hex to file");
                    let ir_input =
                        u8vector::u8_to_ir_fixed(Ok(u8vec), exponent, twos);
                    ir::to_hex(ir_input, &mut converted)
                        .expect("Failed to write binary to file");
                }
            }
            (FileType::Hex, FileType::Binary) => {
                for line in read_to_string(filepath_get).unwrap().lines() {
                    let u8vec = u8vector::hex_to_u8_vec(line)
                        .expect("Failed to write hex to file");
                    let ir_input =
                        u8vector::u8_to_ir_fixed(Ok(u8vec), exponent, twos);
                    ir::to_binary(ir_input, &mut converted, width)
                        .expect("Failed to write binary to file");
                }
            }
            (_, _) => {
                panic!("Invalid Conversion of File Types")
            }
        },
        (NumType::Float, NumType::Float) => match (fromfile, tofile) {
            (FileType::Hex, FileType::Binary) => {
                for line in read_to_string(filepath_get).unwrap().lines() {
                    if line.len() <= FAST_TRACK_THRESHOLD_HEX_TO_BINARY
                        && !inter
                    {
                        fast_track::hex_to_binary(line, &mut converted)
                            .expect("Failed to write binary to file");
                    } else {
                        ir::to_binary(
                            ir::from_hex(line, width),
                            &mut converted,
                            width,
                        )
                        .expect("Failed to write binary to file");
                    }
                }
            }
            (FileType::Decimal, FileType::Binary) => {
                for line in read_to_string(filepath_get).unwrap().lines() {
                    if line.len() <= FAST_TRACK_THRESHOLD_FLOAT_TO_BINARY
                        && !inter
                    {
                        fast_track::float_to_binary(line, &mut converted)
                            .expect("Failed to write binary to file");
                    } else {
                        ir::to_binary(
                            ir::from_float(line),
                            &mut converted,
                            width,
                        )
                        .expect("Failed to write binary to file");
                    }
                }
            }
            (FileType::Binary, FileType::Hex) => {
                for line in read_to_string(filepath_get).unwrap().lines() {
                    print!("used fastpath");
                    if line.len() <= FAST_TRACK_THRESHOLD_BINARY_TO_HEX
                        && !inter
                    {
                        fast_track::binary_to_hex(line, &mut converted)
                            .expect("Failed to write hex to file");
                    } else {
                        let u8vec = u8vector::binary_to_u8_vec(line)
                            .expect("Failed to write hex to file");
                        let ir_input = u8vector::u8_to_ir_float(
                            Ok(u8vec),
                            exp_width.try_into().unwrap(),
                            mant_width.try_into().unwrap(),
                            twos,
                        );
                        ir::to_hex(ir_input, &mut converted)
                            .expect("Failed to write binary to file");
                    }
                }
            }
            (FileType::Binary, FileType::Decimal) => {
                for line in read_to_string(filepath_get).unwrap().lines() {
                    if line.len() <= FAST_TRACK_THRESHOLD_BINARY_TO_FLOAT
                        && !inter
                    {
                        fast_track::binary_to_float(line, &mut converted)
                            .expect("Failed to write float to file");
                    } else {
                        ir::to_float(
                            ir::from_binary(line, width, twos),
                            &mut converted,
                        )
                        .expect("Failed to write binary to file");
                    }
                }
            }
            (FileType::Hex, FileType::Decimal) => {
                for line in read_to_string(filepath_get).unwrap().lines() {
                    ir::to_float(ir::from_hex(line, width), &mut converted)
                        .expect("Failed to write binary to file");
                }
            }
            (FileType::Decimal, FileType::Hex) => {
                for line in read_to_string(filepath_get).unwrap().lines() {
                    ir::to_hex(ir::from_float(line), &mut converted)
                        .expect("Failed to write binary to file");
                }
            }
            (_, _) => {
                panic!("Invalid Conversion of File Types")
            }
        },
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
