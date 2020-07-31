mod cmdline;
mod pass_manager;

use atty::Stream;
use calyx::{
    errors::{Error, Result},
    frontend::{library_syntax, syntax},
    lang::context::Context,
    passes,
    utils::NameGenerator,
};
use cmdline::Opts;
use pass_manager::PassManager;
use passes::{
    compile_control::CompileControl,
    component_interface::ComponentInterface,
    go_insertion::GoInsertion,
    inliner::Inliner,
    merge_assign::MergeAssign,
    static_timing::StaticTiming,
    externalize::Externalize,
    visitor::{Named, Visitor},
};
use std::io::stdin;
use structopt::StructOpt;

/// Construct the pass manager by registering all passes and aliases used
/// by the command line.
fn construct_pass_manager() -> Result<PassManager> {
    // Construct the pass manager and register all passes.
    let mut pm = PassManager::new();

    // Register passes.
    register_pass!(pm, StaticTiming);
    register_pass!(pm, CompileControl);
    register_pass!(pm, GoInsertion);
    register_pass!(pm, ComponentInterface);
    register_pass!(pm, Inliner);
    register_pass!(pm, MergeAssign);
    register_pass!(pm, Externalize);

    // Register aliases
    register_alias!(
        pm,
        "all",
        [
            StaticTiming,
            CompileControl,
            GoInsertion,
            ComponentInterface,
            Inliner,
            MergeAssign
        ]
    );

    register_alias!(
        pm,
        "no-inline",
        [
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
            StaticTiming,
            CompileControl,
            GoInsertion,
            ComponentInterface,
            Inliner,
            MergeAssign,
            Externalize,
        ]
    );

    Ok(pm)
}

fn main() -> Result<()> {
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
        Some(file) => syntax::FutilParser::parse_file(&file),
        None => {
            if atty::isnt(Stream::Stdin) {
                syntax::FutilParser::parse(stdin())
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
            library_syntax::LibraryParser::parse_file(&opts.lib_path.join(path))
        })
        .collect::<Result<Vec<_>>>()?;
    // build context
    let context = Context::from_ast(
        namespace,
        &libraries,
        opts.enable_debug,
        opts.enable_verilator,
    )?;

    // Construct the name generator
    let name_gen = NameGenerator::default();

    // run all passes specified by the command line
    let context =
        pm.execute_plan(context, name_gen, &opts.pass, &opts.disable_pass)?;

    Ok(opts.run_backend(&context, &mut std::io::stdout())?)
}
