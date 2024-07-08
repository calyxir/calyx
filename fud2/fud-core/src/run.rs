use crate::config;
use crate::exec::{Driver, OpRef, Plan, SetupRef, StateRef};
use crate::utils::relative_path;
use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::process::{Command, ExitStatus};

/// An error that arises while emitting the Ninja file or executing Ninja.
#[derive(Debug)]
pub enum RunError {
    /// An IO error when writing the Ninja file.
    Io(std::io::Error),

    /// A required configuration key was missing.
    MissingConfig(String),

    /// An invalid value was found for a configuration key the configuration.
    InvalidValue {
        key: String,
        value: String,
        valid_values: Vec<String>,
    },

    /// The Ninja process exited with nonzero status.
    NinjaFailed(ExitStatus),
}

impl From<std::io::Error> for RunError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            RunError::Io(e) => write!(f, "{}", e),
            RunError::MissingConfig(s) => {
                write!(f, "missing required config key: {}", s)
            }
            RunError::InvalidValue {
                key,
                value,
                valid_values,
            } => {
                write!(
                    f,
                    "invalid value '{}' for key '{}'. Valid values are [{}]",
                    value,
                    key,
                    valid_values.iter().join(", ")
                )
            }
            RunError::NinjaFailed(c) => {
                write!(f, "ninja exited with {}", c)
            }
        }
    }
}

impl std::error::Error for RunError {}

pub type EmitResult = std::result::Result<(), RunError>;

/// Code to emit a Ninja `build` command.
pub trait EmitBuild {
    fn build(
        &self,
        emitter: &mut StreamEmitter,
        input: &[&str],
        output: &[&str],
    ) -> EmitResult;
}

pub type EmitBuildFn = fn(&mut StreamEmitter, &[&str], &[&str]) -> EmitResult;

impl EmitBuild for EmitBuildFn {
    fn build(
        &self,
        emitter: &mut StreamEmitter,
        input: &[&str],
        output: &[&str],
    ) -> EmitResult {
        self(emitter, input, output)
    }
}

// TODO make this unnecessary...
/// A simple `build` emitter that just runs a Ninja rule.
pub struct EmitRuleBuild {
    pub rule_name: String,
}

impl EmitBuild for EmitRuleBuild {
    fn build(
        &self,
        emitter: &mut StreamEmitter,
        input: &[&str],
        output: &[&str],
    ) -> EmitResult {
        emitter.build_cmd(output, &self.rule_name, input, &[])?;
        Ok(())
    }
}

/// Code to emit Ninja code at the setup stage.
pub trait EmitSetup {
    fn setup(&self, emitter: &mut StreamEmitter) -> EmitResult;
}

pub type EmitSetupFn = fn(&mut StreamEmitter) -> EmitResult;

impl EmitSetup for EmitSetupFn {
    fn setup(&self, emitter: &mut StreamEmitter) -> EmitResult {
        self(emitter)
    }
}

pub struct Run<'a> {
    pub driver: &'a Driver,
    pub plan: Plan,
    pub config_data: figment::Figment,
    pub global_config: config::GlobalConfig,
}

impl<'a> Run<'a> {
    pub fn new(driver: &'a Driver, plan: Plan) -> Self {
        let config_data = config::load_config(&driver.name);
        Self::with_config(driver, plan, config_data)
    }

    pub fn with_config(
        driver: &'a Driver,
        plan: Plan,
        config_data: figment::Figment,
    ) -> Self {
        let global_config: config::GlobalConfig =
            config_data.extract().expect("failed to load config");
        Self {
            driver,
            plan,
            config_data,
            global_config,
        }
    }

    /// Just print the plan for debugging purposes.
    pub fn show(self) {
        for (op, files_in, files_out) in self.plan.steps {
            println!(
                "{}: {} -> {}",
                self.driver.ops[op].name,
                files_in
                    .into_iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                files_out
                    .into_iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
    }

    /// Print a GraphViz representation of the plan.
    pub fn show_dot(self) {
        println!("digraph plan {{");
        println!("  rankdir=LR;");
        println!("  node[shape=box];");

        // Record the states and ops that are actually used in the plan.
        let mut states: HashMap<StateRef, String> = HashMap::new();
        let mut ops: HashSet<OpRef> = HashSet::new();
        for (op_ref, files_in, files_out) in &self.plan.steps {
            let op = &self.driver.ops[*op_ref];
            for (s, f) in op.input.iter().zip(files_in.iter()) {
                let filename = f.to_string();
                states.insert(*s, filename.to_string());
            }
            for (s, f) in op.output.iter().zip(files_out.iter()) {
                let filename = format!("{f}");
                states.insert(*s, filename.to_string());
            }
            ops.insert(*op_ref);
        }

        // Show all states.
        for (state_ref, state) in self.driver.states.iter() {
            print!("  {} [", state_ref);
            if let Some(filename) = states.get(&state_ref) {
                print!(
                    "label=\"{}\n{}\" penwidth=3 fillcolor=gray style=filled",
                    state.name, filename
                );
            } else {
                print!("label=\"{}\"", state.name);
            }
            println!("];");
        }

        // Show all operations.
        for (op_ref, op) in self.driver.ops.iter() {
            print!(
                "  {} -> {} [label=\"{}\"",
                op.input[0], op.output[0], op.name
            );
            if ops.contains(&op_ref) {
                print!(" penwidth=3");
            }
            println!("];");
        }

        println!("}}");
    }

    /// Print the `build.ninja` file to stdout.
    pub fn emit_to_stdout(&self) -> EmitResult {
        self.emit(std::io::stdout())
    }

    /// Ensure that a directory exists and write `build.ninja` inside it.
    pub fn emit_to_dir(&self, path: &Utf8Path) -> Result<TempDir, RunError> {
        let dir = TempDir::new(path, self.global_config.keep_build_dir)?;

        let ninja_path = path.join("build.ninja");
        let ninja_file = std::fs::File::create(ninja_path)?;
        self.emit(ninja_file)?;

        Ok(dir)
    }

    /// Emit `build.ninja` to a temporary directory and then actually execute ninja.
    pub fn emit_and_run(&self, dir: &Utf8Path) -> EmitResult {
        // Emit the Ninja file.
        let dir = self.emit_to_dir(dir)?;

        // Capture stdin.
        for filename in self.plan.inputs.iter().filter_map(|f| {
            if f.is_from_stdio() {
                Some(f.filename())
            } else {
                None
            }
        }) {
            let stdin_file =
                std::fs::File::create(self.plan.workdir.join(filename))?;
            std::io::copy(
                &mut std::io::stdin(),
                &mut std::io::BufWriter::new(stdin_file),
            )?;
        }

        // Run `ninja` in the working directory.
        let mut cmd = Command::new(&self.global_config.ninja);
        cmd.current_dir(&dir.path);

        if !self.global_config.verbose {
            if ninja_supports_quiet(&self.global_config.ninja)? {
                cmd.arg("--quiet");
            }
        } else {
            cmd.arg("--verbose");
        }

        cmd.stdout(std::io::stderr()); // Send Ninja's stdout to our stderr.
        let status = cmd.status()?;

        // Emit to stdout, only when Ninja succeeded.
        if status.success() {
            // Outputs results to stdio if tagged as such.
            for filename in self.plan.results.iter().filter_map(|f| {
                if f.is_from_stdio() {
                    Some(f.filename())
                } else {
                    None
                }
            }) {
                let stdout_files =
                    std::fs::File::open(self.plan.workdir.join(filename))?;
                std::io::copy(
                    &mut std::io::BufReader::new(stdout_files),
                    &mut std::io::stdout(),
                )?;
            }
            Ok(())
        } else {
            Err(RunError::NinjaFailed(status))
        }
    }

    pub fn emit<T: Write + 'a>(&self, out: T) -> EmitResult {
        let mut emitter = StreamEmitter::new(
            out,
            self.config_data.clone(),
            self.plan.workdir.clone(),
        );

        // Emit preamble.
        emitter.var("build-tool", &self.global_config.exe)?;
        emitter.rule("get-rsrc", "$build-tool get-rsrc $out")?;
        writeln!(emitter.out)?;

        // Emit the setup for each operation used in the plan, only once.
        let mut done_setups = HashSet::<SetupRef>::new();
        for (op, _, _) in &self.plan.steps {
            for setup in &self.driver.ops[*op].setups {
                if done_setups.insert(*setup) {
                    let setup = &self.driver.setups[*setup];
                    writeln!(emitter.out, "# {}", setup.name)?;
                    setup.emit.setup(&mut emitter)?;
                    writeln!(emitter.out)?;
                }
            }
        }

        // Emit the build commands for each step in the plan.
        emitter.comment("build targets")?;
        for (op, in_files, out_files) in &self.plan.steps {
            let op = &self.driver.ops[*op];
            op.emit.build(
                &mut emitter,
                in_files
                    .iter()
                    .map(|io| io.filename().as_str())
                    .collect::<Vec<_>>()
                    .as_slice(),
                out_files
                    .iter()
                    .map(|io| io.filename().as_str())
                    .collect::<Vec<_>>()
                    .as_slice(),
            )?;
        }
        writeln!(emitter.out)?;

        // Mark the last file as the default targets.
        for result in &self.plan.results {
            writeln!(emitter.out, "default {}", result.filename())?;
        }

        Ok(())
    }
}

/// A context for generating Ninja code.
///
/// Callbacks to build functionality that generate Ninja code (setups and ops) use this
/// to access all the relevant configuration and to write out lines of Ninja code.
pub struct Emitter<W: Write> {
    pub out: W,
    pub config_data: figment::Figment,
    pub workdir: Utf8PathBuf,
}

/// A generic emitter that outputs to any `Write` stream.
pub type StreamEmitter<'a> = Emitter<Box<dyn Write + 'a>>;

/// An emitter that buffers the Ninja code in memory.
pub type BufEmitter = Emitter<Vec<u8>>;

impl<'a> StreamEmitter<'a> {
    fn new<T: Write + 'a>(
        out: T,
        config_data: figment::Figment,
        workdir: Utf8PathBuf,
    ) -> Self {
        Self {
            out: Box::new(out),
            config_data,
            workdir,
        }
    }

    /// Create a new bufferred emitter with the same configuration.
    pub fn buffer(&self) -> BufEmitter {
        Emitter {
            out: Vec::new(),
            config_data: self.config_data.clone(),
            workdir: self.workdir.clone(),
        }
    }

    /// Flush the output from a bufferred emitter to this emitter.
    pub fn unbuffer(&mut self, buf: BufEmitter) -> EmitResult {
        self.out.write_all(&buf.out)?;
        Ok(())
    }
}

impl<W: Write> Emitter<W> {
    /// Fetch a configuration value, or panic if it's missing.
    pub fn config_val(&self, key: &str) -> Result<String, RunError> {
        self.config_data
            .extract_inner::<String>(key)
            .map_err(|_| RunError::MissingConfig(key.to_string()))
    }

    /// Fetch a configuration value that is one of the elements in `values`, or panic if it's missing.
    pub fn config_constrained_val(
        &self,
        key: &str,
        valid_values: Vec<&str>,
    ) -> Result<String, RunError> {
        let value = self.config_val(key)?;
        if valid_values.contains(&value.as_str()) {
            Ok(value)
        } else {
            Err(RunError::InvalidValue {
                key: key.to_string(),
                value,
                valid_values: valid_values
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            })
        }
    }

    /// Fetch a configuration value, using a default if it's missing.
    pub fn config_or(&self, key: &str, default: &str) -> String {
        self.config_data
            .extract_inner::<String>(key)
            .unwrap_or_else(|_| default.into())
    }

    /// Fetch a configuration value that is one of the elements in `values`, or return a default if missing.
    /// If an invalid value is explicitly passed, panics.
    pub fn config_constrained_or(
        &self,
        key: &str,
        valid_values: Vec<&str>,
        default: &str,
    ) -> Result<String, RunError> {
        let value = self.config_or(key, default);
        if value.as_str() == default {
            Ok(value)
        } else {
            self.config_constrained_val(key, valid_values)
        }
    }

    /// Emit a Ninja variable declaration for `name` based on the configured value for `key`.
    pub fn config_var(&mut self, name: &str, key: &str) -> EmitResult {
        self.var(name, &self.config_val(key)?)?;
        Ok(())
    }

    /// Emit a Ninja variable declaration for `name` based on the configured value for `key`, or a
    /// default value if it's missing.
    pub fn config_var_or(
        &mut self,
        name: &str,
        key: &str,
        default: &str,
    ) -> std::io::Result<()> {
        self.var(name, &self.config_or(key, default))
    }

    /// Emit a Ninja variable declaration.
    pub fn var(&mut self, name: &str, value: &str) -> std::io::Result<()> {
        writeln!(self.out, "{} = {}", name, value)
    }

    /// Emit a Ninja rule definition.
    pub fn rule(&mut self, name: &str, command: &str) -> std::io::Result<()> {
        writeln!(self.out, "rule {}", name)?;
        writeln!(self.out, "  command = {}", command)
    }

    /// Emit a simple Ninja build command with one dependency.
    pub fn build(
        &mut self,
        rule: &str,
        input: &str,
        output: &str,
    ) -> std::io::Result<()> {
        self.build_cmd(&[output], rule, &[input], &[])
    }

    /// Emit a Ninja build command.
    pub fn build_cmd(
        &mut self,
        targets: &[&str],
        rule: &str,
        deps: &[&str],
        implicit_deps: &[&str],
    ) -> std::io::Result<()> {
        write!(self.out, "build")?;
        for target in targets {
            write!(self.out, " {}", target)?;
        }
        write!(self.out, ": {}", rule)?;
        for dep in deps {
            write!(self.out, " {}", dep)?;
        }
        if !implicit_deps.is_empty() {
            write!(self.out, " |")?;
            for dep in implicit_deps {
                write!(self.out, " {}", dep)?;
            }
        }
        writeln!(self.out)?;
        Ok(())
    }

    /// Emit a Ninja comment.
    pub fn comment(&mut self, text: &str) -> std::io::Result<()> {
        writeln!(self.out, "# {}", text)?;
        Ok(())
    }

    /// Add a file to the build directory.
    pub fn add_file(&self, name: &str, contents: &[u8]) -> std::io::Result<()> {
        let path = self.workdir.join(name);
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Get a path to an external file. The input `path` may be relative to our original
    /// invocation; we make it relative to the build directory so it can safely be used in the
    /// Ninja file.
    pub fn external_path(&self, path: &Utf8Path) -> Utf8PathBuf {
        relative_path(path, &self.workdir)
    }

    /// Add a variable parameter to a rule or build command.
    pub fn arg(&mut self, name: &str, value: &str) -> std::io::Result<()> {
        writeln!(self.out, "  {} = {}", name, value)?;
        Ok(())
    }

    /// Add a build command to extract a resource file into the build directory.
    pub fn rsrc(&mut self, filename: &str) -> std::io::Result<()> {
        self.build_cmd(&[filename], "get-rsrc", &[], &[])
    }
}

/// Check whether a Ninja executable supports the `--quiet` flag.
fn ninja_supports_quiet(ninja: &str) -> std::io::Result<bool> {
    let version_output = Command::new(ninja).arg("--version").output()?;
    if let Ok(version) = String::from_utf8(version_output.stdout) {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() >= 2 {
            let major = parts[0].parse::<u32>().unwrap_or(0);
            let minor = parts[1].parse::<u32>().unwrap_or(0);
            Ok(major > 1 || (major == 1 && minor >= 11))
        } else {
            Ok(false)
        }
    } else {
        Ok(false)
    }
}

/// A directory that can optionally delete itself when we're done with it.
pub struct TempDir {
    path: Utf8PathBuf,
    delete: bool,
}

impl TempDir {
    /// Create a directory *or* use an existing directory.
    ///
    /// If the directory already exists, we will not delete it (regardless of `keep`). Otherwise,
    /// we will create a new one, and we will delete it when this object is dropped, unless
    /// `keep` is true.
    pub fn new(path: &Utf8Path, keep: bool) -> std::io::Result<Self> {
        let delete = !path.exists() && !keep;
        std::fs::create_dir_all(path)?;
        Ok(Self {
            path: path.into(),
            delete,
        })
    }

    /// If this directory would otherwise be deleted, don't.
    pub fn keep(&mut self) {
        self.delete = false;
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if self.delete {
            // We must ignore errors when attempting to delete.
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}
