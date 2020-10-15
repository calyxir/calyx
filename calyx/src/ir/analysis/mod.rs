//! Module for performing analysis on FuTIL components.
//! This module contains analyses that construct external data-structures
//! to make certain kinds of analysis more efficient.
//! This doesn't include queries that can be answered by directly looking at the IR.

mod graph;

pub use graph::GraphAnalysis;
