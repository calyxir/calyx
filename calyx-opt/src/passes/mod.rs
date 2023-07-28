//! Passes for the Calyx compiler.
mod attribute_promotion;
mod canonical;
mod cell_share;
mod clk_insertion;
mod collapse_control;
mod comb_prop;
mod compile_empty;
mod compile_invoke;
mod compile_ref;
mod compile_repeat;
mod compile_static;
mod component_iniliner;
mod dead_assignment_removal;
mod dead_cell_removal;
mod dead_group_removal;
mod dump_ports;
mod externalize;
mod go_insertion;
mod group_to_invoke;
mod group_to_seq;
mod hole_inliner;
mod infer_share;
mod lower_guards;
mod math_utilities;
mod merge_assign;
mod papercut;
mod par_to_seq;
mod register_unsharing;
mod remove_ids;
mod reset_insertion;
mod simplify_static_guards;
mod static_inliner;
mod static_promotion;
mod sync;
// mod simplify_guards;
mod data_path_infer;
mod discover_external;
mod simplify_with_control;
mod synthesis_papercut;
mod top_down_compile_control;
mod top_down_static_timing;
mod unroll_bound;
mod well_formed;
mod wire_inliner;
mod wrap_main;
mod build_invoke;

pub use attribute_promotion::AttributePromotion;
pub use canonical::Canonicalize;
pub use cell_share::CellShare;
pub use clk_insertion::ClkInsertion;
pub use collapse_control::CollapseControl;
pub use comb_prop::CombProp;
pub use compile_empty::CompileEmpty;
pub use compile_invoke::CompileInvoke;
pub use compile_ref::CompileRef;
pub use compile_repeat::CompileRepeat;
pub use compile_static::CompileStatic;
pub use component_iniliner::ComponentInliner;
pub use data_path_infer::DataPathInfer;
pub use dead_assignment_removal::DeadAssignmentRemoval;
pub use dead_cell_removal::DeadCellRemoval;
pub use dead_group_removal::DeadGroupRemoval;
pub use discover_external::DiscoverExternal;
pub use externalize::Externalize;
pub use go_insertion::GoInsertion;
pub use group_to_invoke::GroupToInvoke;
pub use group_to_seq::GroupToSeq;
pub use hole_inliner::HoleInliner;
pub use infer_share::InferShare;
pub use lower_guards::LowerGuards;
pub use merge_assign::MergeAssign;
pub use papercut::Papercut;
pub use par_to_seq::ParToSeq;
pub use register_unsharing::RegisterUnsharing;
pub use remove_ids::RemoveIds;
pub use reset_insertion::ResetInsertion;
pub use simplify_static_guards::SimplifyStaticGuards;
pub use simplify_with_control::SimplifyWithControl;
pub use static_inliner::StaticInliner;
pub use static_promotion::StaticPromotion;
pub use sync::CompileSync;
pub use sync::CompileSyncWithoutSyncReg;
// pub use simplify_guards::SimplifyGuards;
pub use synthesis_papercut::SynthesisPapercut;
pub use top_down_compile_control::TopDownCompileControl;
pub use top_down_static_timing::TopDownStaticTiming;
pub use unroll_bound::UnrollBounded;
pub use well_formed::WellFormed;
pub use wire_inliner::WireInliner;
pub use wrap_main::WrapMain;
