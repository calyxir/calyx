// format-agnostic file interface

use std::collections::HashMap;

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

pub trait TryFromIR
where
    Self: Sized,
{
    fn try_from_ir(
        inp: &FileMems,
        typeprops: &TypePropsMap,
    ) -> Result<Self, FileFmtErr>;
}

pub type FileFmtErr = String;

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
