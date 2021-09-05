//! Defines the default passes available to [PassManager].
use crate::passes::{
    ClkInsertion, CollapseControl, CompileControl, CompileEmpty, CompileInvoke,
    ComponentInterface, DeadCellRemoval, Externalize, GoInsertion,
    GuardCanonical, InferStaticTiming, Inliner, LowerGuards, MergeAssign,
    MinimizeRegs, Papercut, ParToSeq, RegisterUnsharing, RemoveCombGroups,
    ResetInsertion, ResourceSharing, SimplifyGuards, StaticTiming,
    SynthesisPapercut, TopDownCompileControl, WellFormed,
};
use crate::{
    errors::CalyxResult, ir::traversal::Named, pass_manager::PassManager,
    register_alias,
};

impl PassManager {
    pub fn default_passes() -> CalyxResult<Self> {
        // Construct the pass manager and register all passes.
        let mut pm = PassManager::default();

        // Register passes.
        pm.register_pass::<WellFormed>()?;
        pm.register_pass::<StaticTiming>()?;
        pm.register_pass::<CompileControl>()?;
        pm.register_pass::<CompileInvoke>()?;
        pm.register_pass::<GoInsertion>()?;
        pm.register_pass::<ComponentInterface>()?;
        pm.register_pass::<Inliner>()?;
        pm.register_pass::<Externalize>()?;
        pm.register_pass::<CollapseControl>()?;
        pm.register_pass::<CompileEmpty>()?;
        pm.register_pass::<Papercut>()?;
        pm.register_pass::<ClkInsertion>()?;
        pm.register_pass::<ResetInsertion>()?;
        pm.register_pass::<ResourceSharing>()?;
        pm.register_pass::<DeadCellRemoval>()?;
        pm.register_pass::<MinimizeRegs>()?;
        pm.register_pass::<InferStaticTiming>()?;
        pm.register_pass::<SimplifyGuards>()?;
        pm.register_pass::<MergeAssign>()?;
        pm.register_pass::<TopDownCompileControl>()?;
        pm.register_pass::<SynthesisPapercut>()?;
        pm.register_pass::<RegisterUnsharing>()?;
        pm.register_pass::<GuardCanonical>()?;
        pm.register_pass::<LowerGuards>()?;
        pm.register_pass::<ParToSeq>()?;
        pm.register_pass::<RemoveCombGroups>()?;

        register_alias!(pm, "validate", [WellFormed, Papercut, GuardCanonical]);
        register_alias!(
            pm,
            "pre-opt",
            [
                RemoveCombGroups, // Must run before `infer-static-timing`.
                InferStaticTiming,
                CollapseControl,
                ResourceSharing,
                MinimizeRegs,
                CompileInvoke,
            ]
        );
        register_alias!(
            pm,
            "compile",
            [CompileEmpty, StaticTiming, TopDownCompileControl]
        );
        register_alias!(pm, "post-opt", [DeadCellRemoval]);
        register_alias!(
            pm,
            "lower",
            [
                GoInsertion,
                ComponentInterface,
                Inliner,
                ClkInsertion,
                ResetInsertion,
                MergeAssign,
            ]
        );

        // Register aliases
        register_alias!(
            pm,
            "all",
            ["validate", "pre-opt", "compile", "post-opt", "lower",]
        );

        register_alias!(
            pm,
            "external",
            [
                "validate",
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
