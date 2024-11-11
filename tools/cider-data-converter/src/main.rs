use argh::FromArgs;
use cider_data_converter::{converter, json_data::JsonData};
use core::str;
use interp::serialization::{self, DataDump, SerializationError};
use itertools::Itertools;
use std::{
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    path::PathBuf,
    str::FromStr,
};
use thiserror::Error;

const JSON_EXTENSION: &str = "data";
const CIDER_EXTENSION: &str = "dump";

const HEADER_FILENAME: &str = "header";

#[derive(Error)]
enum CiderDataConverterError {
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse \"to\" argument: {0}")]
    BadToArgument(String),

    #[error("Unable to guess the conversion target. Please specify the target using the \"--to\" argument")]
    UnknownTarget,

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error(transparent)]
    DataDumpError(#[from] SerializationError),
}

impl std::fmt::Debug for CiderDataConverterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// What are we converting the input to
#[derive(Debug, Clone, Copy)]
enum Target {
    /// Cider's Single-file DataDump format
    DataDump,
    /// Verilator/icarus directory format
    Dat,
    /// Human readable output JSON
    Json,
}

impl FromStr for Target {
    type Err = CiderDataConverterError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Target::Json),
            "cider" | "dump" | "data-dump" => Ok(Target::DataDump),
            "dat" | "verilog-dat" | "verilog" | "verilator" | "icarus" => {
                Ok(Target::Dat)
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
    /// the output file to be written. If not provided, it will write to stdout
    #[argh(option, short = 'o')]
    output_path: Option<PathBuf>,

    /// whether to round un-representable floating point instantiations rather than
    /// throwing an error
    #[argh(switch, short = 'r', long = "round-float")]
    round_float: bool,

    /// optional specification of what action to perform. Can be "cider" or
    /// "json". If not provided, the converter will try to guess based on file names
    #[argh(option, short = 't', long = "to")]
    action: Option<Target>,

    /// whether to use quotes around floating point numbers in the output. This
    /// exists solely for backwards compatibility with the old display format.
    #[argh(switch, long = "legacy-quotes")]
    use_quotes: bool,
}

fn main() -> Result<(), CiderDataConverterError> {
    let mut opts: Opts = argh::from_env();

    // if no action is specified, try to guess based on file extensions
    if opts.action.is_none()
        && (opts.input_path.as_ref().is_some_and(|x| {
            x.extension().map_or(false, |y| y == JSON_EXTENSION)
        }) || opts.output_path.as_ref().is_some_and(|x| {
            x.extension().map_or(false, |y| y == CIDER_EXTENSION)
        }))
    {
        opts.action = Some(Target::DataDump);
    } else if opts.action.is_none()
        && (opts.output_path.as_ref().is_some_and(|x| {
            x.extension().map_or(false, |x| x == JSON_EXTENSION)
        }) || opts.input_path.as_ref().is_some_and(|x| {
            x.extension().map_or(false, |x| x == CIDER_EXTENSION)
        }))
    {
        opts.action = Some(Target::Json);
    }

    if let Some(action) = opts.action {
        match action {
            Target::DataDump => {
                let (mut input, mut output) = get_io_handles(&opts)?;

                let parsed_json: JsonData =
                    serde_json::from_reader(&mut input)?;
                converter::convert_to_data_dump(&parsed_json, opts.round_float)
                    .serialize(&mut output)?;
            }
            Target::Json => {
                let data_dump = if let Some(path) = &opts.input_path {
                    if path.is_dir() {
                        // we are converting from a dat directory rather than a
                        // dump

                        let header = {
                            let mut header_file =
                                File::open(path.join(HEADER_FILENAME))?;
                            let mut raw_header = vec![];
                            header_file.read_to_end(&mut raw_header)?;

                            serialization::DataHeader::deserialize(&raw_header)?
                        };

                        let mut data: Vec<u8> = vec![];

                        for mem_dec in &header.memories {
                            let starting_len = data.len();
                            let mem_file = BufReader::new(File::open(
                                path.join(&mem_dec.name),
                            )?);

                            let mut line_data = vec![];
                            for line in mem_file.lines() {
                                let line = line?;
                                for pair in &line.chars().chunks(2) {
                                    let string =
                                        pair.into_iter().collect::<String>();
                                    let val = u8::from_str_radix(&string, 16)
                                        .expect("invalid hex");
                                    line_data.push(val);
                                }
                                // TODO griffin: handle inputs that are
                                // truncated or otherwise shorter than expected

                                assert!(
                                    line_data.len()
                                        == (mem_dec.bytes_per_entry() as usize)
                                );
                                // reverse the byte order to get the expected
                                // little endian and reuse the vec
                                data.extend(line_data.drain(..).rev())
                            }

                            assert_eq!(
                                data.len() - starting_len,
                                mem_dec.byte_count()
                            );
                        }

                        DataDump { header, data }
                    } else {
                        // we are converting from a dump file
                        serialization::DataDump::deserialize(
                            &mut get_read_handle(&opts)?,
                        )?
                    }
                } else {
                    // we are converting from a dump file
                    serialization::DataDump::deserialize(&mut get_read_handle(
                        &opts,
                    )?)?
                };

                let mut output = get_output_handle(&opts)?;

                let json_data = converter::convert_from_data_dump(
                    &data_dump,
                    opts.use_quotes,
                );
                writeln!(
                    &mut output,
                    "{}",
                    serde_json::to_string_pretty(&json_data)?
                )?;
            }
            Target::Dat => {
                let mut input = get_read_handle(&opts)?;
                let parsed_json: JsonData =
                    serde_json::from_reader(&mut input)?;
                let data = converter::convert_to_data_dump(
                    &parsed_json,
                    opts.round_float,
                );

                if let Some(path) = opts.output_path {
                    if path.exists() && !path.is_dir() {
                        // TODO griffin: Make this an actual error
                        panic!("Output path exists but is not a directory")
                    } else if !path.exists() {
                        std::fs::create_dir(&path)?;
                    }

                    let mut header_output = File::create(path.join("header"))?;
                    header_output.write_all(&data.header.serialize()?)?;

                    for memory in &data.header.memories {
                        let file = File::create(path.join(&memory.name))?;
                        let mut writer = BufWriter::new(file);
                        for bytes in data
                            .get_data(&memory.name)
                            .unwrap()
                            .chunks_exact(memory.width().div_ceil(8) as usize)
                        {
                            // data file seems to expect lsb on the right
                            // for the moment electing to print out every byte
                            // and do so with two hex digits per byte rather
                            // than truncating leading zeroes. No need to do
                            // anything fancy here.
                            for byte in bytes.iter().rev() {
                                write!(writer, "{byte:02X}")?;
                            }

                            writeln!(writer)?;
                        }
                    }
                } else {
                    // TODO griffin: Make this an actual error
                    panic!("Output path not specified, this is required for the dat target")
                }
            }
        }
    } else {
        // Since we can't guess based on input/output file names and no target
        // was specified, we just error out.
        return Err(CiderDataConverterError::UnknownTarget);
    }

    Ok(())
}

#[allow(clippy::type_complexity)]
fn get_io_handles(
    opts: &Opts,
) -> Result<(Box<dyn Read>, Box<dyn Write>), CiderDataConverterError> {
    let input = get_read_handle(opts)?;
    let output = get_output_handle(opts)?;
    Ok((input, output))
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
