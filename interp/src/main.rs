use calyx::{
    errors::{Error, FutilResult},
    frontend, ir,
    pass_manager::PassManager,
    utils::OutputFile,
};
use interp::environment;
use interp::interpreter::interpret_component;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use structopt::StructOpt;

/// CLI Options
#[derive(Debug, StructOpt)]
#[structopt(name = "interpreter", about = "interpreter CLI")]
pub struct Opts {
    /// Input file
    #[structopt(parse(from_os_str))]
    pub file: Option<PathBuf>,

    /// Output file, default is stdout
    #[structopt(short = "o", long = "output", default_value)]
    pub output: OutputFile,

    /// Path to the primitives library
    #[structopt(long, short, default_value = "..")]
    pub lib_path: PathBuf,

    /// Component to interpret
    #[structopt(short = "c", long = "component", default_value = "main")]
    pub component: String,

    /// Group to interpret
    /// XX(karen): The user can specify a particular group to interpret,
    /// assuming the group is in `main` if not specified otherwise.
    #[structopt(short = "g", long = "group", default_value = "main")]
    pub group: String,
}

/// Interpret a group from a Calyx program
fn main() -> FutilResult<()> {
    let opts = Opts::from_args();

    // Construct IR
    let namespace = frontend::NamespaceDef::new(&opts.file, &opts.lib_path)?;
    let ir = ir::from_ast::ast_to_ir(namespace, false, false)?;

    let ctx = ir::RRC::new(RefCell::new(ir));

    let pm = PassManager::default_passes()?;

    pm.execute_plan(&mut ctx.borrow_mut(), &["validate".to_string()], &[])?;

    let env = environment::Environment::init(&ctx);

    // Get main component; assuming that opts.component is main
    // TODO: handle when component, group are not default values

    let ctx_ref: &ir::Context = &ctx.borrow();
    let mn = ctx_ref
        .components
        .iter()
        .find(|cm| cm.name == "main")
        .ok_or_else(|| {
            Error::Impossible("Cannot find main component".to_string())
        })?;

    match interpret_component(mn, env) {
        Ok(e) => {
            e.print_env();
            Ok(())
        }
        Err(err) => FutilResult::Err(err),
    }
}
