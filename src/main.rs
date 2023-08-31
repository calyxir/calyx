use calyx::cmdline::{CompileMode, Opts};
use calyx_backend::BackendOpt;
use calyx_frontend as frontend;
use calyx_ir as ir;
use calyx_opt::pass_manager::PassManager;
use calyx_utils::CalyxResult;
use itertools::Itertools;

fn main() -> CalyxResult<()> {
    // parse the command line arguments into Opts struct
    let mut opts = Opts::get_opts()?;

    // Return the version and the git commit this was built on
    if opts.version {
        println!("Calyx compiler version {}", env!("CARGO_PKG_VERSION"));
        println!(
            "Library location: {}",
            option_env!("CALYX_PRIMITIVES_DIR").unwrap_or(".")
        );
        return Ok(());
    }

    // enable tracing
    env_logger::Builder::new()
        .format_timestamp(None)
        .filter_level(opts.log_level)
        .target(env_logger::Target::Stderr)
        .init();

    let pm = PassManager::default_passes()?;

    // list all the avaliable pass options when flag --list-passes is enabled
    if opts.list_passes {
        println!("{}", pm.show_names());
        return Ok(());
    }

    // Construct the namespace.
    let mut ws = frontend::Workspace::construct(&opts.file, &opts.lib_path)?;

    let imports = ws.original_imports.drain(..).collect_vec();

    // Build the IR representation
    let mut ctx = ir::from_ast::ast_to_ir(ws)?;
    // Configuration for the backend
    ctx.bc = ir::BackendConf {
        synthesis_mode: opts.enable_synthesis,
        enable_verification: !opts.disable_verify,
        flat_assign: !opts.nested_assign,
    };
    // Extra options for the passes
    ctx.extra_opts = opts.extra_opts.drain(..).collect();

    // Run all passes specified by the command line
    pm.execute_plan(&mut ctx, &opts.pass, &opts.disable_pass, opts.dump_ir)?;

    // Print out the Calyx program after transformation.
    if opts.backend == BackendOpt::Calyx {
        let out = &mut opts.output.get_write();

        // Print out the original imports for this file.
        if opts.compile_mode == CompileMode::File {
            for import in imports {
                writeln!(out, "import \"{}\";", import)?;
            }
        }
        ir::Printer::write_context(
            &ctx,
            opts.compile_mode == CompileMode::File,
            out,
        )?;

        Ok(())
    } else {
        opts.run_backend(ctx)
    }
}
