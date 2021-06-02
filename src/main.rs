mod cmdline;

use calyx::{errors::FutilResult, frontend, ir, pass_manager::PassManager};
use cmdline::Opts;
use structopt::StructOpt;

fn main() -> FutilResult<()> {
    let pm = PassManager::default_passes()?;

    // parse the command line arguments into Opts struct
    let opts: Opts = Opts::from_args();

    // list all the avaliable pass options when flag --list-passes is enabled
    if opts.list_passes {
        println!("{}", pm.show_names());
        return Ok(());
    }

    // Construct the namespace.
    let namespace = frontend::NamespaceDef::new(&opts.file, &opts.lib_path)?;

    // Build the IR representation
    let mut rep = ir::from_ast::ast_to_ir(
        namespace,
        opts.enable_debug,
        opts.enable_synthesis,
    )?;

    // Run all passes specified by the command line
    pm.execute_plan(&mut rep, &opts.pass, &opts.disable_pass)?;

    opts.run_backend(&rep)?;
    Ok(())
}
