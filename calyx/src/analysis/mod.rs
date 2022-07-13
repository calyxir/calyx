//! Analysis for Calyx programs.
//!
//! The analyses construct data-structures that make answering certain queries
//! about Calyx programs easier.

mod control_order;
mod control_ports;
mod dataflow_order;
mod dominator_map;
mod graph;
mod graph_coloring;
mod live_range_analysis;
mod node_analysis;
mod port_interface;
pub mod reaching_defns;
mod read_write_set;
mod schedule_conflicts;
mod share_set;
mod variable_detection;

pub use control_order::ControlOrder;
pub use control_ports::ControlPorts;
pub use dataflow_order::DataflowOrder;
pub use dominator_map::DominatorMap;
pub use graph::GraphAnalysis;
pub use graph_coloring::GraphColoring;
pub use live_range_analysis::LiveRangeAnalysis;
pub use node_analysis::CellSearch;
pub use node_analysis::NodeReads;
pub use port_interface::PortInterface;
pub use read_write_set::ReadWriteSet;
pub use schedule_conflicts::ScheduleConflicts;
pub use share_set::ShareSet;
pub use variable_detection::VariableDetection;
