//! format-agnostic file interface
//!
//! files can read accept path or stdin / stdout, directories must use a path
//!
//! with the current set of supported formats, there aren't any 'global' options which need to be passed to every read / write implementation. as such, format-specific quirks are implemented within the 'handler' struct for each format.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io::BufRead;
use std::io::Seek;
use std::io::Write;

use num_ir::memrep as nr;
use num_ir::memrep::SingleMem;

/// I/O operations for single-file formats
pub trait FileStore {
    type Err: std::error::Error;

    /// read from a [BufRead] which also implements [Seek].
    ///
    /// the [std::io::BufReader] type does this
    fn read_filelike<R: BufRead + Seek>(
        &self,
        src: R,
    ) -> Result<MemsMap, Self::Err> {
        self.read_stream(src)
    }

    /// read from a [BufRead] which does not necessarily implement [Seek].
    ///
    /// there will be a default implementation for [Self::read_filelike] as long as this method is implemented.
    fn read_stream<R: BufRead>(&self, handle: R) -> Result<MemsMap, Self::Err>;

    /// write ``inp`` according to the trait implementor's formatting. ``inp`` is consumed.
    fn write<W: Write>(&self, inp: MemsMap, handle: W)
    -> Result<(), Self::Err>;
}

/// I/O operations for directory-based formats
pub trait DirStore {
    /// an associated type which a directory format can use to store metadata.
    type MemInfo;
    type Err: std::error::Error;
    /// Read the entirety of the header file into a ``HashMap`` mapping memory name to [Self::MemInfo]
    fn read_header<R: BufRead>(
        &self,
        src: R,
    ) -> Result<HashMap<String, Self::MemInfo>, Self::Err>;

    /// Read the entirety of a given file into the trait implementor
    /// the implementor is responsible for retrieving the relevant header metadata from Self
    fn read_data<R: BufRead>(
        &self,
        src: R,
        inf: Self::MemInfo,
    ) -> Result<SingleMem, Self::Err>;

    /// Write information from a *reference* to a memory and its name. Does not consume either input.
    fn write_header_part<W: Write>(
        &self,
        inp: &SingleMem,
        mem_name: &str,
        dest: W,
    ) -> Result<(), Self::Err>;

    /// Write information from a memory and its name, consuming both
    fn write_data<W: Write>(
        &self,
        inp: SingleMem,
        mem_name: String,
        dest: W,
    ) -> Result<(), Self::Err>;
}

// uses btreemap to preserve ordering
#[derive(Debug, Default)]
pub struct MemsMap {
    pub mems: BTreeMap<String, nr::SingleMem>,
}
