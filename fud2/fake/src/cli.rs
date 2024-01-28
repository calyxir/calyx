use crate::driver::{Driver, Request, StateRef};
use crate::run::Run;
use anyhow::{anyhow, bail};
use argh::FromArgs;
use camino::{Utf8Path, Utf8PathBuf};
use std::fmt::Display;
use std::str::FromStr;

enum Mode {
    EmitNinja,
    ShowPlan,
    ShowDot,
    Generate,
    Run,
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
        }
    }
}

#[derive(FromArgs)]
/// A generic compiler driver.
struct FakeArgs {
    /// the input file
    #[argh(positional)]
    input: Option<Utf8PathBuf>,

    /// the output file
    #[argh(option, short = 'o')]
    output: Option<Utf8PathBuf>,

    /// the state to start from
    #[argh(option)]
    from: Option<String>,

    /// the state to produce
    #[argh(option)]
    to: Option<String>,

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

    /// verbose ouput
    #[argh(switch, short = 'v')]
    verbose: Option<bool>,
}

fn from_state(driver: &Driver, args: &FakeArgs) -> anyhow::Result<StateRef> {
    match &args.from {
        Some(name) => driver
            .get_state(name)
            .ok_or(anyhow!("unknown --from state")),
        None => match args.input {
            Some(ref input) => driver
                .guess_state(input)
                .ok_or(anyhow!("could not infer input state")),
            None => bail!("specify an input file or use --from"),
        },
    }
}

fn to_state(driver: &Driver, args: &FakeArgs) -> anyhow::Result<StateRef> {
    match &args.to {
        Some(name) => driver.get_state(name).ok_or(anyhow!("unknown --to state")),
        None => match &args.output {
            Some(out) => driver
                .guess_state(out)
                .ok_or(anyhow!("could not infer output state")),
            None => Err(anyhow!("specify an output file or use --to")),
        },
    }
}

fn get_request(driver: &Driver, args: &FakeArgs) -> anyhow::Result<Request> {
    // The default working directory (if not specified) depends on the mode.
    let default_workdir = driver.default_workdir();
    let workdir = args.dir.as_deref().unwrap_or_else(|| match args.mode {
        Mode::Generate | Mode::Run => default_workdir.as_ref(),
        _ => Utf8Path::new("."),
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
        start_file: args.input.clone(),
        start_state: from_state(driver, args)?,
        end_file: args.output.clone(),
        end_state: to_state(driver, args)?,
        through: through?,
        workdir: workdir.into(),
    })
}

pub fn cli(driver: &Driver) -> anyhow::Result<()> {
    let args: FakeArgs = argh::from_env();

    // Make a plan.
    let req = get_request(driver, &args)?;
    let workdir = req.workdir.clone();
    let plan = driver.plan(req).ok_or(anyhow!("could not find path"))?;

    // Configure.
    let mut run = Run::new(driver, plan);

    // Override some global config options.
    if let Some(keep) = args.keep {
        run.global_config.keep_build_dir = keep;
    }
    if let Some(verbose) = args.verbose {
        run.global_config.verbose = verbose;
    }

    // Use `--set` arguments to override configuration values.
    for set in args.set {
        let mut parts = set.splitn(2, '=');
        let key = parts.next().unwrap();
        let value = parts
            .next()
            .ok_or(anyhow!("--set arguments must be in key=value form"))?;
        let dict = figment::util::nest(key, value.into());
        run.config_data = run
            .config_data
            .merge(figment::providers::Serialized::defaults(dict));
    }

    // Execute.
    match args.mode {
        Mode::ShowPlan => run.show(),
        Mode::ShowDot => run.show_dot(),
        Mode::EmitNinja => run.emit_to_stdout()?,
        Mode::Generate => run.emit_to_dir(&workdir)?,
        Mode::Run => run.emit_and_run(&workdir)?,
    }

    Ok(())
}
