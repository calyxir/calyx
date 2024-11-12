use std::{
    io::{self, BufWriter},
    path::PathBuf,
    str::FromStr,
};

/// Possible choices for output streams. Used by the `-o` option to the compiler.
/// * "-" and "<out>" are treated as stdout.
/// * "<err>" is treated as stderr.
/// * "<null>" is treated as a null output stream.
/// * All other strings are treated as file paths.
#[derive(Debug, Clone)]
pub enum OutputFile {
    Null,
    Stdout,
    Stderr,
    File {
        path: PathBuf,
        // Has the writer been initialized?
        init: bool,
    },
}

impl OutputFile {
    pub fn file(path: PathBuf) -> Self {
        OutputFile::File { path, init: false }
    }

    pub fn as_path_string(&self) -> String {
        match self {
            OutputFile::Null => "<null>".to_string(),
            OutputFile::Stdout => "<stdout>".to_string(),
            OutputFile::Stderr => "<stderr>".to_string(),
            OutputFile::File { path, .. } => path.to_string_lossy().to_string(),
        }
    }
}

impl FromStr for OutputFile {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "-" | "<out>" => Ok(OutputFile::Stdout),
            "<err>" => Ok(OutputFile::Stderr),
            "<null>" => Ok(OutputFile::Null),
            _ => Ok(OutputFile::file(PathBuf::from(s))),
        }
    }
}

impl ToString for OutputFile {
    fn to_string(&self) -> String {
        match self {
            OutputFile::Stdout => "-".to_string(),
            OutputFile::Stderr => "<err>".to_string(),
            OutputFile::Null => "<null>".to_string(),
            OutputFile::File { path, .. } => path.to_str().unwrap().to_string(),
        }
    }
}

impl OutputFile {
    pub fn isatty(&self) -> bool {
        match self {
            OutputFile::Stdout => atty::is(atty::Stream::Stdout),
            OutputFile::Stderr => atty::is(atty::Stream::Stderr),
            OutputFile::Null | OutputFile::File { .. } => false,
        }
    }

    pub fn get_write(&mut self) -> Box<dyn io::Write> {
        match self {
            OutputFile::Stdout => Box::new(BufWriter::new(std::io::stdout())),
            OutputFile::Stderr => Box::new(BufWriter::new(std::io::stderr())),
            OutputFile::File { path, init } => {
                // If the file does not exist, create it. Otherwise, open it for appending.
                let buf = if *init {
                    assert!(
                        path.exists(),
                        "writer initialized but file does not exist"
                    );
                    BufWriter::new(
                        std::fs::OpenOptions::new()
                            .append(true)
                            .open(path)
                            .unwrap(),
                    )
                } else {
                    *init = true;
                    BufWriter::new(std::fs::File::create(path).unwrap())
                };
                Box::new(buf)
            }
            OutputFile::Null => Box::new(io::sink()),
        }
    }
}
