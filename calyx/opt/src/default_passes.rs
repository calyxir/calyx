//! Defines the default passes available to [PassManager].
use crate::pass_manager::PassResult;
use crate::passes::{
    AddGuard, Canonicalize, CellShare, ClkInsertion, CollapseControl, CombProp,
    CompileInvoke, CompileRepeat, CompileStatic, ComponentInliner,
    ConstantPortProp, DataPathInfer, DeadAssignmentRemoval, DeadCellRemoval,
    DeadGroupRemoval, DefaultAssigns, Externalize, GoInsertion, GroupToInvoke,
    GroupToSeq, InferShare, LowerGuards, MergeAssign, Papercut,
    ProfilerInstrumentation, RemoveIds, ResetInsertion, SimplifyStaticGuards,
    SimplifyWithControl, StaticFSMAllocation, StaticFSMOpts, StaticInference,
    StaticInliner, StaticPromotion, StaticRepeatFSMAllocation,
    SynthesisPapercut, TopDownCompileControl, UniquefyEnables, UnrollBounded,
    WellFormed, WireInliner, WrapMain,
};
use crate::passes_experimental::{
    CompileSync, CompileSyncWithoutSyncReg, DiscoverExternal, ExternalToRef,
    FSMAnnotator, HoleInliner, Metadata, ParToSeq, RegisterUnsharing,
};
use crate::traversal::Named;
use crate::{pass_manager::PassManager, register_alias};

impl PassManager {
    pub fn default_passes() -> PassResult<Self> {
        // Construct the pass manager and register all passes.
        let mut pm = PassManager::default();

        // Validation passes
        pm.register_diagnostic::<WellFormed>()?;
        pm.register_diagnostic::<Papercut>()?;
        pm.register_pass::<Canonicalize>()?;

        // Optimization passes
        pm.register_pass::<CombProp>()?;
        pm.register_pass::<ComponentInliner>()?;
        pm.register_pass::<CollapseControl>()?;
        pm.register_pass::<DeadAssignmentRemoval>()?;
        pm.register_pass::<DeadCellRemoval>()?;
        pm.register_pass::<DeadGroupRemoval>()?;
        pm.register_pass::<GroupToSeq>()?;
        pm.register_pass::<InferShare>()?;
        pm.register_pass::<CellShare>()?;
        pm.register_pass::<StaticInference>()?;
        pm.register_pass::<StaticPromotion>()?;
        pm.register_pass::<SimplifyStaticGuards>()?;
        pm.register_pass::<DataPathInfer>()?;

        // Compilation passes
        pm.register_pass::<StaticInliner>()?;
        pm.register_pass::<StaticFSMAllocation>()?;
        pm.register_pass::<StaticRepeatFSMAllocation>()?;
        pm.register_pass::<StaticFSMOpts>()?;
        pm.register_pass::<CompileStatic>()?;
        pm.register_pass::<CompileInvoke>()?;
        pm.register_pass::<CompileRepeat>()?;
        pm.register_pass::<SimplifyWithControl>()?;
        pm.register_pass::<TopDownCompileControl>()?;
        pm.register_pass::<CompileSync>()?;
        pm.register_pass::<CompileSyncWithoutSyncReg>()?;
        pm.register_pass::<AddGuard>()?;
        pm.register_pass::<FSMAnnotator>()?;

        // Lowering passes
        pm.register_pass::<GoInsertion>()?;
        pm.register_pass::<WireInliner>()?;
        pm.register_pass::<ClkInsertion>()?;
        pm.register_pass::<ResetInsertion>()?;
        pm.register_pass::<MergeAssign>()?;
        pm.register_pass::<WrapMain>()?;
        pm.register_pass::<DefaultAssigns>()?;

        // Enabled in the synthesis compilation flow
        pm.register_diagnostic::<SynthesisPapercut>()?;
        pm.register_pass::<Externalize>()?;

        // Disabled by default
        pm.register_pass::<DiscoverExternal>()?;
        pm.register_pass::<UnrollBounded>()?;
        pm.register_pass::<RegisterUnsharing>()?;
        pm.register_pass::<GroupToInvoke>()?;
        pm.register_pass::<ParToSeq>()?;
        pm.register_pass::<LowerGuards>()?;
        pm.register_pass::<HoleInliner>()?;
        pm.register_pass::<RemoveIds>()?;
        pm.register_pass::<ExternalToRef>()?;
        pm.register_pass::<ConstantPortProp>()?;

        // instrumentation pass to collect profiling information
        pm.register_pass::<ProfilerInstrumentation>()?;
        pm.register_pass::<UniquefyEnables>()?;

        //add metadata
        pm.register_pass::<Metadata>()?;

        register_alias!(pm, "validate", [WellFormed, Papercut, Canonicalize]);
        register_alias!(
            pm,
            "pre-opt",
            [
                DataPathInfer,
                CollapseControl, // Run it twice: once at beginning of pre-opt, once at end.
                CompileSyncWithoutSyncReg,
                GroupToSeq, // FIXME: may make programs *slower*
                DeadAssignmentRemoval,
                GroupToInvoke, // Creates Dead Groups potentially
                InferShare,
                ComponentInliner,
                CombProp,
                ConstantPortProp,
                DeadCellRemoval, // Clean up dead wires left by CombProp
                CellShare,       // LiveRangeAnalaysis should handle comb groups
                SimplifyWithControl, // Must run before compile-invoke
                CompileInvoke,   // creates dead comb groups
                StaticInference,
                StaticPromotion,
                CompileRepeat,
                DeadGroupRemoval, // Since previous passes potentially create dead groups
                CollapseControl,
            ]
        );

        register_alias!(
            pm,
            "fsm-opt",
            [
                DataPathInfer,
                CollapseControl,
                CompileSyncWithoutSyncReg,
                GroupToSeq, // FIXME: may make programs *slower*
                DeadAssignmentRemoval,
                GroupToInvoke,
                ComponentInliner,
                CombProp,
                DeadCellRemoval,
                CellShare,
                SimplifyWithControl,
                CompileInvoke,
                StaticInference,
                StaticPromotion,
                DeadGroupRemoval,
                CollapseControl,
                StaticRepeatFSMAllocation,
                StaticFSMAllocation,
                DeadGroupRemoval,
                MergeAssign,
                CompileRepeat,
                TopDownCompileControl,
            ]
        );

        register_alias!(
            pm,
            "compile-fsm",
            [
                DataPathInfer,
                CollapseControl,
                CompileSyncWithoutSyncReg,
                GroupToSeq,
                DeadAssignmentRemoval,
                GroupToInvoke,
                ComponentInliner,
                CombProp,
                DeadCellRemoval,
                SimplifyWithControl,
                CompileInvoke,
                StaticInference,
                StaticPromotion,
                DeadGroupRemoval,
                CollapseControl,
                FSMAnnotator,
            ]
        );

        register_alias!(
            pm,
            "compile",
            [
                StaticInliner,
                MergeAssign, // Static inliner generates lots of assigns
                DeadGroupRemoval, // Static inliner generates lots of dead groups
                AddGuard,
                SimplifyStaticGuards,
                StaticFSMOpts,
                CompileStatic,
                DeadGroupRemoval,
                TopDownCompileControl,
            ]
        );
        register_alias!(
            pm,
            "post-opt",
            [
                DeadGroupRemoval,
                CombProp,
                DeadAssignmentRemoval,
                DeadCellRemoval
            ]
        );
        register_alias!(
            pm,
            "lower",
            [
                WrapMain,
                GoInsertion,
                WireInliner,
                ClkInsertion,
                ResetInsertion,
                MergeAssign,
                DefaultAssigns,
            ]
        );

        // Default flow
        register_alias!(
            pm,
            "all",
            ["validate", "pre-opt", "compile", "post-opt", "lower",]
        );

        // profiler flow for pass explorer access
        register_alias!(
            pm,
            "profiler",
            [
                "validate",
                CompileInvoke,
                UniquefyEnables,
                ProfilerInstrumentation,
                DeadGroupRemoval,
                // "pre-opt" without GroupToSeq
                DataPathInfer,
                CollapseControl, // Run it twice: once at beginning of pre-opt, once at end.
                CompileSyncWithoutSyncReg,
                DeadAssignmentRemoval,
                GroupToInvoke, // Creates Dead Groups potentially
                InferShare,
                ComponentInliner,
                CombProp,
                ConstantPortProp,
                DeadCellRemoval, // Clean up dead wires left by CombProp
                CellShare,       // LiveRangeAnalaysis should handle comb groups
                SimplifyWithControl, // Must run before compile-invoke
                CompileInvoke,   // creates dead comb groups
                StaticInference,
                StaticPromotion,
                CompileRepeat,
                DeadGroupRemoval, // Since previous passes potentially create dead groups
                CollapseControl,
                "compile",
                "post-opt",
                "lower"
            ]
        );

        // Compilation flow with no optimizations enables
        register_alias!(
            pm,
            "no-opt",
            [
                "validate",
                CompileSync,
                SimplifyWithControl,
                CompileInvoke,
                "compile",
                "lower"
            ]
        );

        register_alias!(
            pm,
            "external",
            [
                "validate",
                SynthesisPapercut,
                "pre-opt",
                "compile",
                "post-opt",
                "lower",
                Externalize,
            ]
        );

        register_alias!(pm, "none", []);

        Ok(pm)
    }
}
