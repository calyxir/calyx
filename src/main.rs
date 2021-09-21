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
    let ws = frontend::Workspace::construct(&opts.file, &opts.lib_path)?;

    // Build the IR representation
    let mut ctx = ir::from_ast::ast_to_ir(
        ws,
        opts.enable_synthesis,
        !opts.disable_verify,
    )?;
    ctx.extra_opts = opts.extra_opts.drain(..).collect();

    // Run all passes specified by the command line
    pm.execute_plan(&mut ctx, &opts.pass, &opts.disable_pass)?;

    if opts.compile_mode == CompileMode::File
        && opts.backend != BackendOpt::Calyx
    {
        return Err(Error::Unsupported(format!("--compile-mode=file is only valid with -b calyx. `-b {}` requires --compile-mode=project", self.backend)));
    }

    if opts.backend == BackendOpt::Calyx {
        if opts.compile_mode == CompileMode::Project {
            for (path, prims) in context.lib.externs() {
                ir::IRPrinter::write_extern(
                    (&path, &prims.into_iter().map(|(_, v)| v).collect_vec()),
                    &mut self.output.get_write(),
                )?;
            }
        } else {
            todo!()
        }
        for comp in &context.components {
            ir::IRPrinter::write_component(comp, &mut self.output.get_write())?;
            writeln!(&mut self.output.get_write())?
        }
        Ok(())
    } else {
        opts.run_backend(ctx)
    }
}
