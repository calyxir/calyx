// format-agnostic file interface

use std::collections::BTreeMap;
use std::fmt::Write;
use std::io::Read;
use std::path::Path;

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

/// a structure for passing options to the string file interfaces.
pub struct OutputOpts {
    // a struct so other output options can be added in the future
    pub print_hex: bool,
}

/*
TODO:
- FileIO most likely doesn't need to be dyn
- DirIO could probably be more structured, s.t. implementer has to do less work.

*/

// these build an object containing opts
// TODO: implement boilerplate types for storing opts per format (?)
// just. think about modularity

pub trait DirFmtOpts
where
    Self: Sized,
{
    fn from_path(src: &Path, ext: String) -> Result<Self, FileFmtErr>;
}

pub trait FileFmtOpts
where
    Self: Sized,
{
    fn with_src(src: Box<dyn Read>);
    fn with_dest(dest: Box<dyn Write>);
}

pub trait TryFromIR
where
    Self: Sized,
{
    fn try_from_ir(self, inp: FileMems) -> Result<(), FileFmtErr>;
}

pub trait TryToIR {
    fn try_to_ir(self) -> Result<FileMems, FileFmtErr>;
}

// uses btreemap to preserve ordering
#[derive(Debug, Default)]
pub struct FileMems {
    pub mems: BTreeMap<String, nr::SingleMem>,
}
