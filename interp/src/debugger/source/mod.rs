//! This module contains the modules used for source-code attribution
pub(crate) mod metadata_parser;
pub(crate) mod new_parser;
pub mod structures;

pub use structures::{NamedTag, SourceMap};
