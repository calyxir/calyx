use argh::FromArgs;
use cider::serialization::{DataDump, SerializationError};
use core::str;
use data_conv_lib::{filerep::HintedTryToIR, *};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
    str::FromStr,
};
use thiserror::Error;

const JSON_EXTENSION: &str = "data";
const CIDER_EXTENSION: &str = "dump";
const DAT_EXTENSION: &str = "dat";

#[derive(Error)]
enum CiderDataConverterError {
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse \"to\" argument: {0}")]
    BadToArgument(String),

    #[error("internal: {0}")]
    BadInternal(String),

    #[error("Bad input target. Specify manually?")]
    BadInTarget,

    #[error(
        "Unable to guess the conversion target. Please specify the target using the \"--to\" argument"
    )]
    UnknownTarget,

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error(transparent)]
    DataDumpError(#[from] SerializationError),

    #[error(
        "Missing output path. This is required for the \"to dat\" conversion"
    )]
    MissingDatOutputPath,
    // #[error("Output path for \"to dat\" exists but it is a file")]
    // DatOutputPathIsFile,
}

impl std::fmt::Debug for CiderDataConverterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl From<filerep::FileFmtErr> for CiderDataConverterError {
    fn from(value: filerep::FileFmtErr) -> Self {
        let filerep::FileFmtErr::FileSpecific(f) = value;

        Self::BadInternal(format!("filefmt: {f}"))
    }
}

impl From<numrep::CheckedConvErr> for CiderDataConverterError {
    fn from(value: numrep::CheckedConvErr) -> Self {
        Self::BadInternal(format!("conversion: {value}"))
    }
}

/// What are we converting the input to
#[derive(Debug, Clone, Copy)]
enum Formats {
    /// Cider's Single-file DataDump format
    DataDump,
    /// Verilator/icarus directory format
    Dat,
    /// Human readable output JSON
    Json,
}

impl FromStr for Formats {
    type Err = CiderDataConverterError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Formats::Json),
            "cider" | "dump" | "data-dump" => Ok(Formats::DataDump),
            "dat" | "verilog-dat" | "verilog" | "verilator" | "icarus" => {
                Ok(Formats::Dat)
            }
            _ => Err(CiderDataConverterError::BadToArgument(s.to_string())),
        }
    }
}

#[derive(FromArgs)]
/// Convert json data files to Cider DataDumps and vice-versa
struct Opts {
    /// the input file to be converted. If not provided, it will read from stdin
    #[argh(positional)]
    input_path: Option<PathBuf>,

    #[argh(option, short = 'f', long = "from")]
    /// format of input
    input_format: Option<Formats>,

    /// the output file to be written. If not provided, it will write to stdout
    #[argh(option, short = 'o')]
    output_path: Option<PathBuf>,

    /// format of desired output
    #[argh(option, short = 't', long = "to")]
    output_format: Option<Formats>,

    /// operation to perform
    #[argh(option, short = 'p', long = "op")]
    op: Option<numrep::OpTypes>,

    /// whether to output everything as hex, not erasing types
    #[argh(switch, short = 'x')]
    hex: bool,

    /// the file extension to use for the output/input file when parsing to and
    /// from the dat target. If not provided, the extension is assumed to be .dat
    #[argh(option, short = 'e', long = "dat-file-extension")]
    #[argh(default = "String::from(DAT_EXTENSION)")]
    file_extension: String,
}

// TODO: not having round_float may present problems
// TODO: not having use_quotes may present problems

fn infer_format(path: &PathBuf) -> Option<Formats> {
    if path.is_dir() {
        Some(Formats::Dat)
    } else if path.extension().is_some_and(|x| x == JSON_EXTENSION) {
        Some(Formats::Json)
    } else if path.extension().is_some_and(|x| x == CIDER_EXTENSION) {
        Some(Formats::DataDump)
    } else {
        None
    }
}

fn main() -> Result<(), CiderDataConverterError> {
    let mut opts: Opts = argh::from_env();

    if opts.input_format.is_none() {
        let Some(ref p) = opts.input_path else {
            return Err(CiderDataConverterError::BadInTarget);
        };
        opts.input_format = infer_format(p);
    }
    if opts.output_format.is_none() {
        let Some(ref p) = opts.output_path else {
            return Err(CiderDataConverterError::UnknownTarget);
        };
        opts.output_format = infer_format(p);
    }

    use filerep::{FileIO, TryFromIR};

    let Some(in_fmt) = opts.input_format else {
        return Err(CiderDataConverterError::BadInTarget);
    };

    let mut loaded_ir = match in_fmt {
        Formats::Json => {
            let input = get_read_handle(&opts)?;
            let parsed_json = json::JsonData::read_into(input)?;
            parsed_json.hinted_try_to_ir()?
        }
        Formats::Dat => {
            // TODO: default 'dat' into untyped
            // NOTE: if a header does not exist, this will fail!
            let Some(ref path) = opts.input_path else {
                return Err(CiderDataConverterError::UnknownTarget);
            };
            use filerep::DirIO;
            let dump = DataDump::read_into_dir(
                path.clone(),
                opts.file_extension.clone(),
            )?;
            dump.hinted_try_to_ir()?
        }
        Formats::DataDump => {
            let input = get_read_handle(&opts)?;
            let parsed_dump = DataDump::read_into(input)?;
            parsed_dump.hinted_try_to_ir()?
        }
    };

    if let Some(ref o) = opts.op {
        match o {
            numrep::OpTypes::Truncate => unimplemented!(),
            numrep::OpTypes::Bitcast => {
                for (_, v) in loaded_ir.mems.iter_mut() {
                    let mut old_t = v.ty().clone();
                    old_t.class = numrep::TypeClass::Bits;
                    v.bitcast(old_t)?
                }
            }
            numrep::OpTypes::SignExtend => unimplemented!(),
        }
    }

    let Some(out_fmt) = opts.output_format else {
        return Err(CiderDataConverterError::UnknownTarget);
    };

    match out_fmt {
        Formats::Json => {
            let jd = if opts.hex {
                json::JsonData::try_from_ir_fmt(
                    &loaded_ir,
                    &filerep::OutputOpts { print_hex: true },
                )?
            } else {
                json::JsonData::try_from_ir(&loaded_ir)?
            };
            let output = get_output_handle(&opts)?;
            jd.write_out(output)?;
        }
        Formats::Dat => {
            // let output = get_output_handle(&opts)?;
            if opts.output_path.is_none() {
                return Err(CiderDataConverterError::MissingDatOutputPath);
            }
            let dd = DataDump::try_from_ir(&loaded_ir)?;
            use filerep::DirIO;
            dd.write_out_dir(opts.output_path.unwrap(), opts.file_extension)?;
        }
        Formats::DataDump => {
            let output = get_output_handle(&opts)?;
            let dd = DataDump::try_from_ir(&loaded_ir)?;

            dd.write_out(output)?;
        }
    }

    Ok(())
}

fn get_output_handle(
    opts: &Opts,
) -> Result<Box<dyn Write>, CiderDataConverterError> {
    let output: Box<dyn Write> = opts
        .output_path
        .as_ref()
        .map(|path| File::create(path).map(|x| Box::new(x) as Box<dyn Write>))
        .unwrap_or(Ok(Box::new(io::stdout())))?;
    Ok(output)
}

fn get_read_handle(
    opts: &Opts,
) -> Result<Box<dyn Read>, CiderDataConverterError> {
    let input: Box<dyn Read> = opts
        .input_path
        .as_ref()
        .map(|path| File::open(path).map(|x| Box::new(x) as Box<dyn Read>))
        .unwrap_or(Ok(Box::new(io::stdin())))?;
    Ok(input)
}
