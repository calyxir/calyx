//! Definitions for tracking source position information of Calyx programs

use std::{cmp, fmt::Write, sync::LazyLock};

use itertools::Itertools;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// Handle to a position in a [PositionTable]
/// The index refers to the index in the [PositionTable::indices] vector.
pub struct PosIdx(u32);

#[derive(Clone, Copy, PartialEq, Eq)]
/// Handle to a file in a [PositionTable]
/// The index refers to the index in the [PositionTable::files] vector.
pub struct FileIdx(u32);

/// A source program file
struct File {
    /// Name of the file
    name: Box<str>,
    /// The source code of the file
    source: Box<str>,
}

struct PosData {
    /// The file in the program. The index refers to the index in the
    /// [PositionTable::files] vector.
    file: FileIdx,
    /// Start of the span
    start: usize,
    /// End of the span
    end: usize,
}

/// Source position information for a Calyx program.
///
/// Note: This is rather large because of [`boxcar::Vec`] so it may be worth
/// putting those behind a [`Box`] in the future. Granted the PositionTable is
/// only ever constructed once statically so the extra indirection probably
/// doesn't actually buy us anything.
struct PositionTable {
    /// The source files of the program
    files: boxcar::Vec<File>,
    /// Mapping from indexes to position data
    indices: boxcar::Vec<PosData>,
}

impl Default for PositionTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PositionTable {
    /// The unknown position
    pub const UNKNOWN: PosIdx = PosIdx(0);

    /// Create a new position table where the first file and first position are unknown
    pub fn new() -> Self {
        let table = PositionTable {
            files: boxcar::Vec::new(),
            indices: boxcar::Vec::new(),
        };
        table.add_file("unknown".to_string(), "".to_string());
        let pos = table.add_pos(FileIdx(0), 0, 0);
        debug_assert!(pos == Self::UNKNOWN);
        table
    }

    /// Add a new file to the position table
    pub fn add_file(&self, name: String, source: String) -> FileIdx {
        let file = File {
            name: name.into(),
            source: source.into(),
        };
        let file_idx = self.files.push(file);
        FileIdx(file_idx as u32)
    }

    /// Return a reference to the file with the given index
    fn get_file_data(&self, file: FileIdx) -> &File {
        &self.files[file.0 as usize]
    }

    pub fn get_source(&self, file: FileIdx) -> &str {
        &self.get_file_data(file).source
    }

    /// Add a new position to the position table
    pub fn add_pos(&self, file: FileIdx, start: usize, end: usize) -> PosIdx {
        let pos = PosData { file, start, end };
        let pos_idx = self.indices.push(pos);
        PosIdx(pos_idx as u32)
    }

    fn get_pos(&self, pos: PosIdx) -> &PosData {
        &self.indices[pos.0 as usize]
    }
}

/// The global position table
pub struct GlobalPositionTable;

static GPOS_TABLE: LazyLock<PositionTable> = LazyLock::new(PositionTable::new);

impl GlobalPositionTable {
    fn get_pos(pos: PosIdx) -> &'static PosData {
        GPOS_TABLE.get_pos(pos)
    }

    fn get_file_data(file: FileIdx) -> &'static File {
        GPOS_TABLE.get_file_data(file)
    }

    pub fn get_source(file: FileIdx) -> &'static str {
        GPOS_TABLE.get_source(file)
    }

    pub fn add_file(name: String, source: String) -> FileIdx {
        GPOS_TABLE.add_file(name, source)
    }

    pub fn add_pos(file: FileIdx, start: usize, end: usize) -> PosIdx {
        GPOS_TABLE.add_pos(file, start, end)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// A position index backed by a global [PositionTable]
pub struct GPosIdx(pub PosIdx);

impl Default for GPosIdx {
    fn default() -> Self {
        Self::UNKNOWN
    }
}

impl GPosIdx {
    /// Symbol for the unknown position
    pub const UNKNOWN: GPosIdx = GPosIdx(PosIdx(0));

    /// Convert the position into an optional.
    /// Returns `None` if the position is the unknown position.
    pub fn into_option(self) -> Option<Self> {
        if self == Self::UNKNOWN {
            None
        } else {
            Some(self)
        }
    }

    /// Returns the
    /// 1. lines associated with this span
    /// 2. start position of the first line in span
    /// 3. line number of the span
    fn get_lines(&self) -> (Vec<&str>, usize, usize) {
        let pos_d = GlobalPositionTable::get_pos(self.0);
        let file = &*GlobalPositionTable::get_file_data(pos_d.file).source;

        let lines = file.split('\n').collect_vec();
        let mut pos: usize = 0;
        let mut linum: usize = 1;
        let mut collect_lines = false;
        let mut buf = Vec::new();

        let mut out_line: usize = 0;
        let mut out_idx: usize = 0;
        for l in lines {
            let next_pos = pos + l.len();
            if pos_d.start >= pos && pos_d.start <= next_pos {
                out_line = linum;
                out_idx = pos;
                collect_lines = true;
            }
            if collect_lines && pos_d.end >= pos {
                buf.push(l)
            }
            if pos_d.end <= next_pos {
                break;
            }
            pos = next_pos + 1;
            linum += 1;
        }
        (buf, out_idx, out_line)
    }

    /// returns:
    /// 1. the name of the file the span is in
    /// 2. the (inclusive) range of lines within the span
    pub fn get_line_num(&self) -> (&str, (usize, usize)) {
        let pos_data = GlobalPositionTable::get_pos(self.0);
        let file_name = &GlobalPositionTable::get_file_data(pos_data.file).name;
        let (buf, _, line_num) = self.get_lines();
        //reformat to return the range (inclusive)
        let rng = (line_num, line_num + buf.len() - 1);
        (file_name, rng)
    }

    /// Format this position with the error message `err_msg`
    pub fn format_raw<S: AsRef<str>>(&self, err_msg: S) -> String {
        let pos_d = GlobalPositionTable::get_pos(self.0);

        let (lines, pos, linum) = self.get_lines();
        let mut buf = String::new();

        let l = lines[0];
        let linum_text = format!("{} ", linum);
        let linum_space: String = " ".repeat(linum_text.len());
        let mark: String = "^".repeat(cmp::min(
            pos_d.end - pos_d.start,
            l.len() - (pos_d.start - pos),
        ));
        let space: String = " ".repeat(pos_d.start - pos);
        writeln!(buf, "{}|{}", linum_text, l).unwrap();
        write!(
            buf,
            "{}|{}{} {}",
            linum_space,
            space,
            mark,
            err_msg.as_ref()
        )
        .unwrap();
        buf
    }

    /// Format this position with filename header and the error message `err_msg`
    pub fn format<S: AsRef<str>>(&self, err_msg: S) -> String {
        let pos_d = GlobalPositionTable::get_pos(self.0);
        let name = &*GlobalPositionTable::get_file_data(pos_d.file).name;

        let mut buf = name.to_string();
        writeln!(buf).unwrap();
        write!(buf, "{}", self.format_raw(err_msg)).unwrap();
        buf
    }

    pub fn get_location(&self) -> (&str, usize, usize) {
        let pos_d = GlobalPositionTable::get_pos(self.0);
        let name = &*GlobalPositionTable::get_file_data(pos_d.file).name;
        (name, pos_d.start, pos_d.end)
    }

    /// Visualizes the span without any message or marking
    pub fn show(&self) -> String {
        let (lines, _, linum) = self.get_lines();
        let l = lines[0];
        let linum_text = format!("{} ", linum);
        format!("{}|{}\n", linum_text, l)
    }
}

/// An IR node that may contain position information.
pub trait WithPos {
    /// Copy the span associated with this node.
    fn copy_span(&self) -> GPosIdx;
}

impl WithPos for GPosIdx {
    fn copy_span(&self) -> GPosIdx {
        *self
    }
}
