// format-agnostic file interface

use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use super::numrep as nr;
use crate::typing::*;

// TODO: string formats should at least perform cursory input validation on their I/O, bin formats are allowed to but not required to.

pub trait TryToIR {
    fn try_to_ir(
        self,
        types: &HashMap<String, TypeSpec>,
    ) -> Result<FileMems, FileFmtErr>;
}

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

pub trait FileIO
where
    Self: Sized,
{
    fn read_into(src: Box<dyn Read>) -> Result<Self, FileFmtErr>;
    fn write_out(&self, dest: Box<dyn Write>) -> Result<(), FileFmtErr>;
}

// equivalent of FileIO but for formats which output to a directory
pub trait DirIO
where
    Self: Sized,
{
    fn read_into_dir(src: PathBuf, ext: String) -> Result<Self, FileFmtErr>;
    fn write_out_dir(
        &self,
        dest: PathBuf,
        ext: String,
    ) -> Result<(), FileFmtErr>;
}

pub trait TryFromIR
where
    Self: Sized,
{
    fn try_from_ir(inp: &FileMems) -> Result<Self, FileFmtErr>;
}

#[derive(Debug)]
pub enum FileFmtErr {
    FileSpecific(String),
}

impl From<String> for FileFmtErr {
    fn from(value: String) -> Self {
        FileFmtErr::FileSpecific(value)
    }
}

impl From<&str> for FileFmtErr {
    fn from(value: &str) -> Self {
        FileFmtErr::FileSpecific(String::from(value))
    }
}

impl From<ReadStringErr> for FileFmtErr {
    fn from(value: ReadStringErr) -> Self {
        let ReadStringErr::BadValue(s) = value;
        FileFmtErr::FileSpecific(s)
    }
}

#[derive(Debug)]
pub struct FileMems {
    pub mems: HashMap<String, nr::SingleMem>,
}

/// file formats which include typing can implement the [ExtractType] trait
/// and gain access to a generalised [HintedTryToIR], which pre-loads types from the file.
pub trait ExtractType {
    fn extract_types(&self) -> Result<HashMap<String, TypeSpec>, FileFmtErr>;
}

pub trait HintedTryToIR
where
    Self: ExtractType + TryToIR + Sized,
{
    fn hinted_try_to_ir(self) -> Result<FileMems, FileFmtErr> {
        let types = self.extract_types()?;
        self.try_to_ir(&types)
    }
}
