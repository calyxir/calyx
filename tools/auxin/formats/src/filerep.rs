// format-agnostic file interface

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io::BufRead;
use std::io::Write;
use std::path::PathBuf;

use num_ir::memrep as nr;
use num_ir::memrep::SingleMem;
use num_ir::typing::*;

#[derive(Debug, thiserror::Error)]
pub enum FileFmtErr {
    #[error("format-specific: {0}")]
    FileSpecific(String),

    #[error("numparse: {0}")]
    BadVal(#[from] NumParseErr),
}

// impl<T: ToString> From<T> for FileFmtErr {
//     fn from(value: T) -> Self {
//         FileFmtErr::FileSpecific(value.to_string())
//     }
// }

// TODO: string formats should at least perform cursory input validation on their I/O, bin formats are allowed to but not required to.

// files can read accept path or stdin / stdout, directories must use a path

/// with the current set of supported formats, there aren't any 'global' options which need to be passed to every read / write implementation. as such, format-specific quirks are implemented within the 'handler' struct for each format.

pub trait FileStore {
    type Err: std::error::Error;

    fn read_to_ir<R: BufRead>(&self, src: R) -> Result<MemsMap, Self::Err>;
    fn write_from_ir<W: Write>(
        &self,
        inp: MemsMap,
        dest: W,
    ) -> Result<(), Self::Err>;

    fn read_stdin(&self, handle: std::io::Stdin) -> Result<MemsMap, Self::Err> {
        let locked = handle.lock();
        self.read_to_ir(locked)
    }

    fn write_stdout(
        &self,
        inp: MemsMap,
        handle: std::io::Stdout,
    ) -> Result<(), Self::Err> {
        self.write_from_ir(inp, handle)
    }
}

pub trait DirStore {
    type MemInfo;
    type Err: std::error::Error;
    /// read the entirety of the header file into the aux type T
    /// we can't necessarily know the information obtained from the header, so this returns unit
    fn read_header<R: BufRead>(
        &self,
        src: R,
    ) -> Result<HashMap<String, Self::MemInfo>, Self::Err>;

    /// read the entirety of a given file into the trait implementor
    /// the implementor is responsible for retrieving the relevant header metadata from Self
    fn read_data<R: BufRead>(
        &self,
        src: R,
        inf: Self::MemInfo,
    ) -> Result<SingleMem, Self::Err>;

    /// [write_header_part] is non-consuming
    fn write_header_part<W: Write>(
        &self,
        inp: &SingleMem,
        mem_name: &str,
        dest: W,
    ) -> Result<(), Self::Err>;

    /// [write_data] consumes the data
    fn write_data<W: Write>(
        &self,
        inp: SingleMem,
        mem_name: String,
        dest: W,
    ) -> Result<(), Self::Err>;
}

pub trait TryWriteThrough<T>
where
    Self: Sized,
{
    fn try_write(&self, dest: T, inp: MemsMap) -> Result<(), FileFmtErr>;
}

pub trait TryReadFrom<T> {
    fn try_read(&self, src: T) -> Result<MemsMap, FileFmtErr>;
}

// uses btreemap to preserve ordering
#[derive(Debug, Default)]
pub struct MemsMap {
    pub mems: BTreeMap<String, nr::SingleMem>,
}
