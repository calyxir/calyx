use itertools::Itertools;
use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    io::Read,
    num::{NonZero, TryFromIntError},
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

    pub fn value(&self) -> Word {
        self.0
    }
}

impl From<Word> for PositionId {
    fn from(value: Word) -> Self {
        Self(value)
    }
}

impl TryFrom<u64> for PositionId {
    type Error = TryFromIntError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let v: u32 = value.try_into()?;
        Ok(Self(v))
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
    pub fn into_inner(self) -> NonZero<Word> {
        self.0
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

/// An ID in the source map labelling some memory location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MemoryLocationId(Word);

impl TryFrom<u64> for MemoryLocationId {
    type Error = TryFromIntError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let v: u32 = value.try_into()?;
        Ok(Self(v))
    }
}

impl From<Word> for MemoryLocationId {
    fn from(value: Word) -> Self {
        Self(value)
    }
}

impl Display for MemoryLocationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> core::fmt::Result {
        <Word as Display>::fmt(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryLocation {
    pub cell: String,
    pub address: Vec<usize>,
}

/// An ID in the source map labelling a set of mappings from variable names to memory locations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VariableAssignmentId(Word);

impl Display for VariableAssignmentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> core::fmt::Result {
        <Word as Display>::fmt(&self.0, f)
    }
}
impl TryFrom<u64> for VariableAssignmentId {
    type Error = TryFromIntError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let v: u32 = value.try_into()?;
        Ok(Self(v))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceInfoTable {
    /// map file ids to the file path, note that this does not contain file content
    file_map: HashMap<FileId, PathBuf>,
    /// maps position ids to their source locations.
    position_map: HashMap<PositionId, SourceLocation>,
    /// assigns ids to locations in memories and registers
    mem_location_map: HashMap<MemoryLocationId, MemoryLocation>,
    /// assigns ids to collections of variable -> location mappings
    variable_assignment_map:
        HashMap<VariableAssignmentId, HashMap<String, MemoryLocationId>>,
    /// collects the mapping from positions representing a point in the control
    /// program to the set of variable assignments for that position
    position_state_map: HashMap<PositionId, VariableAssignmentId>,
}

impl SourceInfoTable {
    const HEADER: &str = "sourceinfo";

    /// Looks up the path of the file with the given id.
    ///
    /// # Panics
    /// Panics if the file id does not exist in the file map
    pub fn lookup_file_path(&self, file: FileId) -> &PathBuf {
        &self.file_map[&file]
    }

    /// Looks up the source location of the position with the given id.
    ///
    /// # Panics
    /// Panics if the position id does not exist in the position map
    pub fn lookup_position(&self, pos: PositionId) -> &SourceLocation {
        &self.position_map[&pos]
    }

    /// Looks up the source location of the position with the given id. If no
    /// such position exists, returns `None`
    pub fn get_position(&self, pos: PositionId) -> Option<&SourceLocation> {
        self.position_map.get(&pos)
    }

    /// Iterate over the stored file map, returning a tuple of references to the
    /// file id and the path
    pub fn iter_file_map(&self) -> impl Iterator<Item = (&FileId, &PathBuf)> {
        self.file_map.iter()
    }

    /// Iterate over the paths of all files in the file map
    pub fn iter_file_paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.file_map.values()
    }

    /// Iterate over all file ids in the file map
    pub fn iter_file_ids(&self) -> impl Iterator<Item = FileId> + '_ {
        self.file_map.keys().copied()
    }

    /// Iterate over the stored position map, returning a tuple of references to
    /// the position id and the source location
    pub fn iter_position_map(
        &self,
    ) -> impl Iterator<Item = (&PositionId, &SourceLocation)> {
        self.position_map.iter()
    }

    /// Iterate over all position ids in the position map
    pub fn iter_positions(&self) -> impl Iterator<Item = PositionId> + '_ {
        self.position_map.keys().copied()
    }

    /// Iterate over the source locations in the position map
    pub fn iter_source_locations(
        &self,
    ) -> impl Iterator<Item = &SourceLocation> {
        self.position_map.values()
    }

    /// Adds a file to the file map with the given id
    pub fn add_file(&mut self, file: FileId, path: PathBuf) {
        self.file_map.insert(file, path);
    }

    /// Adds a file to the file map and generates a new file id
    /// for it. If you want to add a file with a specific id, use
    /// [`SourceInfoTable::add_file`]
    pub fn push_file(&mut self, path: PathBuf) -> FileId {
        // find the largest file id in the map
        let max = self.iter_file_ids().max().unwrap_or(0.into());
        let new = FileId(max.0 + 1);

        self.add_file(new, path);
        new
    }
    pub fn add_position(
        &mut self,
        pos: PositionId,
        file: FileId,
        line: LineNum,
        endline: Option<LineNum>,
    ) {
        self.position_map
            .insert(pos, SourceLocation::new(file, line, endline));
    }

    /// Adds a position to the position map and generates a new position id
    /// for it. If you want to add a position with a specific id, use
    /// [`SourceInfoTable::add_position`]
    pub fn push_position(
        &mut self,
        file: FileId,
        line: LineNum,
        endline: Option<LineNum>,
    ) -> PositionId {
        // find the largest position id in the map
        let max = self.iter_positions().max().unwrap_or(0.into());
        let new = PositionId(max.0 + 1);

        self.add_position(new, file, line, endline);
        new
    }

    pub fn add_location(&mut self, id: MemoryLocationId, info: MemoryLocation) {
        self.mem_location_map.insert(id, info);
    }

    /// Attempts to look up the variable mapping associated with a given
    /// position, if such a mapping exists
    pub fn get_variable_mapping(
        &self,
        pos: PositionId,
    ) -> Option<&HashMap<String, MemoryLocationId>> {
        self.position_state_map
            .get(&pos)
            .and_then(|x| self.variable_assignment_map.get(x))
    }

    pub fn get_memory_location(
        &self,
        loc: &MemoryLocationId,
    ) -> &MemoryLocation {
        &self.mem_location_map[loc]
    }

    /// Creates a new empty source info table
    pub fn new_empty() -> Self {
        Self {
            file_map: HashMap::new(),
            position_map: HashMap::new(),
            mem_location_map: HashMap::new(),
            variable_assignment_map: HashMap::new(),
            position_state_map: HashMap::new(),
        }
    }

    /// A wrapper function to construct a source a source map containing only
    /// files and positions. If an empty map is needed use [SourceInfoTable::new_empty]
    pub fn new_minimal(
        files: impl IntoIterator<Item = (FileId, PathBuf)>,
        positions: impl IntoIterator<
            Item = (PositionId, FileId, LineNum, Option<LineNum>),
        >,
    ) -> SourceInfoResult<Self> {
        // the compiler needs some concrete types here even though the input is
        // all empty
        let loc: Vec<(MemoryLocationId, MemoryLocation)> = vec![];
        let states: Vec<(PositionId, VariableAssignmentId)> = vec![];
        let variable_assigns: Vec<(
            VariableAssignmentId,
            Vec<(String, MemoryLocationId)>,
        )> = vec![];

        Self::new(files, positions, loc, variable_assigns, states)
    }

    // this is awful
    pub fn new(
        files: impl IntoIterator<Item = (FileId, PathBuf)>,
        positions: impl IntoIterator<
            Item = (PositionId, FileId, LineNum, Option<LineNum>),
        >,
        locations: impl IntoIterator<Item = (MemoryLocationId, MemoryLocation)>,
        variable_assigns: impl IntoIterator<
            Item = (
                VariableAssignmentId,
                impl IntoIterator<Item = (String, MemoryLocationId)>,
            ),
        >,
        states: impl IntoIterator<Item = (PositionId, VariableAssignmentId)>,
    ) -> SourceInfoResult<Self> {
        let files = files.into_iter();
        let positions = positions.into_iter();
        let locations = locations.into_iter();
        let vars = variable_assigns.into_iter();
        let states = states.into_iter();

        let mut file_map = HashMap::with_capacity(
            files.size_hint().1.unwrap_or(files.size_hint().0),
        );
        let mut position_map = HashMap::with_capacity(
            positions.size_hint().1.unwrap_or(positions.size_hint().0),
        );

        let mut memory_location_map: HashMap<MemoryLocationId, MemoryLocation> =
            HashMap::new();

        let mut variable_map = HashMap::new();
        let mut state_map = HashMap::new();

        for (file, path) in files {
            if let Some(first_path) = file_map.insert(file, path) {
                let inserted_path = &file_map[&file];
                if &first_path != inserted_path {
                    return Err(SourceInfoTableError::InvalidTable(format!(
                        "File id {file} is defined multiple times:\n   1. {}\n   2. {}\n",
                        first_path.display(),
                        inserted_path.display()
                    )));
                }
            }
        }

        for (pos, file, line, end_line) in positions {
            let source = SourceLocation::new(file, line, end_line);
            if let Some(first_pos) = position_map.insert(pos, source) {
                let inserted_position = &position_map[&pos];
                if inserted_position != &first_pos {
                    return Err(SourceInfoTableError::InvalidTable(format!(
                        "Duplicate positions found in the metadata table. Position {pos} is defined multiple times:\n   1. file {}, line {}\n   2. file {}, line {}\n",
                        first_pos.file,
                        first_pos.line,
                        inserted_position.file,
                        inserted_position.line
                    )));
                }
            }
        }

        for (id, loc) in locations {
            if memory_location_map.insert(id, loc).is_some() {
                return Err(SourceInfoTableError::InvalidTable(format!(
                    "Multiple definitions for memory location {id}"
                )));
            }
        }

        for (assign_label, assigns) in vars {
            let mut mapping = HashMap::new();
            for (name, location) in assigns {
                if !memory_location_map.contains_key(&location) {
                    // unknown memory location
                    return Err(SourceInfoTableError::InvalidTable(format!(
                        "Memory location {location} is referenced but never defined"
                    )));
                }
                // this is to avoid copying the string in all cases since we
                // would only need it when emitting the error. Clippy doesn't
                // like this for good reasons and while I suspect it may be
                // possible using the entry api, I think this is clearer so I'm
                // just suppressing the warning and writing this very long
                // comment about it instead.
                #[allow(clippy::map_entry)]
                if mapping.contains_key(&name) {
                    return Err(SourceInfoTableError::InvalidTable(format!(
                        "In variable mapping {assign_label} the variable '{name}' has multiple definitions"
                    )));
                } else {
                    mapping.insert(name, location);
                }
            }
            if variable_map.insert(assign_label, mapping).is_some() {
                return Err(SourceInfoTableError::InvalidTable(format!(
                    "Duplicate definitions for variable mapping associated with position {assign_label}"
                )));
            };
        }

        for (pos_id, var_id) in states {
            if !variable_map.contains_key(&var_id) {
                return Err(SourceInfoTableError::InvalidTable(format!(
                    "Variable mapping {var_id} is referenced but never defined"
                )));
            }
            if state_map.insert(pos_id, var_id).is_some() {
                return Err(SourceInfoTableError::InvalidTable(format!(
                    "Multiple variable maps have been assigned to position {pos_id}"
                )));
            }
        }

        Ok(SourceInfoTable {
            file_map,
            position_map,
            mem_location_map: memory_location_map,
            variable_assignment_map: variable_map,
            position_state_map: state_map,
        })
    }

    pub fn serialize<W: std::io::Write>(
        &self,
        mut f: W,
    ) -> Result<(), std::io::Error> {
        Self::write_header(&mut f)?;

        // mandatory entries
        self.write_file_table(&mut f)?;
        self.write_pos_table(&mut f)?;

        // optional entries
        if !(self.mem_location_map.is_empty()
            && self.variable_assignment_map.is_empty()
            && self.position_state_map.is_empty())
        {
            self.write_memory_table(&mut f)?;
            self.write_var_assigns(&mut f)?;
            self.write_pos_state_table(&mut f)?;
        }

        Self::write_footer(&mut f)
    }

    fn write_pos_state_table<W: std::io::Write>(
        &self,
        f: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(f, "POSITION_STATE_MAP")?;

        for (pos, var) in
            self.position_state_map.iter().sorted_by_key(|(k, _)| **k)
        {
            writeln!(f, "  {pos}: {var}")?;
        }
        Ok(())
    }

    fn write_var_assigns<W: std::io::Write>(
        &self,
        f: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(f, "VARIABLE_ASSIGNMENTS")?;

        for (id, map) in self
            .variable_assignment_map
            .iter()
            .sorted_by_key(|(k, _)| **k)
        {
            writeln!(f, "  {id}: {{")?;
            for (var, loc) in map.iter().sorted() {
                writeln!(f, "    {var}: {loc}")?;
            }
            writeln!(f, "  }}")?;
        }
        Ok(())
    }

    fn write_pos_table<W: std::io::Write>(
        &self,
        f: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(f, "POSITIONS")?;
        for (position, source_loc) in
            self.position_map.iter().sorted_by_key(|(k, _)| **k)
        {
            write!(f, "  {position}:  ")?;
            source_loc.serialize(f)?;
            writeln!(f)?;
        }
        Ok(())
    }

    fn write_memory_table<W: std::io::Write>(
        &self,
        f: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(f, "MEMORY_LOCATIONS")?;

        for (loc, MemoryLocation { cell, address }) in
            self.mem_location_map.iter().sorted_by_key(|(k, _)| **k)
        {
            write!(f, "  {loc}: {cell}")?;
            if !address.is_empty() {
                write!(f, "[{}]", address.iter().join(","))?;
            }
            writeln!(f)?;
        }
        Ok(())
    }

    fn write_file_table<W: std::io::Write>(
        &self,
        f: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(f, "FILES")?;
        for (file, path) in self.file_map.iter().sorted_by_key(|(k, _)| **k) {
            writeln!(f, "  {file}: {}", path.display())?;
        }
        Ok(())
    }

    /// Attempt to lookup the line that a given position points to. Returns an error in
    /// cases when the position does not exist, the file is unavailable, or the file
    /// does not contain the indicated line.
    pub fn get_position_string(
        &self,
        pos: PositionId,
    ) -> Result<String, SourceLookupError<'_>> {
        let Some(src_loc) = self.get_position(pos) else {
            return Err(SourceLookupError::MissingPosition(pos));
        };
        // this will panic if the file doesn't exist but that would imply the table has
        // incorrect information in it
        let file_path = self.lookup_file_path(src_loc.file);

        let Ok(mut file) = File::open(file_path) else {
            return Err(SourceLookupError::MissingFile(file_path));
        };

        let mut file_contents = String::new();

        match file.read_to_string(&mut file_contents) {
            Ok(_) => {}
            Err(_) => {
                return Err(SourceLookupError::MissingFile(file_path));
            }
        }

        let Some(line) = file_contents.lines().nth(src_loc.line.as_usize() - 1)
        else {
            return Err(SourceLookupError::MissingLine {
                file: file_path,
                line: src_loc.line.as_usize(),
            });
        };

        Ok(String::from(line))
    }

    fn write_header<W: std::io::Write>(
        f: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(f, "{} #{{", SourceInfoTable::HEADER)
    }

    fn write_footer<W: std::io::Write>(
        f: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(f, "}}#")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: FileId,
    pub line: LineNum,
    pub end_line: Option<LineNum>,
}

impl SourceLocation {
    pub fn new(file: FileId, line: LineNum, end_line: Option<LineNum>) -> Self {
        Self {
            line,
            file,
            end_line,
        }
    }

    /// Write out the source location string
    pub fn serialize(
        &self,
        out: &mut impl std::io::Write,
    ) -> Result<(), std::io::Error> {
        if let Some(endline) = self.end_line {
            write!(out, "{} {}:{}", self.file, self.line, endline)
        } else {
            write!(out, "{} {}", self.file, self.line)
        }
    }
}
#[derive(Error)]
pub enum SourceInfoTableError {
    #[error("Source Info is malformed: {0}")]
    InvalidTable(String),
}

/// Any error that can emerge while attempting to pull the actual line of text that a
/// source line points to
#[derive(Error, Debug)]
pub enum SourceLookupError<'a> {
    #[error("unable to open file {0}")]
    MissingFile(&'a PathBuf),
    #[error("file {file} does not have a line {line}")]
    MissingLine { file: &'a PathBuf, line: usize },
    #[error("position id {0} does not exist")]
    MissingPosition(PositionId),
}

impl std::fmt::Debug for SourceInfoTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

pub type SourceInfoResult<T> = Result<T, SourceInfoTableError>;

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
        1: 0 1:12
        2: 0 2
    MEMORY_LOCATIONS
        0: main.reg1
        1: main.reg2
        2: main.mem1 [1,4]
    VARIABLE_ASSIGNMENTS
        0: {
            x: 0
            y: 1
            z: 2
        }
        1: {
            q: 0
        }
    POSITION_STATE_MAP
        0: 0
        2: 1
}#"#;

        let metadata = CalyxParser::parse_source_info_table(input_str)
            .unwrap()
            .unwrap();
        let file = metadata.lookup_file_path(1.into());
        assert_eq!(file, &PathBuf::from("test2.calyx"));

        let pos = metadata.lookup_position(1.into());
        assert_eq!(pos.file, 0.into());
        assert_eq!(pos.line, LineNum::new(1));
    }

    #[test]
    fn test_undefined_mem_loc() {
        let input_str = r#"sourceinfo #{
    FILES
        0: test.calyx
    POSITIONS
        0: 0 5
        1: 0 1
        2: 0 2
    MEMORY_LOCATIONS
        0: main.reg1
        2: main.mem1 [1,4]
    VARIABLE_ASSIGNMENTS
        0: {
            x: 0
            y: 1
            z: 2
        }
        1: {
            q: 0
        }
    POSITION_STATE_MAP
        0: 0
        2: 1
}#"#;
        let metadata = CalyxParser::parse_source_info_table(input_str).unwrap();
        assert!(metadata.is_err());
    }

    #[test]
    fn test_undefined_variable() {
        let input_str = r#"sourceinfo #{
    FILES
        0: test.calyx
    POSITIONS
        0: 0 5
        1: 0 1
        2: 0 2
    MEMORY_LOCATIONS
        0: main.reg1
        1: main.reg2
        2: main.mem1 [1,4]
    VARIABLE_ASSIGNMENTS
        0: {
            x: 0
            y: 1
            z: 2
        }
        1: {
            q: 0
        }
    POSITION_STATE_MAP
        0: 0
        2: 2
}#"#;
        let metadata = CalyxParser::parse_source_info_table(input_str).unwrap();
        assert!(metadata.is_err());
    }

    #[test]
    fn test_duplicate_variable_maps() {
        let input_str = r#"sourceinfo #{
    FILES
        0: test.calyx
    POSITIONS
        0: 0 5
        1: 0 1
        2: 0 2
    MEMORY_LOCATIONS
        0: main.reg1
        1: main.reg2
        2: main.mem1 [1,4]
    VARIABLE_ASSIGNMENTS
        0: {
            x: 0
            y: 1
            z: 2
        }
        1: {
            q: 0
        }
        1: {
            a: 0
        }
    POSITION_STATE_MAP
        0: 0
        2: 1
}#"#;
        let metadata = CalyxParser::parse_source_info_table(input_str).unwrap();
        assert!(metadata.is_err());
    }

    #[test]
    fn test_duplicate_variable_assignment() {
        let input_str = r#"sourceinfo #{
    FILES
        0: test.calyx
    POSITIONS
        0: 0 5
        1: 0 1
        2: 0 2
    MEMORY_LOCATIONS
        0: main.reg1
        1: main.reg2
        2: main.mem1 [1,4]
    VARIABLE_ASSIGNMENTS
        0: {
            x: 0
            y: 1
            z: 2
        }
        1: {
            q: 0
            q: 1
        }
    POSITION_STATE_MAP
        0: 0
        2: 1
}#"#;
        let metadata = CalyxParser::parse_source_info_table(input_str).unwrap();
        assert!(metadata.is_err());
    }

    #[test]
    fn test_duplicate_mem_def() {
        let input_str = r#"sourceinfo #{
    FILES
        0: test.calyx
    POSITIONS
        0: 0 5
        1: 0 1
        2: 0 2
    MEMORY_LOCATIONS
        0: main.reg1
        1: main.reg2
        1: main.mem1 [1,4]
    VARIABLE_ASSIGNMENTS
        0: {
            x: 0
            y: 1
            z: 2
        }
        1: {
            q: 0
        }
    POSITION_STATE_MAP
        0: 0
        2: 1
}#"#;
        let metadata = CalyxParser::parse_source_info_table(input_str).unwrap();
        assert!(metadata.is_err());
    }

    #[test]
    fn test_duplicate_pos_state() {
        let input_str = r#"sourceinfo #{
    FILES
        0: test.calyx
    POSITIONS
        0: 0 5
        1: 0 1
        2: 0 2
    MEMORY_LOCATIONS
        0: main.reg1
        1: main.reg2
        2: main.mem1 [1,4]
    VARIABLE_ASSIGNMENTS
        0: {
            x: 0
            y: 1
            z: 2
        }
        1: {
            q: 0
        }
    POSITION_STATE_MAP
        0: 0
        0: 1
}#"#;
        let metadata = CalyxParser::parse_source_info_table(input_str).unwrap();
        assert!(metadata.is_err());
    }

    #[test]
    fn test_duplicate_file_parse() {
        let input_str = r#"sourceinfo #{
            FILES
                0: test.calyx
                0: test2.calyx
                2: test3.calyx
            POSITIONS
                0: 0 5:6
                1: 0 1
                2: 0 2
        }#"#;
        let metadata = CalyxParser::parse_source_info_table(input_str).unwrap();
        assert!(metadata.is_err());
    }

    #[test]
    fn test_duplicate_position_parse() {
        let input_str = r#"sourceinfo #{
            FILES
                0: test.calyx
                1: test2.calyx
                2: test3.calyx
            POSITIONS
                0: 0 5
                0: 0 1
                2: 0 2
        }#"#;
        let metadata = CalyxParser::parse_source_info_table(input_str).unwrap();

        assert!(metadata.is_err());
    }

    #[test]
    fn test_serialize() {
        let mut metadata = SourceInfoTable::new_empty();
        metadata.add_file(0.into(), "test.calyx".into());
        metadata.add_file(1.into(), "test2.calyx".into());
        metadata.add_file(2.into(), "test3.calyx".into());

        metadata.add_position(0.into(), 0.into(), LineNum::new(1), None);
        metadata.add_position(
            1.into(),
            1.into(),
            LineNum::new(2),
            Some(LineNum::new(4)),
        );
        metadata.add_position(150.into(), 2.into(), LineNum::new(148), None);

        let mut serialized_str = vec![];
        metadata.serialize(&mut serialized_str).unwrap();
        let serialized_str = String::from_utf8(serialized_str).unwrap();

        let parsed_metadata =
            CalyxParser::parse_source_info_table(&serialized_str)
                .unwrap()
                .unwrap();

        assert_eq!(metadata, parsed_metadata)
    }
}
