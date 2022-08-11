//! Defines the default passes available to [PassManager].
use crate::passes::{
    Canonicalize, CellShare, ClkInsertion, CollapseControl, CombProp,
    CompileEmpty, CompileInvoke, CompileRef, CompileSync, ComponentInliner,
    ComponentInterface, DeadCellRemoval, DeadGroupRemoval, Externalize,
    GoInsertion, GroupToInvoke, GroupToSeq, HoleInliner, InferShare,
    InferStaticTiming, LowerGuards, MergeAssign, MergeStaticPar, Papercut,
    ParToSeq, RegisterUnsharing, RemoveCombGroups, RemoveIds, ResetInsertion,
    SimplifyGuards, StaticParConv, SynthesisPapercut, TopDownCompileControl,
    TopDownStaticTiming, UnrollBounded, WellFormed, WireInliner,
};
use crate::{
    errors::CalyxResult, ir::traversal::Named, pass_manager::PassManager,
    register_alias,
};

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
        pm.register_pass::<CompileEmpty>()?;
        pm.register_pass::<DeadCellRemoval>()?;
        pm.register_pass::<DeadGroupRemoval>()?;
        pm.register_pass::<GroupToSeq>()?;
        pm.register_pass::<InferShare>()?;
        pm.register_pass::<CellShare>()?;
        pm.register_pass::<InferStaticTiming>()?;
        pm.register_pass::<MergeStaticPar>()?;
        pm.register_pass::<StaticParConv>()?;

        // Compilation passes
        pm.register_pass::<CompileInvoke>()?;
        pm.register_pass::<RemoveCombGroups>()?;
        pm.register_pass::<TopDownStaticTiming>()?;
        pm.register_pass::<TopDownCompileControl>()?;
        pm.register_pass::<CompileRef>()?;
        pm.register_pass::<CompileSync>()?;

        // Lowering passes
        pm.register_pass::<GoInsertion>()?;
        pm.register_pass::<ComponentInterface>()?;
        pm.register_pass::<WireInliner>()?;
        pm.register_pass::<ClkInsertion>()?;
        pm.register_pass::<ResetInsertion>()?;
        pm.register_pass::<MergeAssign>()?;

        // Enabled in the synthesis compilation flow
        pm.register_pass::<SynthesisPapercut>()?;
        pm.register_pass::<Externalize>()?;

        // Disabled by default
        pm.register_pass::<UnrollBounded>()?;
        pm.register_pass::<SimplifyGuards>()?;
        pm.register_pass::<RegisterUnsharing>()?;
        pm.register_pass::<GroupToInvoke>()?;
        pm.register_pass::<ParToSeq>()?;
        pm.register_pass::<LowerGuards>()?;
        pm.register_pass::<HoleInliner>()?;
        pm.register_pass::<RemoveIds>()?;

        register_alias!(pm, "validate", [WellFormed, Papercut, Canonicalize]);
        register_alias!(
            pm,
            "pre-opt",
            [
                CompileSync,
                GroupToSeq,
                GroupToInvoke, // Creates Dead Groups potentially
                ComponentInliner,
                CombProp,
                CompileRef, //Must run before cell-share.
                InferShare,
                CellShare, // LiveRangeAnalaysis should handle comb groups
                RemoveCombGroups, // Must run before infer-static-timing
                InferStaticTiming,
                CompileInvoke,    // creates dead comb groups
                MergeStaticPar,   // creates dead groups potentially
                StaticParConv,    // Must be before collapse-control
                DeadGroupRemoval, // Since previous passes potentially create dead groups
                CollapseControl,
            ]
        );
        register_alias!(pm, "compile", [TopDownCompileControl]);
        register_alias!(
            pm,
            "post-opt",
            [DeadGroupRemoval, CombProp, DeadCellRemoval]
        );
        register_alias!(
            pm,
            "lower",
            [
                GoInsertion,
                WireInliner,
                ClkInsertion,
                ResetInsertion,
                MergeAssign,
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
            ["validate", RemoveCombGroups, "compile", "lower"]
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
