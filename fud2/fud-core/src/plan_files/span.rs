//! Types for tracking position in a plan file.

use camino::Utf8PathBuf;

/// A position of a piece of text in a source file
#[derive(Clone, Debug)]
pub struct Span {
    /// The path of the source file
    pub file_path: Option<Utf8PathBuf>,
    /// The position of the lowest byte in the associated source file
    pub lo: usize,
    /// The position of the highest byte in the associated source file
    pub hi: usize,
}
