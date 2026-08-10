use argh::FromArgs;
use cider_serde::SerializationError;
use conv_formats::filerep::{DirSink, FileSink, TryReadFrom, TryWriteThrough};
use core::str;
use num_ir::typing;
use std::{
    fs::File,
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
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

impl From<conv_formats::filerep::FileFmtErr> for CiderDataConverterError {
    fn from(value: conv_formats::filerep::FileFmtErr) -> Self {
        Self::BadInternal(format!("filefmt: {value}"))
    }
}

impl From<typing::OpError> for CiderDataConverterError {
    fn from(value: typing::OpError) -> Self {
        Self::BadInternal(format!("typing / conversion: {value}"))
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
    op: Option<num_ir::typing::OpTypes>,

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

fn infer_format(path: &Path) -> Option<Formats> {
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

    let Some(in_fmt) = opts.input_format else {
        return Err(CiderDataConverterError::BadInTarget);
    };

    let mut loaded_ir = match in_fmt {
        Formats::Json => {
            let jh = conv_formats::json::JsonHandler { out_hex: false };
            jh.try_read(adapt_input(&opts))?
        }
        Formats::Dat => {
            // TODO: default 'dat' into untyped
            // NOTE: if a header does not exist, this will fail!
            let Some(ref path) = opts.input_path else {
                return Err(CiderDataConverterError::UnknownTarget);
            };
            let d = DirSink {
                path: path.to_path_buf(),
                ext: opts.file_extension.clone(),
            };

            let h = conv_formats::dat_dir::DirHandler;
            h.try_read(d)?
        }
        Formats::DataDump => {
            let ch = conv_formats::cider_dump::CiderHandler;
            ch.try_read(adapt_input(&opts))?
        }
    };

    if let Some(ref o) = opts.op {
        match o {
            typing::OpTypes::Truncate => unimplemented!(),
            typing::OpTypes::Bitcast => {
                for v in loaded_ir.mems.values_mut() {
                    let mut old_t = v.ty().clone();
                    old_t.class = typing::TypeClass::Bits;
                    v.bitcast(old_t)?
                }
            }
            typing::OpTypes::SignExtend => unimplemented!(),
        }
    }

    let Some(out_fmt) = opts.output_format else {
        return Err(CiderDataConverterError::UnknownTarget);
    };

    match out_fmt {
        Formats::Json => {
            let jh = conv_formats::json::JsonHandler { out_hex: opts.hex };
            jh.try_write(adapt_output(&opts), loaded_ir)?
        }
        Formats::Dat => {
            if opts.output_path.is_none() {
                return Err(CiderDataConverterError::MissingDatOutputPath);
            }
            let p = opts.output_path.unwrap();
            let d = DirSink {
                path: p.to_path_buf(),
                ext: opts.file_extension.clone(),
            };

            let h = conv_formats::dat_dir::DirHandler;
            h.try_write(d, loaded_ir)?;
        }
        Formats::DataDump => {
            let ch = conv_formats::cider_dump::CiderHandler;
            ch.try_write(adapt_output(&opts), loaded_ir)?
        }
    }

    Ok(())
}

fn adapt_input(opts: &Opts) -> FileSink {
    match &opts.input_path {
        Some(path) => FileSink::P(path.to_path_buf()),
        None => FileSink::Stream,
    }
}

fn adapt_output(opts: &Opts) -> FileSink {
    match &opts.output_path {
        Some(path) => FileSink::P(path.to_path_buf()),
        None => FileSink::Stream,
    }
}
