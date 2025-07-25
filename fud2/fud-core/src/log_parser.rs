use std::{error::Error, fmt::Display, fs::File, io::Read, num::ParseIntError};

use camino::Utf8Path;
use itertools::Itertools;

#[derive(Debug)]
pub enum LogParseError {
    /// Doesn't match the expected ninja log format
    UnexpectedFormat,
    /// An IO error for reading the log and writing the CSV
    Io(std::io::Error),
}

impl From<std::io::Error> for LogParseError {
    fn from(v: std::io::Error) -> Self {
        Self::Io(v)
    }
}

impl Display for LogParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogParseError::UnexpectedFormat => {
                write!(f, "ninja log file does not have the expected format")
            }

            LogParseError::Io(error) => write!(f, "{error}"),
        }
    }
}

impl From<ParseIntError> for LogParseError {
    fn from(_: ParseIntError) -> Self {
        LogParseError::UnexpectedFormat
    }
}

impl Error for LogParseError {}

#[derive(Debug, Clone)]
pub struct NinjaLogEntry {
    start_time: u32,
    end_time: u32,
    file_produced: Box<str>,
}

impl NinjaLogEntry {
    fn from_str(input: &str) -> Result<Self, LogParseError> {
        let mut iter = input.split_whitespace();
        let Some((start_time, end_time, _tstamp, filename, _hash)) =
            iter.next_tuple()
        else {
            return Err(LogParseError::UnexpectedFormat);
        };

        let start_time: u32 = start_time.parse()?;
        let end_time: u32 = end_time.parse()?;
        let file_produced: Box<str> = filename.into();

        Ok(NinjaLogEntry {
            start_time,
            end_time,
            file_produced,
        })
    }

    fn emit_csv<W: std::io::Write>(&self, mut writer: W) {
        writeln!(
            writer,
            "{}, {}",
            self.file_produced,
            self.end_time - self.start_time,
        )
        .expect("writing failed");
    }
}

#[derive(Debug, Clone)]
pub struct NinjaLog {
    entries: Vec<NinjaLogEntry>,
}

impl NinjaLog {
    fn from_str(input: &str) -> Result<Self, LogParseError> {
        let mut out = vec![];

        for line in input.lines() {
            if !line.starts_with("#") {
                out.push(NinjaLogEntry::from_str(line)?)
            }
        }

        Ok(NinjaLog { entries: out })
    }

    fn emit_csv<W: std::io::Write>(&self, mut writer: W) {
        writeln!(&mut writer, "filename, duration").expect("writing failed");
        for entry in &self.entries {
            entry.emit_csv(&mut writer);
        }
        writer.flush().expect("writing failed");
    }
}

pub fn generate_timing_csv(
    temp_dir: &Utf8Path,
    csv_file: &Utf8Path,
) -> Result<(), LogParseError> {
    let log_file = temp_dir.join(".ninja_log");
    let mut file = File::open(log_file)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    let log = NinjaLog::from_str(&buffer)?;
    let output = File::create(csv_file)?;
    log.emit_csv(output);
    Ok(())
}
