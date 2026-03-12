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
    EmitJson,
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
            "emit-json" => Ok(Mode::EmitJson),
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
            Mode::EmitJson => write!(f, "emit-json"),
        }
    }
}

/// Types of planners to use on the backend. Except for legacy, they "should" all match
/// specification, but may perform at different efficiencies or choose different paths when there
/// is more than one correct path to choose.
enum Planner {
    Legacy,
    Enumerate,
    FromJson,
    #[cfg(feature = "sat_planner")]
    Sat,
}

impl FromStr for Planner {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "legacy" => Ok(Planner::Legacy),
            "enumerate" => Ok(Planner::Enumerate),
            "json" => Ok(Planner::FromJson),
            #[cfg(feature = "sat_planner")]
            "sat" => Ok(Planner::Sat),
            _ => Err("unknown planner".to_string()),
        }
    }
}

impl Display for Planner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Planner::Legacy => write!(f, "legacy"),
            Planner::Enumerate => write!(f, "enumerate"),
            Planner::FromJson => write!(f, "json"),
            #[cfg(feature = "sat_planner")]
            Planner::Sat => write!(f, "sat"),
        }
    }
}

/// edit the configuration file
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "edit")]
pub struct EditConfig {
    /// the editor to use
    #[argh(option, short = 'e')]
    pub editor: Option<String>,
}

/// Adjust keys in the configuration file.
///
/// When given both a key and a value, update the configuration file
/// accordingly. When given a key without a value, display the current
/// value of the key or delete it depending on the delete flag
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "key")]
pub struct KeyConfig {
    #[argh(positional)]
    key: String,

    #[argh(positional)]
    value: Option<String>,

    /// delete the given key
    #[argh(switch, short = 'd')]
    delete: bool,
}

/// print the path of the config file
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "print-path")]
pub struct PrintConfig {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum ConfigAction {
    Edit(EditConfig),
    Path(PrintConfig),
    Key(KeyConfig),
}

/// manipulate the fud2 config
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "config")]
pub struct ConfigCommand {
    #[argh(subcommand)]
    action: ConfigAction,
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
    /// manipulate the configuration file
    Config(ConfigCommand),

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

    /// execution mode (run, plan, emit, gen, dot, emit-json)
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

    // Special case the json planner to skip input sanitization. The json files are just scripts to run.
    if matches!(args.planner, Planner::FromJson) {
        return Ok(Request {
            start_states: vec![],
            end_states: vec![],
            start_files: vec![],
            end_files: vec![],
            through: vec![],
            workdir,
            timing_csv: args.timing_csv.clone(),
            planner: Box::new(plan::JsonPlanner {}),
        });
    }

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
            Planner::Enumerate => Box::new(plan::EnumeratePlanner {}),
            Planner::FromJson => Box::new(plan::JsonPlanner {}),
            #[cfg(feature = "sat_planner")]
            Planner::Sat => Box::new(plan::SatPlanner {}),
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

fn run_config_command(
    driver: &Driver,
    cmd: ConfigCommand,
) -> anyhow::Result<()> {
    match cmd.action {
        ConfigAction::Edit(edit_cmd) => edit_config(driver, edit_cmd),
        ConfigAction::Path(_) => print_config_path(driver),
        ConfigAction::Key(k) => {
            if k.delete {
                delete_config_key(driver, &k.key)
            } else if let Some(val) = k.value {
                set_config_key(driver, &k.key, &val)
            } else {
                print_config_key(driver, &k.key)
            }
        }
    }
}

fn print_config_path(driver: &Driver) -> anyhow::Result<()> {
    let config_path = config::config_path(&driver.name);
    println!("{}", config_path.display());
    Ok(())
}

fn get_resource(driver: &Driver, cmd: GetResource) -> anyhow::Result<()> {
    let to_path = cmd.output.as_deref().unwrap_or(&cmd.filename);

    // Try extracting embedded resource data.
    if let Some(rsrc_files) = &driver.rsrc_files
        && let Some(data) = rsrc_files.get(cmd.filename.as_str())
    {
        log::info!("extracting {} to {}", cmd.filename, to_path);
        std::fs::write(to_path, data)?;
        return Ok(());
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
    let mut toml_doc = get_toml_doc(&config_path)?;

    let config = config::load_config(&driver.name)
        .adjoin(Serialized::default("plugins", [full_path.to_string()]));

    toml_doc["plugins"] = toml_edit::value(
        config
            .extract_inner::<Vec<String>>("plugins")?
            .into_iter()
            .dedup()
            .collect::<toml_edit::Array>(),
    );

    write_config(config_path, toml_doc)?;
    Ok(())
}

fn write_config(
    config_path: std::path::PathBuf,
    toml_doc: toml_edit::DocumentMut,
) -> Result<(), anyhow::Error> {
    // best effort attempt to ensure that the file can be written. There's
    // probably a better way to do this.
    if !config_path.parent().unwrap().exists() {
        fs::create_dir_all(config_path.parent().unwrap())?;
    }

    fs::write(&config_path, toml_doc.to_string())?;
    Ok(())
}

/// gets a toml doc from the configuration file. If the configuration file does
/// not exist, returns an empty document but does not create the file.
fn get_toml_doc(
    config_path: &std::path::PathBuf,
) -> Result<toml_edit::DocumentMut, anyhow::Error> {
    let toml_doc = fs::read_to_string(config_path)
        .map(|s| s.parse::<toml_edit::DocumentMut>())
        .unwrap_or_else(|_| Ok(toml_edit::DocumentMut::new()))?;

    Ok(toml_doc)
}

/// sets the given config key in the config toml to the given value. Note: this
/// cannot handle quoted strings
fn set_config_key(
    driver: &Driver,
    key: &str,
    val: &str,
) -> Result<(), anyhow::Error> {
    let path = config::config_path(&driver.name);
    let mut toml_doc = get_toml_doc(&path)?;
    let mut key_line = key.split('.').collect_vec();
    let final_key = key_line.pop().unwrap();

    let mut_tab = walk_toml(&mut toml_doc, &key_line);

    mut_tab.insert(final_key, val.into());

    write_config(path, toml_doc)
}

/// traverses the config toml and returns the table specified by the list of keys.
/// if the toml lacks the given sub-tables they will be constructed on the way
/// down
fn walk_toml<'a>(
    toml_doc: &'a mut toml_edit::DocumentMut,
    key_line: &[&str],
) -> &'a mut toml_edit::Table {
    let mut mut_tab = toml_doc.as_table_mut();

    for key in key_line {
        mut_tab = mut_tab
            .entry(key)
            .or_insert_with(|| {
                let mut table = toml_edit::Table::new();
                table.set_implicit(true);
                table.set_dotted(true);
                table.into()
            })
            .as_table_mut()
            .unwrap_or_else(|| panic!("{key} is defined and is not a table"));
    }
    mut_tab
}

/// traverses the config file and deletes the given key. Note: cannot handle
/// quoted strings
fn delete_config_key(driver: &Driver, key: &str) -> Result<(), anyhow::Error> {
    let path = config::config_path(&driver.name);
    let mut toml_doc = get_toml_doc(&path)?;
    let mut key_line = key.split('.').collect_vec();
    let final_key = key_line.pop().unwrap();

    let mut_tab = walk_toml(&mut toml_doc, &key_line);
    mut_tab.remove(final_key);

    write_config(path, toml_doc)
}

/// prints the current value of the given config key, if it exists
fn print_config_key(driver: &Driver, key: &str) -> Result<(), anyhow::Error> {
    let path = config::config_path(&driver.name);
    let mut toml_doc = get_toml_doc(&path)?;
    let mut key_line = key.split('.').collect_vec();
    let final_key = key_line.pop().unwrap();

    let tab = walk_toml(&mut toml_doc, &key_line);
    if let Some(val) = tab.get(final_key) {
        println!("{val}");
    }

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
        Some(Subcommand::Config(cmd)) => {
            return run_config_command(driver, cmd);
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
    let mut run = Run::new(driver, plan, req.workdir, config.clone());

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
        Mode::EmitJson => run.show_ops_json(),
    }

    Ok(())
}
