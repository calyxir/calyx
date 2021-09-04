//! Passes for the Calyx compiler.
mod clk_insertion;
mod collapse_control;
mod compile_empty;
mod compile_invoke;
mod component_interface;
mod dead_cell_removal;
mod dead_group_removal;
mod externalize;
mod go_insertion;
mod guard_canonical;
mod infer_static_timing;
mod inliner;
mod math_utilities;
mod merge_assign;
mod minimize_regs;
mod papercut;
mod par_to_seq;
mod register_unsharing;
mod remove_comb_groups;
mod reset_insertion;
mod resource_sharing;
mod sharing_components;
mod simplify_guards;
mod synthesis_papercut;
mod top_down_compile_control;
mod well_formed;

pub use clk_insertion::ClkInsertion;
pub use collapse_control::CollapseControl;
pub use compile_empty::CompileEmpty;
pub use compile_invoke::CompileInvoke;
pub use component_interface::ComponentInterface;
pub use dead_cell_removal::DeadCellRemoval;
pub use dead_group_removal::DeadGroupRemoval;
pub use externalize::Externalize;
pub use go_insertion::GoInsertion;
pub use guard_canonical::GuardCanonical;
pub use infer_static_timing::InferStaticTiming;
pub use inliner::Inliner;
pub use merge_assign::MergeAssign;
pub use minimize_regs::MinimizeRegs;
pub use papercut::Papercut;
pub use par_to_seq::ParToSeq;
pub use register_unsharing::RegisterUnsharing;
pub use remove_comb_groups::RemoveCombGroups;
pub use reset_insertion::ResetInsertion;
pub use resource_sharing::ResourceSharing;
pub use simplify_guards::SimplifyGuards;
pub use synthesis_papercut::SynthesisPapercut;
pub use top_down_compile_control::TopDownCompileControl;
pub use well_formed::WellFormed;
