use calyx::{
    cmdline::Opts,
    errors,
    lang::context::Context,
    passes,
    passes::visitor::{Named, Visitor},
};
use passes::{
    automatic_par::AutomaticPar, collapse_seq::CollapseSeq,
    lat_insensitive::LatencyInsensitive, redundant_par::RedundantPar,
    remove_if::RemoveIf,
};
use std::collections::HashMap;
use structopt::StructOpt;

type PassResult = Result<Box<dyn Visitor>, errors::Error>;

fn pass_map() -> HashMap<String, Box<dyn Fn(&Context) -> PassResult>> {
    let mut names: HashMap<String, Box<dyn Fn(&Context) -> PassResult>> =
        HashMap::new();
    names.insert(
        LatencyInsensitive::name().to_string(),
        Box::new(|ctx| {
            let r = LatencyInsensitive::do_pass_default(ctx)?;
            Ok(Box::new(r))
        }),
    );
    names.insert(
        CollapseSeq::name().to_string(),
        Box::new(|ctx| {
            let r = CollapseSeq::do_pass_default(ctx)?;
            Ok(Box::new(r))
        }),
    );
    names.insert(
        RemoveIf::name().to_string(),
        Box::new(|ctx| {
            let r = RemoveIf::do_pass_default(ctx)?;
            Ok(Box::new(r))
        }),
    );
    names.insert(
        RedundantPar::name().to_string(),
        Box::new(|ctx| {
            let r = RedundantPar::do_pass_default(ctx)?;
            Ok(Box::new(r))
        }),
    );
    names.insert(
        AutomaticPar::name().to_string(),
        Box::new(|ctx| {
            let r = AutomaticPar::do_pass_default(ctx)?;
            Ok(Box::new(r))
        }),
    );
    names.insert(
        "all".to_string(),
        Box::new(|ctx| {
            LatencyInsensitive::do_pass_default(ctx)?;
            RedundantPar::do_pass_default(ctx)?;
            RemoveIf::do_pass_default(ctx)?;
            CollapseSeq::do_pass_default(ctx)?;
            let r = AutomaticPar::do_pass_default(ctx)?;
            Ok(Box::new(r))
        }),
    );
    names
}

fn main() -> Result<(), errors::Error> {
    // parse the command line arguments into Opts struct
    let opts: Opts = Opts::from_args();
    let context = Context::from_opts(&opts)?;

    // Construct pass manager.
    let names = pass_map();

    //list all the avaliable pass options when flag -listpasses is enabled
    if opts.list_passes {
        for key in names.keys() {
            println!("- {}", key);
        }
        return Ok(());
    }

    //run all passes specified by the command line
    for name in opts.pass {
        if let Some(pass) = names.get(&name) {
            pass(&context)?;
        } else {
            let known_passes: String = names
                .keys()
                .into_iter()
                .map(|p| p.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(errors::Error::UnknownPass(name, known_passes));
        }
    }
    opts.backend.run(&context, std::io::stdout())?;
    Ok(())
}
