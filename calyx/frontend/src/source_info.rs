use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
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
    variable_assignment_map: HashMap<
        VariableAssignmentId,
        HashMap<VariableName, VariableDefinition>,
    >,
    /// collects the mapping from positions representing a point in the control
    /// program to the set of variable assignments for that position
    position_state_map: HashMap<PositionId, VariableAssignmentId>,
    /// stores information about the source information used by the program
    type_table: HashMap<TypeId, SourceType>,
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
    ) -> Option<&HashMap<VariableName, VariableDefinition>> {
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
            type_table: HashMap::new(),
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
            Vec<(VariableName, VariableDefinition)>,
        )> = vec![];
        let types: Vec<(TypeId, SourceType)> = vec![];

        Self::new(files, positions, loc, variable_assigns, states, types)
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
                impl IntoIterator<Item = (VariableName, VariableDefinition)>,
            ),
        >,
        states: impl IntoIterator<Item = (PositionId, VariableAssignmentId)>,
        types: impl IntoIterator<Item = (TypeId, SourceType)>,
    ) -> SourceInfoResult<Self> {
        let files = files.into_iter();
        let positions = positions.into_iter();
        let locations = locations.into_iter();
        let vars = variable_assigns.into_iter();
        let states = states.into_iter();
        let types = types.into_iter();

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
        let mut type_map = HashMap::new();

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

        let mut types_referenced: HashSet<TypeId> = HashSet::new();

        for (assign_label, assigns) in vars {
            let mut mapping = HashMap::new();
            for (name, location) in assigns {
                for loc in location.referenced_memory_locations() {
                    if !memory_location_map.contains_key(&loc) {
                        // unknown memory location
                        return Err(SourceInfoTableError::InvalidTable(
                            format!(
                                "Memory location {loc} is referenced but never defined"
                            ),
                        ));
                    }
                }

                if let Some(ty) = location.referenced_type() {
                    types_referenced.insert(ty);
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

        for (id, source_type) in types {
            types_referenced.extend(source_type.types_referenced());

            if type_map.insert(id, source_type).is_some() {
                return Err(SourceInfoTableError::InvalidTable(format!(
                    "multiple definitions for type id {id}"
                )));
            }
        }

        for ty in types_referenced {
            if !type_map.contains_key(&ty) {
                return Err(SourceInfoTableError::InvalidTable(format!(
                    "type id {ty} is referenced but never defined"
                )));
            }
        }

        Ok(SourceInfoTable {
            file_map,
            position_map,
            mem_location_map: memory_location_map,
            variable_assignment_map: variable_map,
            position_state_map: state_map,
            type_table: type_map,
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
            && self.position_state_map.is_empty()
            && self.type_table.is_empty())
        {
            self.write_memory_table(&mut f)?;
            self.write_var_assigns(&mut f)?;
            self.write_pos_state_table(&mut f)?;
            self.write_type_table(&mut f)?;
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
            for (var, loc) in map.iter().sorted_by_key(|(v, _)| *v) {
                write!(f, "    {var}:")?;
                match loc {
                    VariableDefinition::Untyped(memory_location_id) => {
                        writeln!(f, " {memory_location_id}")?;
                    }
                    VariableDefinition::Typed(variable_layout) => {
                        write!(
                            f,
                            " ty {}, {}",
                            variable_layout.type_info,
                            variable_layout.layout_fn
                        )?;
                        for loc in variable_layout.layout_args.iter() {
                            write!(f, " {loc}")?;
                        }
                        writeln!(f)?;
                    }
                }
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
            write!(f, "  {position}: ")?;
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

    fn write_type_table<W: std::io::Write>(
        &self,
        f: &mut W,
    ) -> Result<(), std::io::Error> {
        writeln!(f, "TYPES")?;
        for (id, source_type) in
            self.type_table.iter().sorted_by_key(|(k, _)| **k)
        {
            write!(f, "    {id}: {{ ",)?;
            match source_type {
                SourceType::Array { ty, length } => {
                    write!(f, "{ty}; {length}")?;
                }
                SourceType::Struct { fields } => {
                    if let Some((name, ty)) = fields.first() {
                        write!(f, "{name}: {ty}")?;
                    }

                    for (name, ty) in fields.iter().skip(1) {
                        write!(f, ", {name}: {ty}")?;
                    }
                }
            }

            writeln!(f, " }}")?;
        }
        Ok(())
    }

    /// Attempt to lookup the line that a given position points to. Returns an error in
    /// cases when the position does not exist, the file is unavailable, or the file
    /// does not contain the indicated line.
    pub fn get_position_string(
        &self,
        pos: PositionId,
    ) -> Result<String, SourceInfoTableError> {
        let Some(src_loc) = self.get_position(pos) else {
            return Err(SourceInfoTableError::LookupFailure(format!(
                "position {pos} does not exist"
            )));
        };
        // this will panic if the file doesn't exist but that would imply the table has
        // incorrect information in it
        let file_path = self.lookup_file_path(src_loc.file);

        let Ok(mut file) = File::open(file_path) else {
            return Err(SourceInfoTableError::LookupFailure(format!(
                "unable to open file '{}'",
                file_path.display()
            )));
        };

        let mut file_contents = String::new();

        match file.read_to_string(&mut file_contents) {
            Ok(_) => {}
            Err(e) => {
                return Err(SourceInfoTableError::LookupFailure(format!(
                    "read of file '{}' failed with error {e}",
                    file_path.display()
                )));
            }
        }

        let Some(line) = file_contents.lines().nth(src_loc.line.as_usize() - 1)
        else {
            return Err(SourceInfoTableError::LookupFailure(format!(
                "file '{}' does not contain a line {}",
                file_path.display(),
                src_loc.line.as_usize()
            )));
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum PrimitiveType {
    Uint(u32),
    Sint(u32),
    Bool,
    Bitfield(u32),
}

impl Display for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveType::Uint(x) => write!(f, "u{x}"),
            PrimitiveType::Sint(x) => write!(f, "i{x}"),
            PrimitiveType::Bool => write!(f, "bool"),
            PrimitiveType::Bitfield(x) => write!(f, "b{x}"),
        }
    }
}

impl PrimitiveType {
    pub fn type_size(&self) -> usize {
        match self {
            PrimitiveType::Uint(width) => *width as usize,
            PrimitiveType::Sint(width) => *width as usize,
            PrimitiveType::Bool => 1_usize,
            PrimitiveType::Bitfield(width) => *width as usize,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    Primitive(PrimitiveType),
    Composite(TypeId),
}

impl Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldType::Primitive(primitive_type) => primitive_type.fmt(f),
            FieldType::Composite(type_id) => type_id.fmt(f),
        }
    }
}

impl FieldType {
    pub fn type_size(&self, type_map: &HashMap<TypeId, SourceType>) -> usize {
        match self {
            FieldType::Primitive(primitive_type) => primitive_type.type_size(),
            FieldType::Composite(type_id) => {
                type_map[type_id].type_size(type_map)
            }
        }
    }

    /// Return the number of primitive types that must be mapped for this type
    pub fn entry_count(&self, type_map: &HashMap<TypeId, SourceType>) -> usize {
        match self {
            FieldType::Primitive(_) => 1,
            FieldType::Composite(type_id) => {
                type_map[type_id].entry_count(type_map)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceType {
    Array {
        ty: FieldType,
        length: u32,
    },
    Struct {
        fields: Vec<(VariableName, FieldType)>,
    },
}

impl SourceType {
    pub fn type_size(&self, type_map: &HashMap<TypeId, SourceType>) -> usize {
        match self {
            SourceType::Array { ty, length } => {
                ty.type_size(type_map) * (*length as usize)
            }
            SourceType::Struct { fields } => fields
                .iter()
                .fold(0, |acc, (_, ty)| acc + ty.type_size(type_map)),
        }
    }

    pub fn types_referenced(&self) -> Vec<TypeId> {
        match self {
            SourceType::Array { ty, .. } => {
                if let FieldType::Composite(id) = ty {
                    vec![*id]
                } else {
                    vec![]
                }
            }
            SourceType::Struct { fields } => fields
                .iter()
                .filter_map(|(_, ty)| {
                    if let FieldType::Composite(id) = ty {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }

    /// Return the number of primitive types that must be mapped for this type
    pub fn entry_count(&self, type_map: &HashMap<TypeId, SourceType>) -> usize {
        match self {
            SourceType::Array { ty, length } => {
                let entries_per_field = ty.entry_count(type_map);
                entries_per_field * (*length as usize)
            }
            SourceType::Struct { fields } => fields
                .iter()
                .fold(0, |acc, (_, ty)| acc + ty.entry_count(type_map)),
        }
    }
}

/// ID for types from the source language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeId(Word);

impl TypeId {
    pub fn new(v: Word) -> Self {
        Self(v)
    }
}

impl Display for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> core::fmt::Result {
        <Word as Display>::fmt(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// A thin wrapper over a String for names used by the source info table
pub struct VariableName {
    name: String,
    /// a bool tracking whether or not this name came from a string literal for
    /// purposes of serialization
    name_is_literal: bool,
}

impl Display for VariableName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.name_is_literal {
            write!(f, "\"{}\"", self.name)
        } else {
            self.name.fmt(f)
        }
    }
}

impl VariableName {
    pub fn new(name: String, name_is_literal: bool) -> Self {
        Self {
            name,
            name_is_literal,
        }
    }

    pub fn new_non_literal(name: String) -> Self {
        Self {
            name,
            name_is_literal: false,
        }
    }

    pub fn new_literal(name: String) -> Self {
        Self {
            name,
            name_is_literal: true,
        }
    }

    pub fn into_string(self) -> String {
        self.name
    }

    /// flips the name_is_literal bool. should only be used when searching for
    /// a value in a hashmap
    pub fn flip_is_literal(&mut self) {
        self.name_is_literal = !self.name_is_literal
    }

    pub fn set_is_literal(&mut self) {
        self.name_is_literal = true
    }

    pub fn unset_is_literal(&mut self) {
        self.name_is_literal = false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutFunction {
    /// Standard layout function which maps all fields into a single memory
    /// slot / register. This function must be given exactly a single argument
    Packed,
    /// Standard layout function which maps each entry in the variable structure
    /// to a distinct register / memory location. Must take N arguments where N
    /// is the number of entries for the type.
    Split,
}

impl Display for LayoutFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutFunction::Packed => write!(f, "packed"),
            LayoutFunction::Split => write!(f, "split"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableLayout {
    pub type_info: FieldType,
    pub layout_fn: LayoutFunction,
    pub layout_args: Box<[MemoryLocationId]>,
}

impl VariableLayout {
    pub fn new(
        type_info: FieldType,
        layout_fn: LayoutFunction,
        layout_args: impl IntoIterator<Item = MemoryLocationId>,
    ) -> Self {
        Self {
            type_info,
            layout_fn,
            layout_args: layout_args.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableDefinition {
    /// There is no associated type information with this variable. This is for
    /// definitions of the form `name: MEMORY_LOCATION`
    Untyped(MemoryLocationId),
    /// The metadata defines a type for this source variable
    Typed(VariableLayout),
}

impl VariableDefinition {
    pub fn referenced_memory_locations(
        &self,
    ) -> Box<dyn Iterator<Item = MemoryLocationId> + '_> {
        match self {
            VariableDefinition::Untyped(memory_location_id) => {
                Box::new(std::iter::once(*memory_location_id))
            }
            VariableDefinition::Typed(variable_layout) => {
                Box::new(variable_layout.layout_args.iter().copied())
            }
        }
    }

    pub fn referenced_type(&self) -> Option<TypeId> {
        if let Self::Typed(var_layout) = self
            && let FieldType::Composite(ty) = var_layout.type_info
        {
            Some(ty)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_typed(&self) -> Option<&VariableLayout> {
        if let Self::Typed(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_untyped(&self) -> Option<&MemoryLocationId> {
        if let Self::Untyped(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Error)]
pub enum SourceInfoTableError {
    /// A fatal error representing a malformed table
    #[error("source info is malformed. {0}")]
    InvalidTable(String),
    /// A non-fatal error representing a failed lookup
    #[error("source lookup failed. {0}")]
    LookupFailure(String),
}

impl std::fmt::Debug for SourceInfoTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

pub type SourceInfoResult<T> = Result<T, SourceInfoTableError>;

#[cfg(test)]
mod tests {
    use super::SourceInfoTable;
    use crate::{parser::CalyxParser, source_info::LineNum};
    use std::path::PathBuf;

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
        3: main.mem1 [0,1]
        4: main.mem1 [0,2]
    VARIABLE_ASSIGNMENTS
        0: {
            x: 0
            y: 1
            z: ty 2, split 2 3 4
        }
        1: {
            q: ty 0, packed 1
        }
    POSITION_STATE_MAP
        0: 0
        2: 1
    TYPES
        0: { 0: u4, 1: i6 }
        1: { bool; 15 }
        2: { coordinate: 0, bvec: 1 }

}#"#;

        let metadata = CalyxParser::parse_source_info_table(input_str)
            .unwrap()
            .unwrap();
        let file = metadata.lookup_file_path(1.into());
        assert_eq!(file, &PathBuf::from("test2.calyx"));

        let pos = metadata.lookup_position(1.into());
        assert_eq!(pos.file, 0.into());
        assert_eq!(pos.line, LineNum::new(1));

        let mut serialized_str = vec![];
        metadata.serialize(&mut serialized_str).unwrap();
        let serialized_str = String::from_utf8(serialized_str).unwrap();
        eprintln!("{}", &serialized_str);
        let parsed_metadata =
            CalyxParser::parse_source_info_table(&serialized_str)
                .unwrap()
                .unwrap();

        assert_eq!(metadata, parsed_metadata)
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
    TYPES
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
    TYPES
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
    TYPES
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
    TYPES
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
    TYPES
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
    TYPES
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
    fn test_unknown_type() {
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
            x: ty 2, packed 0
            y: 1
            z: 2
        }
        1: {
            q: 0
        }
    POSITION_STATE_MAP
        0: 0
        1: 1
    TYPES
}#"#;
        let metadata = CalyxParser::parse_source_info_table(input_str).unwrap();
        assert!(metadata.is_err());
        eprintln!("{}", metadata.unwrap_err());
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
