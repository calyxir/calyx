use calyx::cmdline::Opts;
use calyx::errors;
use calyx::lang::context;
use calyx::{backend, backend::traits::Backend};
use calyx::{passes, passes::visitor::Visitor};
use structopt::StructOpt;

fn main() -> Result<(), errors::Error> {
    // parse the command line arguments into Opts struct
    let opts: Opts = Opts::from_args();

    let context = context::Context::from_opts(&opts)?;
    passes::lat_insensitive::LatencyInsenstive::do_pass_default(&context)?;
    passes::redundant_par::RedundantPar::do_pass_default(&context)?;
    passes::remove_if::RemoveIf::do_pass_default(&context)?;
    passes::collapse_seq::CollapseSeq::do_pass_default(&context)?;

    opts.backend.run(&context)?;

    Ok(())
}
