//! Module for performing analysis of FuTIL programs.
//! The analyses construct data-structures that make answering certain queries
//! about FuTIL programs easier.

mod graph;
mod schedule_conflicts;
mod read_write_set;

pub use graph::GraphAnalysis;
pub use schedule_conflicts::ScheduleConflicts;
pub use read_write_set::ReadWriteSet;
