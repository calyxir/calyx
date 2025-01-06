use std::{
    cell::RefCell,
    collections::HashMap,
    io::{BufReader, Read},
    path::PathBuf,
};

use cider_idx::maps::IndexedMap;

use crate::ids::{FileId, LineNum, PositionId};

#[derive(Debug, Clone)]
pub struct MetadataTable {
    /// map file ids to the file path, note that this does not contain file content
    file_map: IndexedMap<FileId, PathBuf>,
    position_map: IndexedMap<PositionId, SourceLocation>,
}

impl MetadataTable {
    pub fn lookup_file_path(&self, file: FileId) -> &PathBuf {
        &self.file_map[file]
    }

    pub fn lookup_position(&self, pos: PositionId) -> &SourceLocation {
        &self.position_map[pos]
    }

    pub fn file_reader(&self) -> MetadataFileReader<'_> {
        MetadataFileReader::new(self)
    }

    pub fn add_file(&mut self, path: PathBuf) -> FileId {
        self.file_map.push(path)
    }

    pub fn add_position(&mut self, file: FileId, line: LineNum) -> PositionId {
        self.position_map.push(SourceLocation::new(line, file))
    }
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    line: LineNum,
    file: FileId,
}

impl SourceLocation {
    pub fn new(line: LineNum, file: FileId) -> Self {
        Self { line, file }
    }

    pub fn line(&self) -> &LineNum {
        &self.line
    }

    pub fn file(&self) -> FileId {
        self.file
    }
}

pub struct MetadataFileReader<'a> {
    metadata: &'a MetadataTable,
    reader_map: RefCell<HashMap<FileId, Box<str>>>,
}

impl<'a> MetadataFileReader<'a> {
    pub fn new(metadata: &'a MetadataTable) -> Self {
        Self {
            metadata,
            reader_map: RefCell::new(HashMap::new()),
        }
    }

    /// Looks up the given source position. If the file used by this position
    /// has not been read yet this will cause the contents of the file to be
    /// read into memory. Will panic if the file does not exist or does not have
    /// the line number indicated by the position
    ///
    /// TODO griffin: make this able to return [str] instead of [String]. Maybe
    /// also don't buffer file contents into memory?
    pub fn lookup_source(&self, pos: &SourceLocation) -> String {
        let mut mut_read = self.reader_map.borrow_mut();
        let content = mut_read.entry(pos.file).or_insert_with(|| {
            let file_path = self.metadata.lookup_file_path(pos.file);
            let mut buffer = String::new();
            BufReader::new(
                std::fs::File::open(file_path).expect("unable to open file"),
            )
            .read_to_string(&mut buffer)
            .expect("couldn't read into str");
            buffer.into_boxed_str()
        });

        let line = content
            .lines()
            .nth(pos.line.as_usize())
            .expect("file does not have the given line number");

        line.to_string()
    }
}
