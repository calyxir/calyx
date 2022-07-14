//! Analysis for Dominator Map.
//!
//! Constructs and supports queries regarding DominatorMap

mod dominator_map;
mod node_analysis;

pub use dominator_map::DominatorMap;
pub use node_analysis::NodeReads;
pub use node_analysis::NodeSearch;
