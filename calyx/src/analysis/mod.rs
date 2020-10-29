//! Module for performing analysis of FuTIL programs.
//! The analyses construct data-structures that make answering certain queries
//! about FuTIL programs easier.

mod graph;
mod read_write_set;
mod schedule_conflicts;

pub use graph::GraphAnalysis;
pub use read_write_set::ReadWriteSet;
pub use schedule_conflicts::ScheduleConflicts;
