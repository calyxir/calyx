use std::path::PathBuf;

use crate::{GPosIdx, WithPos};

/// A positioned string.
#[derive(Clone, Debug, Default)]
pub struct PosString {
    data: String,
    span: GPosIdx,
}

impl From<String> for PosString {
    fn from(data: String) -> PosString {
        PosString {
            data,
            span: GPosIdx::UNKNOWN,
        }
    }
}

impl From<PosString> for String {
    fn from(value: PosString) -> String {
        value.data
    }
}

impl From<PosString> for PathBuf {
    fn from(value: PosString) -> Self {
        value.data.into()
    }
}

impl ToString for PosString {
    fn to_string(&self) -> String {
        self.data.to_string()
    }
}

impl AsRef<str> for PosString {
    fn as_ref(&self) -> &str {
        &self.data
    }
}

impl AsRef<std::path::Path> for PosString {
    fn as_ref(&self) -> &std::path::Path {
        self.data.as_ref()
    }
}

impl WithPos for PosString {
    fn copy_span(&self) -> GPosIdx {
        self.span
    }
}

impl PosString {
    /// Construct a nw PosString from a String and a span.
    pub fn new(data: String, span: GPosIdx) -> Self {
        Self { data, span }
    }

    /// Add a span to an existing PosString.
    pub fn with_span(mut self, span: GPosIdx) -> Self {
        self.span = span;
        self
    }
}
