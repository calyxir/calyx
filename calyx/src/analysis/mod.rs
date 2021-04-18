//! Analysis for Calyx programs.
//!
//! The analyses construct data-structures that make answering certain queries
//! about Calyx programs easier.

mod graph;
mod graph_coloring;
mod live_range_analysis;
mod reaching_defns;
mod read_write_set;
mod schedule_conflicts;
mod variable_detection;

pub use graph::GraphAnalysis;
pub use graph_coloring::GraphColoring;
pub use live_range_analysis::LiveRangeAnalysis;
pub use reaching_defns::ReachingDefinitionAnalysis;
pub use read_write_set::ReadWriteSet;
pub use schedule_conflicts::ScheduleConflicts;
pub use variable_detection::VariableDetection;
