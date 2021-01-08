use std::{io::Write, path::PathBuf, str::FromStr};

/// Possible choices for output streams.
/// Used by the `-o` option to the compiler.
#[derive(Debug)]
pub enum OutputFile {
    Stdout,
    File(PathBuf),
}

impl FromStr for OutputFile {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "-" => Ok(OutputFile::Stdout),
            _ => Ok(OutputFile::File(PathBuf::from(s))),
        }
    }
}

impl ToString for OutputFile {
    fn to_string(&self) -> String {
        match self {
            OutputFile::Stdout => "-".to_string(),
            OutputFile::File(p) => p.to_str().unwrap().to_string(),
        }
    }
}

impl Default for OutputFile {
    fn default() -> Self {
        OutputFile::Stdout
    }
}

impl OutputFile {
    pub fn isatty(&self) -> bool {
        match self {
            OutputFile::Stdout => atty::is(atty::Stream::Stdout),
            OutputFile::File(_) => false,
        }
    }

    pub fn get_write(&self) -> Box<dyn Write> {
        match self {
            OutputFile::Stdout => Box::new(std::io::stdout()),
            OutputFile::File(path) => {
                Box::new(std::fs::File::create(path).unwrap())
            }
        }
    }
}
