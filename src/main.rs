mod cmdline;
//mod pass_manager;

use crate::ir::traversal::Visitor;
use atty::Stream;
use calyx::{
    errors::{Error, FutilResult},
    frontend::{library, parser},
    ir,
    //lang::context::Context,
    passes,
    //passes,
    //utils::NameGenerator,
};
use cmdline::Opts;
use passes::{CompileEmpty, Inliner /* WellFormed */};
//use pass_manager::PassManager;
/*use passes::{
    collapse_control::CollapseControl,
    compile_control::CompileControl,
    compile_empty::CompileEmpty,
    component_interface::ComponentInterface,
    externalize::Externalize,
    go_insertion::GoInsertion,
    inliner::Inliner,
    merge_assign::MergeAssign,
    papercut::Papercut,
    remove_external_memories::RemoveExternalMemories,
    static_timing::StaticTiming,
    visitor::{Named, Visitor},
    well_formed::WellFormed,
};*/
use std::io::stdin;
use structopt::StructOpt;

/// Construct the pass manager by registering all passes and aliases used
/// by the command line.
/*fn construct_pass_manager() -> FutilResult<PassManager> {
    // Construct the pass manager and register all passes.
    let mut pm = PassManager::new();

    // Register passes.
    register_pass!(pm, WellFormed);
    register_pass!(pm, StaticTiming);
    register_pass!(pm, CompileControl);
    register_pass!(pm, GoInsertion);
    register_pass!(pm, ComponentInterface);
    register_pass!(pm, Inliner);
    register_pass!(pm, MergeAssign);
    register_pass!(pm, Externalize);
    register_pass!(pm, RemoveExternalMemories);
    register_pass!(pm, CollapseControl);
    register_pass!(pm, CompileEmpty);
    register_pass!(pm, Papercut);

    // Register aliases
    register_alias!(
        pm,
        "all",
        [
            WellFormed,
            Papercut,
            RemoveExternalMemories,
            CompileEmpty,
            CollapseControl,
            StaticTiming,
            CompileControl,
            GoInsertion,
            ComponentInterface,
            Inliner,
            MergeAssign,
        ]
    );

    register_alias!(
        pm,
        "no-inline",
        [
            RemoveExternalMemories,
            CompileEmpty,
            StaticTiming,
            CompileControl,
            GoInsertion,
            ComponentInterface,
        ]
    );

    register_alias!(
        pm,
        "external",
        [
            WellFormed,
            Papercut,
            CompileEmpty,
            CollapseControl,
            CompileControl,
            StaticTiming,
            CompileControl,
            GoInsertion,
            ComponentInterface,
            Inliner,
            MergeAssign,
            Externalize,
        ]
    );

    register_alias!(pm, "none", []);

    Ok(pm)
}*/

fn main() -> FutilResult<()> {
    //let pm = construct_pass_manager()?;

    // parse the command line arguments into Opts struct
    let opts: Opts = Opts::from_args();

    // list all the avaliable pass options when flag --list-passes is enabled
    /*if opts.list_passes {
        println!("{}", pm.show_names());
        return Ok(());
    }*/

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

    // build context
    /*let context = Context::from_ast(
        namespace,
        &libraries,
        opts.enable_debug,
        opts.enable_verilator,
        opts.color,
    )?;*/

    // Build the IR representation
    let mut rep: ir::Context = ir::from_ast::ast_to_ir(
        namespace.components,
        &libraries,
        opts.enable_debug,
    )?;

    // WellFormed::do_pass_default(&mut rep)?;
    Inliner::do_pass_default(&mut rep)?;
    CompileEmpty::do_pass_default(&mut rep)?;

    // // Construct the name generator
    // let name_gen = NameGenerator::default();

    // // run all passes specified by the command line
    // let context =
    //     pm.execute_plan(context, name_gen, &opts.pass, &opts.disable_pass)?;

    opts.run_backend(&rep)?;
    Ok(())
}
