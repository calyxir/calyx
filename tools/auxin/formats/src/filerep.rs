// format-agnostic file interface

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io::BufRead;
use std::io::Seek;
use std::io::Write;

use num_ir::memrep as nr;
use num_ir::memrep::SingleMem;

// files can read accept path or stdin / stdout, directories must use a path

/// with the current set of supported formats, there aren't any 'global' options which need to be passed to every read / write implementation. as such, format-specific quirks are implemented within the 'handler' struct for each format.

pub trait FileStore {
    type Err: std::error::Error;

    fn read_filelike<R: BufRead + Seek>(
        &self,
        src: R,
    ) -> Result<MemsMap, Self::Err> {
        self.read_stream(src)
    }

    fn read_stream<R: BufRead>(&self, handle: R) -> Result<MemsMap, Self::Err>;

    fn write<W: Write>(&self, inp: MemsMap, handle: W)
    -> Result<(), Self::Err>;
}

pub trait DirStore {
    type MemInfo;
    type Err: std::error::Error;
    /// read the entirety of the header file into a ``HashMap`` with [MemInfo]
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

// uses btreemap to preserve ordering
#[derive(Debug, Default)]
pub struct MemsMap {
    pub mems: BTreeMap<String, nr::SingleMem>,
}
