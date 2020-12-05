//! Module for performing analysis of FuTIL programs.
//! The analyses construct data-structures that make answering certain queries
//! about FuTIL programs easier.

mod graph;
mod graph_coloring;
mod read_write_set;
mod schedule_conflicts;
mod variable_detection;

pub use graph::GraphAnalysis;
pub use graph_coloring::GraphColoring;
pub use read_write_set::ReadWriteSet;
pub use schedule_conflicts::ScheduleConflicts;
pub use variable_detection::VariableDetection;
