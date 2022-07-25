//! Passes for the Calyx compiler.
mod canonical;
mod cell_share;
mod clk_insertion;
mod collapse_control;
mod comb_prop;
mod compile_empty;
mod compile_invoke;
mod compile_ref;
mod component_iniliner;
mod component_interface;
mod dead_cell_removal;
mod dead_group_removal;
mod dump_ports;
mod externalize;
mod go_insertion;
mod group_to_invoke;
mod group_to_seq;
mod hole_inliner;
mod infer_share;
mod infer_static_timing;
mod lower_guards;
mod math_utilities;
mod merge_assign;
mod merge_static_par;
mod papercut;
mod par_to_seq;
mod register_unsharing;
mod remove_comb_groups;
mod reset_insertion;
mod sharing_components;
mod simplify_guards;
mod static_par_conv;
mod synthesis_papercut;
mod top_down_compile_control;
mod top_down_static_timing;
mod unroll_bound;
mod well_formed;
mod wire_inliner;

pub use canonical::Canonicalize;
pub use cell_share::CellShare;
pub use clk_insertion::ClkInsertion;
pub use collapse_control::CollapseControl;
pub use comb_prop::CombProp;
pub use compile_empty::CompileEmpty;
pub use compile_invoke::CompileInvoke;
pub use compile_ref::CompileRef;
pub use component_iniliner::ComponentInliner;
pub use component_interface::ComponentInterface;
pub use dead_cell_removal::DeadCellRemoval;
pub use dead_group_removal::DeadGroupRemoval;
pub use externalize::Externalize;
pub use go_insertion::GoInsertion;
pub use group_to_invoke::GroupToInvoke;
pub use group_to_seq::GroupToSeq;
pub use hole_inliner::HoleInliner;
pub use infer_share::InferShare;
pub use infer_static_timing::InferStaticTiming;
pub use lower_guards::LowerGuards;
pub use merge_assign::MergeAssign;
pub use merge_static_par::MergeStaticPar;
pub use papercut::Papercut;
pub use par_to_seq::ParToSeq;
pub use register_unsharing::RegisterUnsharing;
pub use remove_comb_groups::RemoveCombGroups;
pub use reset_insertion::ResetInsertion;
pub use simplify_guards::SimplifyGuards;
pub use static_par_conv::StaticParConv;
pub use synthesis_papercut::SynthesisPapercut;
pub use top_down_compile_control::TopDownCompileControl;
pub use top_down_static_timing::TopDownStaticTiming;
pub use unroll_bound::UnrollBounded;
pub use well_formed::WellFormed;
pub use wire_inliner::WireInliner;
