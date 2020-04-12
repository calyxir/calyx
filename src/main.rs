use calyx::{
    cmdline::Opts,
    errors,
    lang::context::Context,
    passes,
    passes::visitor::{Named, Visitor},
    utils::NameGenerator,
};
use passes::{
    automatic_par::AutomaticPar, collapse_seq::CollapseSeq,
    connect_clock::ConnectClock, fsm_seq::FsmSeq,
    lat_insensitive::LatencyInsenstive, redundant_par::RedundantPar,
    remove_if::RemoveIf,
};
use std::collections::HashMap;
use structopt::StructOpt;

type PassResult = Result<Box<dyn Visitor>, errors::Error>;

fn main() -> Result<(), errors::Error> {
    // parse the command line arguments into Opts struct
    let opts: Opts = Opts::from_args();
    let mut names: HashMap<String, Box<dyn Fn(&Context) -> PassResult>> =
        HashMap::new();
    names.insert(
        LatencyInsenstive::name().to_string(),
        Box::new(|ctx| {
            let r = LatencyInsenstive::do_pass_default(ctx)?;
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
        ConnectClock::name().to_string(),
        Box::new(|ctx| {
            let r = ConnectClock::do_pass_default(ctx)?;
            Ok(Box::new(r))
        }),
    );
    // names.insert(
    //     FsmSeq::name().to_string(),
    //     Box::new(move |ctx| {
    //         let r = FsmSeq::new(&mut name_gen);
    //         r.do_pass(ctx)?;
    //         Ok(Box::new(r))
    //     }),
    // );
    names.insert(
        "all".to_string(),
        Box::new(move |ctx| {
            LatencyInsenstive::do_pass_default(ctx)?;
            RedundantPar::do_pass_default(ctx)?;
            RemoveIf::do_pass_default(ctx)?;
            CollapseSeq::do_pass_default(ctx)?;
            AutomaticPar::do_pass_default(ctx)?;
            // fsm generation
            LatencyInsenstive::do_pass_default(&ctx)?;
            let mut name_gen = NameGenerator::default();
            FsmSeq::new(&mut name_gen).do_pass(&ctx)?;

            // interfacing generation
            let r = ConnectClock::do_pass_default(&ctx)?;
            Ok(Box::new(r))
        }),
    );

    // list all the avaliable pass options when flag -listpasses is enabled
    if opts.list_passes {
        for key in names.keys() {
            println!("- {}", key);
        }
        return Ok(());
    }

    let context = Context::from_opts(&opts)?;
    // run all passes specified by the command line
    for name in opts.pass {
        if let Some(pass) = names.get(&name) {
            pass(&context)?;
        }
    }
    opts.backend.run(&context, std::io::stdout())?;
    Ok(())
}
