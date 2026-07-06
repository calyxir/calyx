// format-agnostic file interface

use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use super::numimpl::*;
use super::numrep as nr;

/// if a type is provided with store S, then it must be either kept in the Result or fail

/// string formats should at least perform cursory input validation on their I/O, bin formats are allowed to but not required to.

pub trait TryToIR {
    fn try_to_ir(
        self,
        types: &HashMap<String, nr::TypeSpec>,
        typeprops: &TypePropsMap,
    ) -> Result<FileMems, FileFmtErr>;
}

// TODO: this may not need to be dyn?? maybe?
// TODO: also these interfaces are pretty Bad, in particular the DirIO one: implementer may have to do a lot of work.
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
    fn read_into(src: PathBuf) -> Result<Self, FileFmtErr>;
    fn write_out(&self, dest: PathBuf) -> Result<(), FileFmtErr>;
}

pub trait TryFromIR
where
    Self: Sized,
{
    fn try_from_ir(
        inp: &FileMems,
        typeprops: &TypePropsMap,
    ) -> Result<Self, FileFmtErr>;
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

pub struct FileMems {
    pub mems: HashMap<String, nr::SingleMem>,
}

/// file formats which include typing can implement the [ExtractType] trait
/// and gain access to a generalised [HintedTryToIR], which pre-loads types from the file.

pub trait ExtractType {
    fn extract_types(
        &self,
    ) -> Result<HashMap<String, nr::TypeSpec>, FileFmtErr>;
}

pub trait HintedTryToIR
where
    Self: ExtractType + TryToIR + Sized,
{
    fn hinted_try_to_ir(
        self,
        typeprops: &TypePropsMap,
    ) -> Result<FileMems, FileFmtErr> {
        let types = self.extract_types()?;
        self.try_to_ir(&types, typeprops)
    }
}
