pub use crate::cli_ext::{CliExt, FakeCli, FromArgFn, RedactArgFn};
use crate::config;
use crate::exec::{Driver, Request, StateRef, plan};
use crate::run::Run;
use anyhow::{Context, anyhow, bail};
use argh::FromArgs;
use camino::Utf8PathBuf;
use figment::providers::Serialized;
use itertools::Itertools;
use std::fmt::{Debug, Display};
use std::fs;
use std::str::FromStr;

enum Mode {
    EmitNinja,
    ShowPlan,
    ShowDot,
    Generate,
    Run,
    Cmds,
    OpSeq,
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "emit" => Ok(Mode::EmitNinja),
            "plan" => Ok(Mode::ShowPlan),
            "gen" => Ok(Mode::Generate),
            "run" => Ok(Mode::Run),
            "dot" => Ok(Mode::ShowDot),
            "cmds" => Ok(Mode::Cmds),
            "op-seq" => Ok(Mode::OpSeq),
            _ => Err("unknown mode".to_string()),
        }
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::EmitNinja => write!(f, "emit"),
            Mode::ShowPlan => write!(f, "plan"),
            Mode::Generate => write!(f, "gen"),
            Mode::Run => write!(f, "run"),
            Mode::ShowDot => write!(f, "dot"),
            Mode::Cmds => write!(f, "cmds"),
            Mode::OpSeq => write!(f, "op-seq"),
        }
    }
}

/// Types of planners to use on the backend. Except for legacy, they "should" all match
/// specification, but may perform at different efficiencies or choose different paths when there
/// is more than one correct path to choose.
enum Planner {
    Legacy,
    #[cfg(feature = "egg_planner")]
    Egg,
    Enumerate,
}

impl FromStr for Planner {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "legacy" => Ok(Planner::Legacy),
            #[cfg(feature = "egg_planner")]
            "egg" => Ok(Planner::Egg),
            "enumerate" => Ok(Planner::Enumerate),
            _ => Err("unknown planner".to_string()),
        }
    }
}

impl Display for Planner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Planner::Legacy => write!(f, "legacy"),
            #[cfg(feature = "egg_planner")]
            Planner::Egg => write!(f, "egg"),
            Planner::Enumerate => write!(f, "enumerate"),
        }
    }
}

/// edit the configuration file
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "edit-config")]
pub struct EditConfig {
    /// the editor to use
    #[argh(option, short = 'e')]
    pub editor: Option<String>,
}

/// extract a resource file
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "get-rsrc")]
pub struct GetResource {
    /// the filename to extract
    #[argh(positional)]
    filename: Utf8PathBuf,

    /// destination for the resource file
    #[argh(option, short = 'o')]
    output: Option<Utf8PathBuf>,
}

/// list the available states and ops
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "list")]
pub struct ListCommand {}

/// register a plugin
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "register")]
pub struct RegisterCommand {
    /// the filename of the plugin to register
    #[argh(positional)]
    filename: Utf8PathBuf,
}

/// supported subcommands
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum Subcommand<T: CliExt> {
    /// edit the configuration file
    EditConfig(EditConfig),

    /// extract a resource file
    GetResource(GetResource),

    /// list the available states and ops
    List(ListCommand),

    /// register a plugin
    Register(RegisterCommand),

    #[argh(dynamic)]
    Extended(FakeCli<T>),
}

#[derive(FromArgs)]
/// A generic compiler driver.
pub struct FudArgs<T: CliExt> {
    #[argh(subcommand)]
    pub sub: Option<Subcommand<T>>,

    /// the input file
    #[argh(positional)]
    input: Vec<Utf8PathBuf>,

    /// the output file
    #[argh(option, short = 'o')]
    output: Vec<Utf8PathBuf>,

    /// the states to start from.
    /// The ith state is applied to the ith input file.
    /// If more states are specified than files, files for these states are read from stdin.
    #[argh(option)]
    from: Vec<String>,

    /// the state to produce
    /// The ith state is applied to the ith output file.
    /// If more states are specified than files, files for these states are written to stdout
    #[argh(option)]
    to: Vec<String>,

    /// execution mode (run, plan, emit, gen, dot)
    #[argh(option, short = 'm', default = "Mode::Run")]
    mode: Mode,

    /// working directory for the build
    #[argh(option)]
    dir: Option<Utf8PathBuf>,

    /// in run mode, keep the temporary directory
    #[argh(switch)]
    keep: Option<bool>,

    /// set a configuration variable (key=value)
    #[argh(option, short = 's')]
    set: Vec<String>,

    /// route the conversion through a specific operation
    #[argh(option)]
    through: Vec<String>,

    /// verbose output
    #[argh(switch, short = 'v')]
    verbose: Option<bool>,

    /// quiet mode
    #[argh(switch, short = 'q')]
    quiet: bool,

    /// force rebuild
    #[argh(switch, long = "force-rebuild")]
    force_rebuild: bool,

    /// log level for debugging fud internal
    #[argh(option, long = "log", default = "log::LevelFilter::Warn")]
    pub log_level: log::LevelFilter,

    /// planner for the backend
    #[argh(option, default = "Planner::Legacy")]
    planner: Planner,

    /// name of the file to output timing csv if in running mode
    #[argh(option, long = "csv")]
    timing_csv: Option<Utf8PathBuf>,
}

fn get_states_with_errors(
    driver: &Driver,
    explicit_states: &[String],
    files: &[Utf8PathBuf],
    unknown_state: &str,
    uninferable_file: &str,
    no_states: &str,
) -> anyhow::Result<Vec<StateRef>> {
    let explicit_states = explicit_states.iter().map(|state_str| {
        driver
            .get_state(state_str)
            .ok_or(anyhow!("{unknown_state}"))
    });
    let inferred_states =
        files.iter().skip(explicit_states.len()).map(|input_str| {
            driver
                .guess_state(input_str)
                .ok_or(anyhow!("{uninferable_file}"))
        });
    let states = explicit_states
        .chain(inferred_states)
        .collect::<Result<Vec<_>, _>>()?;
    if states.is_empty() {
        bail!("{no_states}");
    }
    Ok(states)
}

fn from_states<T: CliExt>(
    driver: &Driver,
    args: &FudArgs<T>,
) -> anyhow::Result<Vec<StateRef>> {
    get_states_with_errors(
        driver,
        &args.from,
        &args.input,
        "unknown --from state",
        "could not infer input state",
        "specify an input file or use --from",
    )
}

fn to_state<T: CliExt>(
    driver: &Driver,
    args: &FudArgs<T>,
) -> anyhow::Result<Vec<StateRef>> {
    get_states_with_errors(
        driver,
        &args.to,
        &args.output,
        "unknown --to state",
        "could no infer output state",
        "specify an output file or use --to",
    )
}

fn get_request<T: CliExt>(
    driver: &Driver,
    args: &FudArgs<T>,
) -> anyhow::Result<Request> {
    // The default working directory (if not specified) depends on the mode.
    let workdir = args.dir.clone().unwrap_or_else(|| match args.mode {
        Mode::Generate | Mode::Run => {
            if args.keep.unwrap_or(false) {
                driver.stable_workdir()
            } else {
                driver.fresh_workdir()
            }
        }
        _ => ".".into(),
    });

    // Find all the operations to route through.
    let through: Result<Vec<_>, _> = args
        .through
        .iter()
        .map(|s| {
            driver
                .get_op(s)
                .ok_or(anyhow!("unknown --through op {}", s))
        })
        .collect();
    Ok(Request {
        start_files: args.input.clone(),
        start_states: from_states(driver, args)?,
        end_files: args.output.clone(),
        end_states: to_state(driver, args)?,
        through: through?,
        workdir,
        planner: match args.planner {
            Planner::Legacy => Box::new(plan::LegacyPlanner {}),
            #[cfg(feature = "egg_planner")]
            Planner::Egg => Box::new(plan::EggPlanner {}),
            Planner::Enumerate => Box::new(plan::EnumeratePlanner {}),
        },
        timing_csv: args.timing_csv.clone(),
    })
}

fn edit_config(driver: &Driver, cmd: EditConfig) -> anyhow::Result<()> {
    let editor =
        if let Some(e) = cmd.editor.or_else(|| std::env::var("EDITOR").ok()) {
            e
        } else {
            bail!("$EDITOR not specified. Use -e")
        };
    let config_path = config::config_path(&driver.name);
    log::info!("Editing config at {}", config_path.display());
    let status = std::process::Command::new(editor)
        .arg(config_path)
        .status()
        .expect("failed to execute editor");
    if !status.success() {
        bail!("editor exited with status {}", status);
    }
    Ok(())
}

fn get_resource(driver: &Driver, cmd: GetResource) -> anyhow::Result<()> {
    let to_path = cmd.output.as_deref().unwrap_or(&cmd.filename);

    // Try extracting embedded resource data.
    if let Some(rsrc_files) = &driver.rsrc_files {
        if let Some(data) = rsrc_files.get(cmd.filename.as_str()) {
            log::info!("extracting {} to {}", cmd.filename, to_path);
            std::fs::write(to_path, data)?;
            return Ok(());
        }
    }

    // Try copying a resource file from the resource directory.
    if let Some(rsrc_dir) = &driver.rsrc_dir {
        let from_path = rsrc_dir.join(&cmd.filename);
        if !from_path.exists() {
            bail!("resource file not found: {}", cmd.filename);
        }
        log::info!("copying {} to {}", cmd.filename, to_path);
        std::fs::copy(from_path, to_path)?;
        return Ok(());
    }

    bail!("unknown resource file {}", cmd.filename);
}

fn register_plugin(
    driver: &Driver,
    cmd: RegisterCommand,
) -> anyhow::Result<()> {
    let full_path = cmd
        .filename
        .canonicalize_utf8()
        .with_context(|| format!("Can not find `{}`", cmd.filename))?;

    println!("Registering {full_path}");

    let config_path = config::config_path(&driver.name);
    let contents = fs::read_to_string(&config_path)?;

    let mut toml_doc = contents.parse::<toml_edit::DocumentMut>()?;

    let config = config::load_config(&driver.name)
        .adjoin(Serialized::default("plugins", [full_path.to_string()]));

    toml_doc["plugins"] = toml_edit::value(
        config
            .extract_inner::<Vec<String>>("plugins")?
            .into_iter()
            .dedup()
            .collect::<toml_edit::Array>(),
    );

    fs::write(&config_path, toml_doc.to_string())?;
    Ok(())
}

pub trait CliStart<T: CliExt> {
    /// Given the name of a Driver, returns a config based on that name and CLI arguments.
    fn config_from_cli(name: &str) -> anyhow::Result<figment::Figment>;

    /// Given a driver and config, start the CLI.
    fn cli(driver: &Driver, config: &figment::Figment) -> anyhow::Result<()>;
}

/// Default CLI that provides an interface to core actions.
pub struct DefaultCli;

impl CliStart<()> for DefaultCli {
    fn config_from_cli(name: &str) -> anyhow::Result<figment::Figment> {
        config_from_cli_ext::<()>(name)
    }

    fn cli(driver: &Driver, config: &figment::Figment) -> anyhow::Result<()> {
        cli_ext::<()>(driver, config)
    }
}

impl<T: CliExt> CliStart<T> for T {
    fn config_from_cli(name: &str) -> anyhow::Result<figment::Figment> {
        config_from_cli_ext::<T>(name)
    }

    fn cli(driver: &Driver, config: &figment::Figment) -> anyhow::Result<()> {
        cli_ext::<T>(driver, config)
    }
}

fn config_from_cli_ext<T: CliExt>(
    name: &str,
) -> anyhow::Result<figment::Figment> {
    let args: FudArgs<T> = argh::from_env();
    let mut config = config::load_config(name);

    // Use `--set` arguments to override configuration values.
    for set in args.set {
        let mut parts = set.splitn(2, '=');
        let key = parts.next().unwrap();
        let value = parts
            .next()
            .ok_or(anyhow!("--set arguments must be in key=value form"))?;
        let dict = figment::util::nest(key, value.into());
        config = config.merge(figment::providers::Serialized::defaults(dict));
    }

    Ok(config)
}

fn cli_ext<T: CliExt>(
    driver: &Driver,
    config: &figment::Figment,
) -> anyhow::Result<()> {
    let args: FudArgs<T> = argh::from_env();
    // Configure logging.
    env_logger::Builder::new()
        .format_timestamp(None)
        .filter_level(args.log_level)
        .target(env_logger::Target::Stderr)
        .init();

    // Special commands that bypass the normal behavior.
    match args.sub {
        Some(Subcommand::EditConfig(cmd)) => {
            return edit_config(driver, cmd);
        }
        Some(Subcommand::GetResource(cmd)) => {
            return get_resource(driver, cmd);
        }
        Some(Subcommand::List(_)) => {
            driver.print_info();
            return Ok(());
        }
        Some(Subcommand::Register(cmd)) => {
            return register_plugin(driver, cmd);
        }
        Some(Subcommand::Extended(cmd)) => {
            return cmd.0.run(driver);
        }
        None => {}
    }

    // Make a plan.
    let req = get_request(driver, &args)?;
    let workdir = req.workdir.clone();
    let csv_file = req.timing_csv.clone();
    let csv_path = csv_file.as_ref().map(Utf8PathBuf::as_path);
    let plan = driver.plan(&req).ok_or_else(|| {
        let dest = req
            .end_states
            .iter()
            .map(|state_ref| &driver.states[*state_ref].name)
            .join(", ");
        let src = req
            .start_states
            .iter()
            .map(|state_ref| &driver.states[*state_ref].name)
            .join(", ");
        anyhow!("could not find path from {{{src}}} to {{{dest}}}")
    })?;

    // Configure.
    let mut run = Run::new(driver, plan, config.clone());

    // Override some global config options.
    if let Some(keep) = args.keep {
        run.global_config.keep_build_dir = keep;
    } else if args.dir.is_some() {
        // using the `--dir` argument implies `--keep`
        run.global_config.keep_build_dir = true;
    }
    if let Some(verbose) = args.verbose {
        run.global_config.verbose = verbose;
    }

    // Execute.
    match args.mode {
        Mode::ShowPlan => run.show(),
        Mode::ShowDot => run.show_dot(),
        Mode::EmitNinja => run.emit_to_stdout()?,
        Mode::Generate => run.emit_to_dir(&workdir)?.keep(),
        Mode::Run => run.emit_and_run(
            &workdir,
            false,
            args.quiet,
            args.force_rebuild,
            csv_path,
        )?,
        Mode::Cmds => run.emit_and_run(
            &workdir,
            true,
            false,
            args.force_rebuild,
            csv_path,
        )?,
        Mode::OpSeq => run.show_ops(),
    }

    Ok(())
}
