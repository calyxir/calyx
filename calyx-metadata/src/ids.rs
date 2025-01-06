use cider_idx::impl_index;

/// An identifier representing a given file path
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(u32);
impl_index!(FileId);

/// An identifier representing a location in the Calyx source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionId(u32);
impl_index!(PositionId);

#[derive(Debug, Clone)]
pub struct LineNum(u32);

impl LineNum {
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}
