//! Analysis for Calyx programs.
//!
//! The analyses construct data-structures that make answering certain queries
//! about Calyx programs easier.

mod control_ports;
mod dataflow_order;
mod graph;
mod graph_coloring;
mod live_range_analysis;
pub mod reaching_defns;
mod read_write_set;
mod port_interface;
mod schedule_conflicts;
mod variable_detection;

pub use control_ports::ControlPorts;
pub use dataflow_order::DataflowOrder;
pub use graph::GraphAnalysis;
pub use graph_coloring::GraphColoring;
pub use live_range_analysis::LiveRangeAnalysis;
pub use read_write_set::ReadWriteSet;
pub use port_interface::PortInterface;
pub use schedule_conflicts::ScheduleConflicts;
pub use variable_detection::VariableDetection;
