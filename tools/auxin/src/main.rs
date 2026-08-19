use argh::FromArgs;
use num_ir::typing;
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

mod io;
use crate::io::AuxinError;

const JSON_EXTENSION: &str = "data";
const CIDER_EXTENSION: &str = "dump";
const DAT_EXTENSION: &str = "dat";

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
    type Err = io::AuxinError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Formats::Json),
            "cider" | "dump" | "data-dump" => Ok(Formats::DataDump),
            "dat" | "verilog-dat" | "verilog" | "verilator" | "icarus" => {
                Ok(Formats::Dat)
            }
            _ => Err(io::AuxinError::ToArgErr(s.to_string())),
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
// TODO: maybe protect the file_extension? / separate read and write

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

fn main() -> Result<(), AuxinError> {
    let mut opts: Opts = argh::from_env();

    if opts.input_format.is_none() {
        let Some(ref p) = opts.input_path else {
            return Err(AuxinError::UnknownIn);
        };
        opts.input_format = infer_format(p);
    }
    if opts.output_format.is_none() {
        let Some(ref p) = opts.output_path else {
            return Err(AuxinError::UnknownTarget);
        };
        opts.output_format = infer_format(p);
    }

    let Some(in_fmt) = opts.input_format else {
        return Err(AuxinError::UnknownIn);
    };

    let Some(out_fmt) = opts.output_format else {
        return Err(AuxinError::UnknownTarget);
    };

    let mut loaded_ir = match in_fmt {
        Formats::Json => {
            let jh = conv_formats::json::JsonHandler { out_hex: false };
            io::file_read(&jh, &opts.input_path, "json")?
        }
        Formats::Dat => {
            // TODO: default 'dat' into untyped
            let Some(ref path) = opts.input_path else {
                return Err(AuxinError::UnknownTarget);
            };

            let dh = conv_formats::dat_dir::DirHandler;
            io::dir_read(&dh, path, opts.file_extension.clone())?
        }
        Formats::DataDump => {
            let ch = conv_formats::cider_dump::CiderHandler;
            io::file_read(&ch, &opts.input_path, "data_dump")?
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

    match out_fmt {
        Formats::Json => {
            let jh = conv_formats::json::JsonHandler { out_hex: opts.hex };
            io::file_write(&jh, loaded_ir, &opts.output_path, "json")?
        }
        Formats::Dat => {
            let Some(p) = opts.output_path else {
                return Err(AuxinError::MissingDatOutputPath);
            };

            let dh = conv_formats::dat_dir::DirHandler;
            io::dir_write(&dh, loaded_ir, &p, opts.file_extension.clone())?;
        }
        Formats::DataDump => {
            let ch = conv_formats::cider_dump::CiderHandler;
            io::file_write(&ch, loaded_ir, &opts.output_path, "data_dump")?
        }
    }

    Ok(())
}
