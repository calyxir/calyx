//! # The Calyx Compiler
//!
//! This crate plumbs together the Calyx compiler crates and provides a command-line interface for the Calyx compiler.
//! What `clang` it to `llvm`, this crate is to the Calyx IL.
//! You SHOULD NOT depend on this crate since does things like installing the primitives library in a global location.
//! Instead, depend on the crates that this crate depends: [`calyx_frontend`], [`calyx_ir`], [`calyx_opt`].

mod cmdline;
use calyx_backend::BackendOpt;
use calyx_frontend as frontend;
use calyx_ir as ir;
use calyx_opt::pass_manager::{PassManager, PassResult};
use cmdline::{CompileMode, Opts};

fn main() -> PassResult<()> {
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

    // handle pass-help and format
    if let Some(sub) = opts.sub {
        match sub {
            cmdline::Subcommand::Help(cmdline::Help { name }) => {
                match name {
                    Some(n) => {
                        if let Some(help) = pm.specific_help(&n) {
                            println!("{help}");
                        } else {
                            println!("Unknown pass or alias: {n}");
                        }
                    }
                    None => println!("{}", pm.complete_help()),
                }
                return Ok(());
            }
            cmdline::Subcommand::Format(cmdline::Format { file }) => {
                let mut ws = frontend::Workspace::construct(
                    &Some(file.clone()),
                    &opts.lib_path,
                )?;
                let imports = std::mem::take(&mut ws.original_imports);
                // Build the IR representation
                let ctx = ir::from_ast::ast_to_ir(
                    ws,
                    ir::from_ast::AstConversionConfig {
                        extend_signatures: false,
                    },
                )?;
                let out = &mut opts.output.get_write();

                // Print out the original imports for this file.
                for import in imports {
                    writeln!(out, "import \"{import}\";")?;
                }
                ir::Printer::write_context(&ctx, true, out)?;
                return Ok(());
            }
        }
    }

    // Construct the namespace.
    let mut ws = frontend::Workspace::construct(&opts.file, &opts.lib_path)?;

    let imports = std::mem::take(&mut ws.original_imports);

    // Build the IR representation
    let mut ctx = ir::from_ast::ast_to_ir(
        ws,
        ir::from_ast::AstConversionConfig::default(),
    )?;
    // Configuration for the backend
    ctx.bc = ir::BackendConf {
        synthesis_mode: opts.enable_synthesis,
        enable_verification: !opts.disable_verify,
        flat_assign: !opts.nested_assign,
        emit_primitive_extmodules: opts.emit_primitive_extmodules,
    };
    // Extra options for the passes
    ctx.extra_opts = opts.extra_opts.drain(..).collect();

    // Run all passes specified by the command line
    pm.execute_plan(
        &mut ctx,
        &opts.pass,
        &opts.disable_pass,
        &opts.insertions,
        opts.dump_ir,
    )?;

    // Print out the Calyx program after transformation.
    if opts.backend == BackendOpt::Calyx {
        let out = &mut opts.output.get_write();

        // Print out the original imports for this file.
        if opts.compile_mode == CompileMode::File {
            for import in imports {
                writeln!(out, "import \"{import}\";")?;
            }
        }
        ir::Printer::write_context(
            &ctx,
            opts.compile_mode == CompileMode::File,
            out,
        )?;

        Ok(())
    } else {
        Ok(opts.run_backend(ctx)?)
    }
}
