// format-independent representation

use std::collections::HashMap;

use super::numrep as nr;

/// if a type is provided with store S, then it must be either kept in the Result or fail

/// string formats should at least perform cursory input validation on their I/O, bin formats are allowed to but not required to.

pub trait TryToIR<S = Self> {
    fn try_to_ir<T: nr::ReprType>(inp: S)
    -> Result<nr::DataSet<T>, FileFmtErr>;
}

pub trait TryFromIR<T: nr::ReprType, S> {
    fn try_from_ir(inp: &nr::DataSet<T>) -> Result<S, FileFmtErr>;
}

pub type FileFmtErr = String;

pub struct FileMems {
    pub store: HashMap<String, Box<dyn nr::DataTrait>>,
}
