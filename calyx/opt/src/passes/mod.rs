//! Passes for the Calyx compiler.
mod canonical;
mod cell_share;
mod clk_insertion;
mod collapse_control;
mod comb_prop;
mod compile_invoke;
mod compile_repeat;
mod compile_static;
mod component_inliner;
mod constant_port_prop;
mod dead_assignment_removal;
mod dead_cell_removal;
mod dead_group_removal;
mod externalize;
mod go_insertion;
mod group_to_invoke;
mod group_to_seq;
mod infer_share;
mod lower_guards;
mod merge_assign;
mod papercut;
mod profiler_instrumentation;
mod reset_insertion;
mod simplify_static_guards;
mod static_fsm_allocation;
mod static_fsm_opts;
mod static_inference;
mod static_inliner;
mod static_promotion;
mod static_repeat_fsm_allocation;
mod uniquefy_enables;

// mod simplify_guards;
mod add_guard;
mod data_path_infer;
mod default_assigns;
mod dump_ports;
mod remove_ids;
mod simplify_with_control;
mod synthesis_papercut;
mod top_down_compile_control;
mod unroll_bound;
mod well_formed;
mod wire_inliner;
mod wrap_main;

pub use canonical::Canonicalize;
pub use cell_share::CellShare;
pub use clk_insertion::ClkInsertion;
pub use collapse_control::CollapseControl;
pub use comb_prop::CombProp;
pub use compile_invoke::CompileInvoke;
pub use compile_repeat::CompileRepeat;
pub use compile_static::CompileStatic;
pub use component_inliner::ComponentInliner;
pub use constant_port_prop::ConstantPortProp;
pub use data_path_infer::DataPathInfer;
pub use dead_assignment_removal::DeadAssignmentRemoval;
pub use dead_cell_removal::DeadCellRemoval;
pub use dead_group_removal::DeadGroupRemoval;
pub use dump_ports::DumpResults;
pub use externalize::Externalize;
pub use go_insertion::GoInsertion;
pub use group_to_invoke::GroupToInvoke;
pub use group_to_seq::GroupToSeq;
pub use infer_share::InferShare;
pub use lower_guards::LowerGuards;
pub use merge_assign::MergeAssign;
pub use papercut::Papercut;
pub use profiler_instrumentation::ProfilerInstrumentation;
pub use remove_ids::RemoveIds;
pub use reset_insertion::ResetInsertion;
pub use simplify_static_guards::SimplifyStaticGuards;
pub use simplify_with_control::SimplifyWithControl;
pub use static_fsm_allocation::StaticFSMAllocation;
pub use static_fsm_opts::StaticFSMOpts;
pub use static_inference::StaticInference;
pub use static_inliner::StaticInliner;
pub use static_promotion::StaticPromotion;
pub use static_repeat_fsm_allocation::StaticRepeatFSMAllocation;
pub use uniquefy_enables::UniquefyEnables;
pub use unroll_bound::UnrollBounded;
// pub use simplify_guards::SimplifyGuards;
pub use add_guard::AddGuard;
pub use default_assigns::DefaultAssigns;
pub use synthesis_papercut::SynthesisPapercut;
pub use top_down_compile_control::TopDownCompileControl;
pub use well_formed::WellFormed;
pub use wire_inliner::WireInliner;
pub use wrap_main::WrapMain;
