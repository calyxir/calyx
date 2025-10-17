//! Types for tracking position in a plan file.

use super::session::ParseSession;

/// A position of a piece of text in a source file
#[derive(Clone, Debug)]
pub struct Span<'a> {
    /// The path of the source file
    pub sess: &'a ParseSession,
    /// The position of the lowest byte in the associated source file
    pub lo: usize,
    /// The position of the highest byte in the associated source file
    pub hi: usize,
}
