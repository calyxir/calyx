mod backend;
mod cmdline;

use calyx::{
    errors::{CalyxResult, Error},
    frontend, ir,
    pass_manager::PassManager,
};
use cmdline::{BackendOpt, CompileMode, Opts};
use itertools::Itertools;

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
    let mut ws = frontend::Workspace::construct(&opts.file, &opts.lib_path)?;

    let imports = ws.original_imports.drain(..).collect_vec();
    let bc = ir::BackendConf {
        synthesis_mode: opts.enable_synthesis,
        enable_verification: !opts.disable_verify,
        initialize_inputs: !opts.disable_init,
    };
    // Build the IR representation
    let mut ctx = ir::from_ast::ast_to_ir(ws, bc)?;
    ctx.extra_opts = opts.extra_opts.drain(..).collect();

    // Run all passes specified by the command line
    pm.execute_plan(&mut ctx, &opts.pass, &opts.disable_pass)?;

    if opts.compile_mode == CompileMode::File
        && !matches!(opts.backend, BackendOpt::Calyx | BackendOpt::None)
    {
        return Err(Error::Misc(format!(
            "--compile-mode=file is only valid with -b calyx. `-b {}` requires --compile-mode=project",
            opts.backend.to_string()
        )));
    }

    // Print out the Calyx program after transformation.
    if opts.backend == BackendOpt::Calyx {
        let out = &mut opts.output.get_write();
        if opts.compile_mode == CompileMode::Project {
            for (path, prims) in ctx.lib.externs() {
                ir::IRPrinter::write_extern(
                    (&path, &prims.into_iter().map(|(_, v)| v).collect_vec()),
                    out,
                )?;
            }
        } else {
            // Print out the original imports for this file.
            for import in imports {
                writeln!(out, "import \"{}\";", import)?;
            }
        }
        for comp in &ctx.components {
            ir::IRPrinter::write_component(comp, out)?;
            writeln!(out)?
        }
        Ok(())
    } else {
        opts.run_backend(ctx)
    }
}
