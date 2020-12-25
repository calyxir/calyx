mod cmdline;
mod pass_manager;

use crate::ir::traversal::Visitor;
use atty::Stream;
use calyx::{
    errors::{Error, FutilResult},
    frontend::parser,
    ir,
    ir::traversal::Named,
    passes,
};
use cmdline::Opts;
use pass_manager::PassManager;
use passes::{
    ClkInsertion, CollapseControl, CompileControl, CompileEmpty, CompileInvoke,
    ComponentInterface, DeadCellRemoval, Externalize, GoInsertion,
    InferStaticTiming, Inliner, MergeAssign, MinimizeRegs, Papercut,
    RemoveExternalMemories, ResourceSharing, SimplifyGuards, StaticTiming,
    WellFormed,
};
use std::io::stdin;
use std::path::PathBuf;
use structopt::StructOpt;

/// Construct the pass manager by registering all passes and aliases used
/// by the command line.
fn construct_pass_manager() -> FutilResult<PassManager> {
    // Construct the pass manager and register all passes.
    let mut pm = PassManager::new();

    // Register passes.
    register_pass!(pm, WellFormed);
    register_pass!(pm, StaticTiming);
    register_pass!(pm, CompileControl);
    register_pass!(pm, CompileInvoke);
    register_pass!(pm, GoInsertion);
    register_pass!(pm, ComponentInterface);
    register_pass!(pm, Inliner);
    register_pass!(pm, Externalize);
    register_pass!(pm, RemoveExternalMemories);
    register_pass!(pm, CollapseControl);
    register_pass!(pm, CompileEmpty);
    register_pass!(pm, Papercut);
    register_pass!(pm, ClkInsertion);
    register_pass!(pm, ResourceSharing);
    register_pass!(pm, DeadCellRemoval);
    register_pass!(pm, MinimizeRegs);
    register_pass!(pm, InferStaticTiming);
    register_pass!(pm, SimplifyGuards);
    register_pass!(pm, MergeAssign);

    register_alias!(pm, "validate", [WellFormed, Papercut]);
    register_alias!(
        pm,
        "pre-opt",
        [
            CompileInvoke,
            CollapseControl,
            InferStaticTiming,
            ResourceSharing,
            MinimizeRegs
        ]
    );
    register_alias!(
        pm,
        "compile",
        [CompileEmpty, StaticTiming, CompileControl]
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
            MergeAssign,
            SimplifyGuards
        ]
    );

    // Register aliases
    register_alias!(
        pm,
        "all",
        [
            "validate",
            RemoveExternalMemories,
            "pre-opt",
            "compile",
            "post-opt",
            "lower",
        ]
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

fn main() -> FutilResult<()> {
    let pm = construct_pass_manager()?;

    // parse the command line arguments into Opts struct
    let opts: Opts = Opts::from_args();

    // list all the avaliable pass options when flag --list-passes is enabled
    if opts.list_passes {
        println!("{}", pm.show_names());
        return Ok(());
    }

    // ==== Construct the context ====
    // parse the file
    let namespace = match &opts.file {
        Some(file) => parser::FutilParser::parse_file(&file),
        None => {
            if atty::isnt(Stream::Stdin) {
                parser::FutilParser::parse(stdin())
            } else {
                Err(Error::InvalidFile(
                    "No file provided and terminal not a TTY".to_string(),
                ))
            }
        }
    }?;

    // Parse all transitive dependencies
    let mut libs = ir::LibrarySignatures::default();
    namespace.externs.into_iter().for_each(|(path, exts)| {
        libs.extend(path, exts);
    });
    let mut components = namespace.components;
    let mut deps: Vec<PathBuf> = namespace
        .imports
        .into_iter()
        .map(|f| opts.lib_path.join(f))
        .collect();

    while let Some(path) = deps.pop() {
        let mut namespace = parser::FutilParser::parse_file(&path)?;
        components.append(&mut namespace.components);
        namespace.externs.into_iter().for_each(|(path, exts)| {
            libs.extend(path, exts);
        });

        // All imports are relative to the file being currently parsed.
        deps.append(
            &mut namespace
                .imports
                .into_iter()
                .map(|f| match path.parent() {
                    None => PathBuf::from(f),
                    Some(p) => p.join(f),
                })
                .collect(),
        );
    }

    // Build the IR representation
    let mut rep: ir::Context = ir::from_ast::ast_to_ir(
        components,
        libs,
        opts.enable_debug,
        opts.enable_synthesis,
    )?;

    // Run all passes specified by the command line
    pm.execute_plan(&mut rep, &opts.pass, &opts.disable_pass)?;

    opts.run_backend(&rep)?;
    Ok(())
}
