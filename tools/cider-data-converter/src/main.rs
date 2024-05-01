use argh::FromArgs;
use cider_data_converter::{converter, json_data::JsonData};
use interp::serialization::data_dump::{self, SerializationError};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
    str::FromStr,
};
use thiserror::Error;

const JSON_EXTENSION: &str = "data";
const CIDER_EXTENSION: &str = "dump";

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

enum Action {
    ToDataDump,
    ToJson,
}

impl FromStr for Action {
    type Err = CiderDataConverterError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Action::ToJson),
            "cider" | "dump" | "data-dump" => Ok(Action::ToDataDump),
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

    /// optional specification of what action to perform. Can be "cider" or
    /// "json". If not provided, the converter will try to guess based on file names
    #[argh(option, short = 't', long = "to")]
    action: Option<Action>,
}

fn main() -> Result<(), CiderDataConverterError> {
    let mut opts: Opts = argh::from_env();

    let mut input: Box<dyn Read> = opts
        .input_path
        .as_ref()
        .map(|path| File::open(path).map(|x| Box::new(x) as Box<dyn Read>))
        .unwrap_or(Ok(Box::new(io::stdin())))?;

    let mut output: Box<dyn Write> = opts
        .output_path
        .as_ref()
        .map(|path| File::create(path).map(|x| Box::new(x) as Box<dyn Write>))
        .unwrap_or(Ok(Box::new(io::stdout())))?;

    // if no action is specified, try to guess based on file extensions
    if opts.action.is_none()
        && (opts.input_path.as_ref().is_some_and(|x| {
            x.extension().map_or(false, |y| y == JSON_EXTENSION)
        }) || opts.output_path.as_ref().is_some_and(|x| {
            x.extension().map_or(false, |y| y == CIDER_EXTENSION)
        }))
    {
        opts.action = Some(Action::ToDataDump);
    } else if opts.action.is_none()
        && (opts.output_path.as_ref().is_some_and(|x| {
            x.extension().map_or(false, |x| x == JSON_EXTENSION)
        }) || opts.input_path.as_ref().is_some_and(|x| {
            x.extension().map_or(false, |x| x == CIDER_EXTENSION)
        }))
    {
        opts.action = Some(Action::ToJson);
    }

    if let Some(action) = opts.action {
        match action {
            Action::ToDataDump => {
                let parsed_json: JsonData =
                    serde_json::from_reader(&mut input)?;
                converter::convert_to_data_dump(&parsed_json)
                    .serialize(&mut output)?;
            }
            Action::ToJson => {
                let data_dump = data_dump::DataDump::deserialize(&mut input)?;
                let json_data = converter::convert_from_data_dump(&data_dump);
                writeln!(
                    &mut output,
                    "{}",
                    serde_json::to_string_pretty(&json_data)?
                )?;
            }
        }
    } else {
        // Since we can't guess based on input/output file names and no target
        // was specified, we just error out.
        return Err(CiderDataConverterError::UnknownTarget);
    }

    Ok(())
}
