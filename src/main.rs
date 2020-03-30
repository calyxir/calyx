use calyx::{
    cmdline::Opts, errors, lang::context, passes, passes::visitor::Visitor,
    utils::NameGenerator,
};
use structopt::StructOpt;

fn main() -> Result<(), errors::Error> {
    // parse the command line arguments into Opts struct
    let opts: Opts = Opts::from_args();

    let mut names: NameGenerator = NameGenerator::default();

    let context = context::Context::from_opts(&opts)?;

    // optimizations
    passes::redundant_par::RedundantPar::do_pass_default(&context)?;
    passes::remove_if::RemoveIf::do_pass_default(&context)?;
    passes::collapse_seq::CollapseSeq::do_pass_default(&context)?;

    // fsm generation
    passes::fsm_seq::FsmSeq::new(&mut names).do_pass(&context)?;

    // interfacing generation
    passes::lat_insensitive::LatencyInsenstive::do_pass_default(&context)?;
    passes::connect_clock::ConnectClock::do_pass_default(&context)?;

    opts.backend.run(&context, std::io::stdout())?;

    Ok(())
}
