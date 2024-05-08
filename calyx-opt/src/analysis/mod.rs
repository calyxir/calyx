//! Analysis for Calyx programs.
//!
//! The analyses construct data-structures that make answering certain queries
//! about Calyx programs easier.

mod compaction_analysis;
mod compute_static;
mod control_id;
mod control_order;
mod control_ports;
mod dataflow_order;
mod domination_analysis;
mod graph;
mod graph_coloring;
mod inference_analysis;
mod live_range_analysis;
mod port_interface;
mod promotion_analysis;
pub mod reaching_defns;
mod read_write_set;
mod schedule_conflicts;
mod share_set;
mod static_par_timing;
mod static_schedule;
mod variable_detection;

pub use compaction_analysis::CompactionAnalysis;
pub use compute_static::IntoStatic;
pub use compute_static::WithStatic;
pub use control_id::ControlId;
pub use control_order::ControlOrder;
pub use control_ports::ControlPorts;
pub use dataflow_order::DataflowOrder;
pub use domination_analysis::DominatorMap;
pub use graph::GraphAnalysis;
pub use graph_coloring::GraphColoring;
pub use inference_analysis::GoDone;
pub use inference_analysis::InferenceAnalysis;
pub use live_range_analysis::LiveRangeAnalysis;
pub use port_interface::PortInterface;
pub use promotion_analysis::PromotionAnalysis;
pub use read_write_set::{AssignmentAnalysis, ReadWriteSet};
pub use schedule_conflicts::ScheduleConflicts;
pub use share_set::ShareSet;
pub use static_par_timing::StaticParTiming;
pub use static_schedule::{StaticFSM, StaticSchedule};
pub use variable_detection::VariableDetection;
