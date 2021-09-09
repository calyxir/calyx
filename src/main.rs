mod backend;
mod cmdline;

use calyx::{errors::CalyxResult, frontend, ir, pass_manager::PassManager};
use cmdline::Opts;

fn main() -> CalyxResult<()> {
    let pm = PassManager::default_passes()?;

    // parse the command line arguments into Opts struct
    let mut opts = Opts::get_opts();

    // list all the avaliable pass options when flag --list-passes is enabled
    if opts.list_passes {
        println!("{}", pm.show_names());
        return Ok(());
    }

    // Construct the namespace.
    let namespace = frontend::NamespaceDef::new(&opts.file, &opts.lib_path)?;

    // Build the IR representation
    let mut ctx = ir::from_ast::ast_to_ir(
        namespace,
        opts.enable_synthesis,
        !opts.disable_verify,
    )?;
    ctx.extra_opts = opts.extra_opts.drain(..).collect();

    // Run all passes specified by the command line
    pm.execute_plan(&mut ctx, &opts.pass, &opts.disable_pass)?;

    opts.run_backend(&ctx)?;
    Ok(())
}
