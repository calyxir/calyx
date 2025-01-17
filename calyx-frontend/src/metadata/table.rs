use std::{
    cell::RefCell, collections::HashMap, fmt::Display, io::Read, path::PathBuf,
};
use thiserror::Error;

type Word = u32;

/// An identifier representing a given file path
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(Word);

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
        write!(f, "{}", self.0)
    }
}

/// A newtype wrapping a line number
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineNum(Word);

impl LineNum {
    pub fn new(line: Word) -> Self {
        Self(line)
    }
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

impl From<Word> for LineNum {
    fn from(value: Word) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone)]
pub struct MetadataTable {
    /// map file ids to the file path, note that this does not contain file content
    file_map: HashMap<FileId, PathBuf>,
    /// maps position ids to their source locations. Positions must be handed
    /// out in order
    position_map: HashMap<PositionId, SourceLocation>,
}

impl MetadataTable {
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
            .insert(pos, SourceLocation::new(line, file));
    }

    pub fn new<F, P>(file_map: F, position_map: P) -> Self
    where
        F: IntoIterator<Item = (FileId, PathBuf)>,
        P: IntoIterator<Item = (PositionId, FileId, LineNum)>,
    {
        let mut table = MetadataTable {
            file_map: HashMap::new(),
            position_map: HashMap::new(),
        };

        for (file, path) in file_map {
            table.add_file(file, path);
        }

        for (pos, file, line) in position_map {
            table.add_position(pos, file, line);
        }

        table
    }
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub line: LineNum,
    pub file: FileId,
}

impl SourceLocation {
    pub fn new(line: LineNum, file: FileId) -> Self {
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
    metadata: &'a MetadataTable,
    /// I'm not thrilled using interior mutability here, but the alternative is
    /// having reads always require a mutable access which is not ideal. A more
    /// comprehensive solution might involve extending lifetimes rather than
    /// cloning strings
    reader_map: RefCell<HashMap<FileId, Box<str>>>,
}

impl<'a> MetadataFileReader<'a> {
    pub fn new(metadata: &'a MetadataTable) -> Self {
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
            // works I guess
            .nth(pos.line.as_usize())
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
