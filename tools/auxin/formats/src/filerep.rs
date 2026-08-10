// format-agnostic file interface

use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use num_ir::memrep as nr;
use num_ir::typing::*;

#[derive(Debug, thiserror::Error)]
pub enum FileFmtErr {
    #[error("format-specific: {0}")]
    FileSpecific(String),

    #[error("numparse: {0}")]
    BadVal(#[from] NumParseErr),
}

// TODO: string formats should at least perform cursory input validation on their I/O, bin formats are allowed to but not required to.

// files can read accept path or stdin / stdout, directories must use a path

/// with the current set of supported formats, there aren't any 'global' options which need to be passed to every read / write implementation. as such, format-specific quirks are implemented within the 'handler' struct for each format.

pub enum FileSink {
    P(PathBuf),
    Stream,
}

impl FileSink {
    pub fn get_writer(&self) -> Result<Box<dyn Write>, FileFmtErr> {
        let e = match &self {
            FileSink::P(path) => {
                let f = File::create(path).map(|x| BufWriter::new(x))?;
                Box::new(f) as Box<dyn Write>
            }
            FileSink::Stream => Box::new(std::io::stdout()),
        };
        Ok(e)
    }

    pub fn get_reader(&self) -> Result<Box<dyn Read>, FileFmtErr> {
        let e = match &self {
            FileSink::P(path) => {
                let f = File::open(path)?;
                Box::new(f) as Box<dyn Read>
            }
            FileSink::Stream => Box::new(std::io::stdin()),
        };
        Ok(e)
    }
}

pub struct DirSink {
    pub path: PathBuf,
    pub ext: String,
}

impl DirSink {
    pub fn is_dir(&self) -> Result<(), FileFmtErr> {
        if self.path.exists() && !self.path.is_dir() {
            return Err(FileFmtErr::FileSpecific(format!(
                "{:?}: not a directory",
                self.path
            )));
        }
        Ok(())
    }
}

// T will usually be FileSink or DirSink. it could possibly be expanded in the future.

pub trait TryWriteThrough<T>
where
    Self: Sized,
{
    fn try_write(&self, dest: T, inp: FileMems) -> Result<(), FileFmtErr>;
}

pub trait TryReadFrom<T> {
    fn try_read(&self, src: T) -> Result<FileMems, FileFmtErr>;
}

// uses btreemap to preserve ordering
#[derive(Debug, Default)]
pub struct FileMems {
    pub mems: BTreeMap<String, nr::SingleMem>,
}
