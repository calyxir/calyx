//! Defines the default passes available to [PassManager].
use crate::passes::{
    AddGuard, Canonicalize, CellShare, ClkInsertion, CollapseControl, CombProp,
    CompileInvoke, CompileRepeat, CompileStatic, CompileSync,
    CompileSyncWithoutSyncReg, ComponentInliner, DataPathInfer,
    DeadAssignmentRemoval, DeadCellRemoval, DeadGroupRemoval, DefaultAssigns,
    DiscoverExternal, ExternalToRef, Externalize, GoInsertion, GroupToInvoke,
    GroupToSeq, HoleInliner, InferShare, LowerGuards, MergeAssign, Papercut,
    ParToSeq, RegisterUnsharing, RemoveIds, ResetInsertion,
    SimplifyStaticGuards, SimplifyWithControl, StaticFSMOpts, StaticInference,
    StaticInliner, StaticPromotion, SynthesisPapercut, TopDownCompileControl,
    UnrollBounded, WellFormed, WireInliner, WrapMain,
};
use crate::traversal::Named;
use crate::{pass_manager::PassManager, register_alias};
use calyx_utils::CalyxResult;

impl PassManager {
    pub fn default_passes() -> CalyxResult<Self> {
        // Construct the pass manager and register all passes.
        let mut pm = PassManager::default();

        // Validation passes
        pm.register_pass::<WellFormed>()?;
        pm.register_pass::<Papercut>()?;
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
        pm.register_pass::<StaticFSMOpts>()?;
        pm.register_pass::<CompileStatic>()?;
        pm.register_pass::<CompileInvoke>()?;
        pm.register_pass::<CompileRepeat>()?;
        pm.register_pass::<SimplifyWithControl>()?;
        pm.register_pass::<TopDownCompileControl>()?;
        pm.register_pass::<CompileSync>()?;
        pm.register_pass::<CompileSyncWithoutSyncReg>()?;
        pm.register_pass::<AddGuard>()?;

        // Lowering passes
        pm.register_pass::<GoInsertion>()?;
        pm.register_pass::<WireInliner>()?;
        pm.register_pass::<ClkInsertion>()?;
        pm.register_pass::<ResetInsertion>()?;
        pm.register_pass::<MergeAssign>()?;
        pm.register_pass::<WrapMain>()?;
        pm.register_pass::<DefaultAssigns>()?;

        // Enabled in the synthesis compilation flow
        pm.register_pass::<SynthesisPapercut>()?;
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

        register_alias!(pm, "validate", [WellFormed, Papercut, Canonicalize]);
        register_alias!(
            pm,
            "pre-opt",
            [
                DataPathInfer,
                CollapseControl, // Run it twice: once at beginning of pre-opt, once at end.
                CompileSyncWithoutSyncReg,
                GroupToSeq,
                DeadAssignmentRemoval,
                GroupToInvoke, // Creates Dead Groups potentially
                InferShare,
                ComponentInliner,
                CombProp,
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
            "compile",
            [
                StaticInliner,
                MergeAssign, // Static inliner generates lots of assigns
                DeadGroupRemoval, // Static inliner generates lots of dead groups
                SimplifyStaticGuards,
                AddGuard,
                StaticFSMOpts,
                CompileStatic,
                DeadGroupRemoval,
                TopDownCompileControl
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
