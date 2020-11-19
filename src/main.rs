mod cmdline;
mod pass_manager;

use crate::ir::traversal::Visitor;
use atty::Stream;
use calyx::{
    errors::{Error, FutilResult},
    frontend::{library, parser},
    ir,
    ir::traversal::Named,
    passes,
};
use cmdline::Opts;
use pass_manager::PassManager;
use passes::{
    ClkInsertion, CollapseControl, CompileControl, CompileEmpty,
    ComponentInterface, DeadCellRemoval, Externalize, GoInsertion, Inliner,
    LiveRangeAnalysis, MinimizeRegs, Papercut, RemoveExternalMemories,
    ResourceSharing, StaticTiming, WellFormed, IfElseSpec
};
use std::io::stdin;
use structopt::StructOpt;

/// Construct the pass manager by registering all passes and aliases used
/// by the command line.
fn construct_pass_manager() -> FutilResult<PassManager> {
    // Construct the pass manager and register all passes.
    let mut pm = PassManager::new();

    // Register passes.
    register_pass!(pm, WellFormed);
    register_pass!(pm, IfElseSpec);
    register_pass!(pm, StaticTiming);
    register_pass!(pm, CompileControl);
    register_pass!(pm, GoInsertion);
    register_pass!(pm, ComponentInterface);
    register_pass!(pm, Inliner);
    //register_pass!(pm, MergeAssign);
    register_pass!(pm, Externalize);
    register_pass!(pm, RemoveExternalMemories);
    register_pass!(pm, CollapseControl);
    register_pass!(pm, CompileEmpty);
    register_pass!(pm, Papercut);
    register_pass!(pm, ClkInsertion);
    register_pass!(pm, ResourceSharing);
    register_pass!(pm, DeadCellRemoval);

    // custom register pass
    let register_removal_pass_f: pass_manager::PassClosure = Box::new(|ctx| {
        let analysis = LiveRangeAnalysis::do_pass_default(ctx)?;
        MinimizeRegs::new(analysis).do_pass(ctx)?;
        Ok(())
    });
    pm.add_pass(MinimizeRegs::name().to_string(), register_removal_pass_f)?;

    // Register aliases
    // TODO: Add resource sharing.
    register_alias!(
        pm,
        "all",
        [
            IfElseSpec,
            WellFormed,
            Papercut,
            RemoveExternalMemories,
            ResourceSharing,
            MinimizeRegs,
            CompileEmpty,
            CollapseControl,
            StaticTiming,
            CompileControl,
            DeadCellRemoval,
            GoInsertion,
            ComponentInterface,
            Inliner,
            ClkInsertion,
            //MergeAssign,
        ]
    );

    register_alias!(
        pm,
        "external",
        [
            WellFormed,
            Papercut,
            ResourceSharing,
            MinimizeRegs,
            CompileEmpty,
            CollapseControl,
            CompileControl,
            StaticTiming,
            CompileControl,
            DeadCellRemoval,
            GoInsertion,
            ComponentInterface,
            Inliner,
            ClkInsertion,
            //MergeAssign,
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

    // parse libraries
    let libraries: Vec<_> = namespace
        .libraries
        .iter()
        .map(|path| {
            library::parser::LibraryParser::parse_file(
                &opts.lib_path.join(path),
            )
        })
        .collect::<FutilResult<Vec<_>>>()?;

    // Build the IR representation
    let mut rep: ir::Context = ir::from_ast::ast_to_ir(
        namespace.components,
        &libraries,
        namespace.libraries,
        opts.enable_debug,
        opts.enable_synthesis,
    )?;

    // Run all passes specified by the command line
    pm.execute_plan(&mut rep, &opts.pass, &opts.disable_pass)?;

    opts.run_backend(&rep)?;
    Ok(())
}
