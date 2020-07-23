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
    visitor::{Named, Visitor},
};
use std::io::stdin;
use structopt::StructOpt;

/*fn pass_map() -> HashMap<String, PassClosure> {
    let mut names: HashMap<String, PassClosure> = HashMap::new();
    names.insert(
        StaticTiming::name().to_string(),
        Box::new(|ctx, _| {
            StaticTiming::do_pass_default(&ctx)?;
            Ok(())
        }),
    );
    names.insert(
        CompileControl::name().to_string(),
        Box::new(|ctx, _| {
            CompileControl::do_pass_default(&ctx)?;
            Ok(())
        }),
    );
    names.insert(
        GoInsertion::name().to_string(),
        Box::new(|ctx, _| {
            GoInsertion::do_pass_default(&ctx)?;
            Ok(())
        }),
    );
    names.insert(
        ComponentInterface::name().to_string(),
        Box::new(|ctx, _| {
            ComponentInterface::do_pass_default(&ctx)?;
            Ok(())
        }),
    );
    names.insert(
        Inliner::name().to_string(),
        Box::new(|ctx, _| {
            Inliner::do_pass_default(&ctx)?;
            Ok(())
        }),
    );
    names.insert(
        MergeAssign::name().to_string(),
        Box::new(|ctx, _| {
            MergeAssign::do_pass_default(&ctx)?;
            Ok(())
        }),
    );
    names.insert(
        "all".to_string(),
        Box::new(|ctx, _name_gen| {
            StaticTiming::do_pass_default(ctx)?;
            CompileControl::do_pass_default(ctx)?;
            GoInsertion::do_pass_default(ctx)?;
            ComponentInterface::do_pass_default(ctx)?;
            Inliner::do_pass_default(ctx)?;
            MergeAssign::do_pass_default(ctx)?;
            Ok(())
        }),
    );
    names.insert(
        "no-inline".to_string(),
        Box::new(|ctx, _name_gen| {
            StaticTiming::do_pass_default(ctx)?;
            CompileControl::do_pass_default(ctx)?;
            GoInsertion::do_pass_default(ctx)?;
            ComponentInterface::do_pass_default(ctx)?;
            Ok(())
        }),
    );
    names
}*/

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
                Err(Error::InvalidFile)
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
    let context = Context::from_ast(namespace, &libraries, opts.enable_debug)?;

    // Construct the name generator
    let name_gen = NameGenerator::default();

    // run all passes specified by the command line
    println!("{:?}", opts.pass);
    let context = pm.execute_plan(context, name_gen, &opts.pass, &vec![])?;

    Ok(opts.run_backend(&context, &mut std::io::stdout())?)
}
