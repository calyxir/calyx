//! Module for performing analysis of FuTIL programs.
//! The analyses construct data-structures that make answering certain queries
//! about FuTIL programs easier.

mod graph;
mod schedule_conflicts;

pub use graph::GraphAnalysis;
pub use schedule_conflicts::ScheduleConflicts;
