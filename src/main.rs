use calyx::{
    cmdline::Opts,
    errors,
    lang::context::Context,
    passes,
    passes::visitor::{Named, Visitor},
    utils::NameGenerator,
};
use std::collections::HashMap;
use structopt::StructOpt;

type PassClosure =
    Box<dyn Fn(&Context, &mut NameGenerator) -> Result<(), errors::Error>>;

fn pass_map() -> HashMap<String, PassClosure> {
    use passes::{
        automatic_par::AutomaticPar, collapse_seq::CollapseSeq,
        connect_clock::ConnectClock, fsm_seq::FsmSeq,
        lat_insensitive::LatencyInsensitive, redundant_par::RedundantPar,
        remove_if::RemoveIf,
    };

    let mut names: HashMap<String, PassClosure> = HashMap::new();
    names.insert(
        LatencyInsensitive::name().to_string(),
        Box::new(|ctx, _| {
            LatencyInsensitive::do_pass_default(ctx)?;
            Ok(())
        }),
    );
    names.insert(
        CollapseSeq::name().to_string(),
        Box::new(|ctx, _| {
            CollapseSeq::do_pass_default(ctx)?;
            Ok(())
        }),
    );
    names.insert(
        AutomaticPar::name().to_string(),
        Box::new(|ctx, _| {
            AutomaticPar::do_pass_default(ctx)?;
            Ok(())
        }),
    );
    names.insert(
        RemoveIf::name().to_string(),
        Box::new(|ctx, _| {
            RemoveIf::do_pass_default(ctx)?;
            Ok(())
        }),
    );
    names.insert(
        RedundantPar::name().to_string(),
        Box::new(|ctx, _| {
            RedundantPar::do_pass_default(ctx)?;
            Ok(())
        }),
    );
    names.insert(
        ConnectClock::name().to_string(),
        Box::new(|ctx, _| {
            ConnectClock::do_pass_default(ctx)?;
            Ok(())
        }),
    );
    names.insert(
        FsmSeq::name().to_string(),
        Box::new(|ctx, mut name_gen| {
            FsmSeq::new(&mut name_gen).do_pass(&ctx)?;
            Ok(())
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
        Box::new(|ctx, mut name_gen| {
            RedundantPar::do_pass_default(ctx)?;
            RemoveIf::do_pass_default(ctx)?;
            CollapseSeq::do_pass_default(ctx)?;
            AutomaticPar::do_pass_default(ctx)?;
            // fsm generation
            LatencyInsensitive::do_pass_default(&ctx)?;
            FsmSeq::new(&mut name_gen).do_pass(&ctx)?;

            // interfacing generation
            ConnectClock::do_pass_default(&ctx)?;
            Ok(())
        }),
    );
    names
}

fn main() -> Result<(), errors::Error> {
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

    // Construct the context.
    let context = Context::from_opts(&opts)?;

    // Construct pass manager.
    let names = pass_map();

    // Construct the name generator
    let mut name_gen = NameGenerator::default();

    // run all passes specified by the command line
    for name in opts.pass {
        if let Some(pass) = names.get(&name) {
            pass(&context, &mut name_gen)?;
        } else {
            let known_passes: String =
                names.keys().cloned().collect::<Vec<_>>().join(", ");
            return Err(errors::Error::UnknownPass(name, known_passes));
        }
    }
    Ok(opts.backend.run(&context, std::io::stdout())?)
}
