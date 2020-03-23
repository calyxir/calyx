mod backend;
mod cmdline;
mod errors;
mod lang;
mod passes;
mod utils;

use crate::cmdline::Opts;
use crate::lang::context;
use crate::passes::visitor::Visitor;
use structopt::StructOpt;

fn main() -> Result<(), errors::Error> {
    // better stack traces
    better_panic::install();

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
