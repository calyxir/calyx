use calyx::{
    cmdline::Opts, errors, lang::context, passes, passes::visitor::Visitor,
};
use passes::collapse_seq::CollapseSeq;
use passes::lat_insensitive::LatencyInsenstive;
use passes::redundant_par::RedundantPar;
use passes::remove_if::RemoveIf;
use passes::visitor::Named;
use std::collections::HashMap;
use structopt::StructOpt;

fn main() -> Result<(), errors::Error> {
    // parse the command line arguments into Opts struct
    let opts: Opts = Opts::from_args();
    let context = context::Context::from_opts(&opts)?;
    let mut names: HashMap<
        String,
        Box<dyn Fn() -> Result<Box<dyn Visitor>, errors::Error>>,
    > = HashMap::new();
    names.insert(
        LatencyInsenstive::name().to_string(),
        Box::new(|| {
            let r = LatencyInsenstive::do_pass_default(&context)?;
            Ok(Box::new(r))
        }),
    );
    names.insert(
        CollapseSeq::name().to_string(),
        Box::new(|| {
            let r = CollapseSeq::do_pass_default(&context)?;
            Ok(Box::new(r))
        }),
    );
    names.insert(
        RemoveIf::name().to_string(),
        Box::new(|| {
            let r = RemoveIf::do_pass_default(&context)?;
            Ok(Box::new(r))
        }),
    );
    names.insert(
        RedundantPar::name().to_string(),
        Box::new(|| {
            let r = RedundantPar::do_pass_default(&context)?;
            Ok(Box::new(r))
        }),
    );
    names.insert(
        "all".to_string(),
        Box::new(|| {
            passes::lat_insensitive::LatencyInsenstive::do_pass_default(
                &context,
            )?;
            passes::redundant_par::RedundantPar::do_pass_default(&context)?;
            passes::remove_if::RemoveIf::do_pass_default(&context)?;
            let r =
                passes::collapse_seq::CollapseSeq::do_pass_default(&context)?;
            Ok(Box::new(r))
        }),
    );
    //list all the avaliable pass options when flag -listpasses is enabled
    if opts.listpasses {
        for key in names.keys() {
            println!("- {}", key);
        }
        return Ok(());
    }
    //run all passes specified by the command line
    for name in opts.pass {
        if let Some(pass) = names.get(&name) {
            pass()?;
        }
    }
    opts.backend.run(&context, std::io::stdout())?;
    Ok(())
}
