//! Passes for the FuTIL compiler.
mod collapse_control;
mod compile_control;
mod compile_empty;
mod component_interface;
mod externalize;
mod inliner;
mod well_formed;
//mod merge_assign;
mod clk_insertion;
mod go_insertion;
mod papercut;
mod remove_external_memories;
mod static_timing;

pub use clk_insertion::ClkInsertion;
pub use collapse_control::CollapseControl;
pub use compile_control::CompileControl;
pub use compile_empty::CompileEmpty;
pub use component_interface::ComponentInterface;
pub use externalize::Externalize;
pub use go_insertion::GoInsertion;
pub use inliner::Inliner;
pub use papercut::Papercut;
pub use remove_external_memories::RemoveExternalMemories;
pub use static_timing::StaticTiming;
pub use well_formed::WellFormed;
