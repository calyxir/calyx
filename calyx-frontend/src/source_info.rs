use itertools::Itertools;
use std::{
    cell::RefCell, collections::HashMap, fmt::Display, io::Read, num::NonZero,
    path::PathBuf,
};
use thiserror::Error;

type Word = u32;

/// An identifier representing a given file path
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(Word);

impl Display for FileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl FileId {
    pub fn new(id: Word) -> Self {
        Self(id)
    }
}

impl From<Word> for FileId {
    fn from(value: Word) -> Self {
        Self(value)
    }
}

/// An identifier representing a location in the Calyx source code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PositionId(Word);

impl PositionId {
    pub fn new(id: Word) -> Self {
        Self(id)
    }
}

impl From<Word> for PositionId {
    fn from(value: Word) -> Self {
        Self(value)
    }
}

impl Display for PositionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A newtype wrapping a line number
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineNum(NonZero<Word>);

impl LineNum {
    pub fn new(line: Word) -> Self {
        Self(NonZero::new(line).expect("Line number must be non-zero"))
    }
    pub fn as_usize(&self) -> usize {
        self.0.get() as usize
    }
}

impl Display for LineNum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Error)]
#[error("Line number cannot be zero")]
pub struct LineNumCreationError;

impl std::fmt::Debug for LineNumCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl TryFrom<Word> for LineNum {
    type Error = LineNumCreationError;

    fn try_from(value: Word) -> Result<Self, Self::Error> {
        if value != 0 {
            Ok(Self(NonZero::new(value).unwrap()))
        } else {
            Err(LineNumCreationError)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInfoTable {
    /// map file ids to the file path, note that this does not contain file content
    file_map: HashMap<FileId, PathBuf>,
    /// maps position ids to their source locations. Positions must be handed
    /// out in order
    position_map: HashMap<PositionId, SourceLocation>,
}

impl SourceInfoTable {
    const HEADER: &str = "sourceinfo";

    pub fn lookup_file_path(&self, file: FileId) -> &PathBuf {
        &self.file_map[&file]
    }

    pub fn lookup_position(&self, pos: PositionId) -> &SourceLocation {
        &self.position_map[&pos]
    }

    pub fn file_reader(&self) -> MetadataFileReader<'_> {
        MetadataFileReader::new(self)
    }

    pub fn add_file(&mut self, file: FileId, path: PathBuf) {
        self.file_map.insert(file, path);
    }

    pub fn add_position(
        &mut self,
        pos: PositionId,
        file: FileId,
        line: LineNum,
    ) {
        self.position_map
            .insert(pos, SourceLocation::new(file, line));
    }

    pub fn new_empty() -> Self {
        Self {
            file_map: HashMap::new(),
            position_map: HashMap::new(),
        }
    }

    pub fn new<F, P>(file_map: F, position_map: P) -> Self
    where
        F: IntoIterator<Item = (FileId, PathBuf)>,
        P: IntoIterator<Item = (PositionId, FileId, LineNum)>,
    {
        SourceInfoTable {
            file_map: file_map.into_iter().collect(),
            position_map: position_map
                .into_iter()
                .map(|(pos, file, line)| (pos, SourceLocation::new(file, line)))
                .collect(),
        }
    }

    pub fn serialize<W: std::io::Write>(
        &self,
        mut f: W,
    ) -> Result<(), std::io::Error> {
        writeln!(f, "{} #{{", Self::HEADER)?;

        // write file table
        writeln!(f, "FILES")?;
        for (file, path) in self.file_map.iter().sorted_by_key(|(&k, _)| k) {
            writeln!(f, "{file}: {}", path.display())?;
        }

        // write the position table
        writeln!(f, "POSITIONS")?;
        for (position, SourceLocation { line, file }) in
            self.position_map.iter().sorted_by_key(|(&k, _)| k)
        {
            writeln!(f, "{position}: {file} {line}")?;
        }

        writeln!(f, "}}#")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: FileId,
    pub line: LineNum,
}

impl SourceLocation {
    pub fn new(file: FileId, line: LineNum) -> Self {
        Self { line, file }
    }
}

/// A wrapper around the metadata table that reads file contents into memory.
/// Constructed via [`MetadataTable::file_reader`].
///
///
/// These allocations are dropped when the reader goes out of scope. Since the
/// lifetime of the reader is tied to the metadata table that created it, this
/// isn't intended to be a long term structure.
pub struct MetadataFileReader<'a> {
    metadata: &'a SourceInfoTable,
    /// I'm not thrilled using interior mutability here, but the alternative is
    /// having reads always require a mutable access which is not ideal. A more
    /// comprehensive solution might involve extending lifetimes rather than
    /// cloning strings
    reader_map: RefCell<HashMap<FileId, Box<str>>>,
}

impl<'a> MetadataFileReader<'a> {
    pub fn new(metadata: &'a SourceInfoTable) -> Self {
        Self {
            metadata,
            reader_map: RefCell::new(HashMap::new()),
        }
    }

    fn read_file_into_memory(&self, file: FileId) -> MetadataResult<()> {
        let path = self.metadata.lookup_file_path(file);
        if path.exists() {
            let mut reader = std::fs::File::open(path)?;
            let mut content = String::new();
            reader.read_to_string(&mut content)?;
            self.reader_map
                .borrow_mut()
                .insert(file, content.into_boxed_str());
            Ok(())
        } else {
            Err(MetadataTableError::FileDoesNotExist(path.clone()))
        }
    }

    /// Looks up the given source position. If the file used by this position
    /// has not been read yet this will cause the contents of the file to be
    /// read into memory. Returns None if either the file or line does not exist
    ///
    /// TODO griffin: make this able to return [str] instead of [String]. Maybe
    /// also don't buffer file contents into memory? This allocation probably
    /// isn't a big deal though
    pub fn lookup_source(
        &self,
        pos: &SourceLocation,
    ) -> MetadataResult<String> {
        // bind this as a separate variable to avoid borrow collisions since
        // reading the file into memory requires
        let contains_key = self.reader_map.borrow().contains_key(&pos.file);
        if !contains_key {
            self.read_file_into_memory(pos.file)?;
        }

        let content = &self.reader_map.borrow()[&pos.file];

        let line = content
            .lines()
            // this is very stupid and there's probably a better way but it
            // works I guess.
            // Need to subtract 1 from the line number since iterators are 0-indexed
            .nth(pos.line.as_usize() - 1)
            .expect("file does not have the given line number");

        Ok(line.to_string())
    }

    /// Given a position id, returns the line of source code that it references
    /// if it exists
    pub fn lookup_position(&self, pos: PositionId) -> MetadataResult<String> {
        if let Some(entry) = self.metadata.position_map.get(&pos) {
            self.lookup_source(entry)
        } else {
            Err(MetadataTableError::PositionDoesNotExist(pos))
        }
    }

    /// Panicking version of [`MetadataFileReader::lookup_source`]
    pub fn unwrap_source(&self, pos: &SourceLocation) -> String {
        self.lookup_source(pos).unwrap()
    }
}

#[derive(Error)]
pub enum MetadataTableError {
    /// General IO error other than file does not exist
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File {0} does not exist")]
    FileDoesNotExist(PathBuf),

    #[error("Position {0} does not exist in the metadata table")]
    PositionDoesNotExist(PositionId),
}

impl std::fmt::Debug for MetadataTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

pub type MetadataResult<T> = Result<T, MetadataTableError>;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{parser::CalyxParser, source_info::LineNum};

    use super::SourceInfoTable;

    #[test]
    fn test_parse_metadata() {
        let input_str = r#"sourceinfo #{
    FILES
        0: test.calyx
        1: test2.calyx
        2: test3.calyx
    POSITIONS
        0: 0 5
        1: 0 1
        2: 0 2
}#"#;

        let metadata = CalyxParser::parse_metadata(input_str).unwrap();
        let file = metadata.lookup_file_path(1.into());
        assert_eq!(file, &PathBuf::from("test2.calyx"));

        let pos = metadata.lookup_position(1.into());
        assert_eq!(pos.file, 0.into());
        assert_eq!(pos.line, LineNum::new(1));
    }

    #[test]
    fn test_serialize() {
        let mut metadata = SourceInfoTable::new_empty();
        metadata.add_file(0.into(), "test.calyx".into());
        metadata.add_file(1.into(), "test2.calyx".into());
        metadata.add_file(2.into(), "test3.calyx".into());

        metadata.add_position(0.into(), 0.into(), LineNum::new(1));
        metadata.add_position(1.into(), 1.into(), LineNum::new(2));
        metadata.add_position(150.into(), 2.into(), LineNum::new(148));

        let mut serialized_str = vec![];
        metadata.serialize(&mut serialized_str).unwrap();
        let serialized_str = String::from_utf8(serialized_str).unwrap();

        let parsed_metadata =
            CalyxParser::parse_metadata(&serialized_str).unwrap();

        assert_eq!(metadata, parsed_metadata)
    }
}
