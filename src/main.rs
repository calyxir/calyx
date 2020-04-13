use calyx::{
    cmdline::Opts,
    errors,
    lang::context::Context,
    passes,
    passes::visitor::{Named, Visitor},
};
use std::collections::HashMap;
use structopt::StructOpt;

type PassResult = Result<Box<dyn Visitor>, errors::Error>;

fn pass_map() -> HashMap<String, Box<dyn Fn(&Context) -> PassResult>> {
    use passes::{
        automatic_par::AutomaticPar, collapse_seq::CollapseSeq,
        control_id::ControlId, lat_insensitive::LatencyInsensitive,
        redundant_par::RedundantPar, remove_if::RemoveIf,
        remove_par::RemovePar,
    };

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
        AutomaticPar::name().to_string(),
        Box::new(|ctx| {
            let r = AutomaticPar::do_pass_default(ctx)?;
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
        RemovePar::name().to_string(),
        Box::new(|ctx| {
            let mut remove_par = RemovePar::default();
            remove_par.do_pass(ctx)?;
            let mut control_id = ControlId::new(remove_par);
            control_id.do_pass(ctx)?;
            Ok(Box::new(control_id))
        }),
    );
    //println!(" {:#?}", ctx);
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

    // Construct pass manager.
    let names = pass_map();

    //list all the avaliable pass options when flag -listpasses is enabled
    if opts.list_passes {
        let mut passes = names.keys().cloned().collect::<Vec<_>>();
        passes.sort();
        for key in passes {
            println!("- {}", key);
        }
        return Ok(());
    }

    // Construct the context.
    let context = Context::from_opts(&opts)?;

    //run all passes specified by the command line
    for name in opts.pass {
        if let Some(pass) = names.get(&name) {
            pass(&context)?;
        } else {
            let known_passes: String =
                names.keys().cloned().collect::<Vec<_>>().join(", ");
            return Err(errors::Error::UnknownPass(name, known_passes));
        }
    }
    Ok(opts.backend.run(&context, std::io::stdout())?)
}
