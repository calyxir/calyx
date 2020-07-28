mod cmdline;

use atty::Stream;
use calyx::{
    errors::{Error, Result},
    frontend::{library_syntax, syntax},
    lang::context::Context,
    passes,
    utils::NameGenerator,
};
use cmdline::Opts;
use passes::{
    compile_control::CompileControl,
    component_interface::ComponentInterface,
    go_insertion::GoInsertion,
    inliner::Inliner,
    merge_assign::MergeAssign,
    visitor::{Named, Visitor},
};
use std::collections::HashMap;
use std::io::stdin;
use structopt::StructOpt;

type PassClosure = Box<dyn Fn(&Context, &mut NameGenerator) -> Result<()>>;

fn pass_map() -> HashMap<String, PassClosure> {
    let mut names: HashMap<String, PassClosure> = HashMap::new();
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
            CompileControl::do_pass_default(ctx)?;
            GoInsertion::do_pass_default(ctx)?;
            ComponentInterface::do_pass_default(ctx)?;
            Ok(())
        }),
    );
    names
}

fn main() -> Result<()> {
    // parse the command line arguments into Opts struct
    let opts: Opts = Opts::from_args();

    // list all the avaliable pass options when flag --list-passes is enabled
    if opts.list_passes {
        let names = pass_map();
        let mut passes = names.keys().collect::<Vec<_>>();
        passes.sort();
        for key in passes {
            println!("- {}", key);
        }
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
    let context = Context::from_ast(namespace, &libraries, opts.enable_debug, opts.enable_verilator)?;

    // Construct pass manager.
    let names = pass_map();

    // Construct the name generator
    let mut name_gen = NameGenerator::default();

    // run all passes specified by the command line
    for name in &opts.pass {
        if let Some(pass) = names.get(name) {
            pass(&context, &mut name_gen)?;
        } else {
            // construct known passes for error message
            let known_passes: String =
                names.keys().cloned().collect::<Vec<_>>().join(", ");
            return Err(Error::UnknownPass(name.to_string(), known_passes));
        }
    }
    Ok(opts.run_backend(&context, &mut std::io::stdout())?)
}
