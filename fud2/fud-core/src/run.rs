use crate::config;
use crate::exec::{Driver, OpRef, Plan, SetupRef, StateRef};
use crate::utils::relative_path;
use camino::{Utf8Path, Utf8PathBuf};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::process::Command;

/// An error that arises while emitting the Ninja file.
#[derive(Debug)]
pub enum EmitError {
    Io(std::io::Error),
    MissingConfig(String),
}

impl From<std::io::Error> for EmitError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::fmt::Display for EmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            EmitError::Io(e) => write!(f, "{}", e),
            EmitError::MissingConfig(s) => {
                write!(f, "missing required config key: {}", s)
            }
        }
    }
}

impl std::error::Error for EmitError {}

pub type EmitResult = std::result::Result<(), EmitError>;

/// Code to emit a Ninja `build` command.
pub trait EmitBuild {
    fn build(
        &self,
        emitter: &mut Emitter,
        input: &str,
        output: &str,
    ) -> EmitResult;
}

pub type EmitBuildFn = fn(&mut Emitter, &str, &str) -> EmitResult;

impl EmitBuild for EmitBuildFn {
    fn build(
        &self,
        emitter: &mut Emitter,
        input: &str,
        output: &str,
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
        emitter: &mut Emitter,
        input: &str,
        output: &str,
    ) -> EmitResult {
        emitter.build(&self.rule_name, input, output)?;
        Ok(())
    }
}

/// Code to emit Ninja code at the setup stage.
pub trait EmitSetup {
    fn setup(&self, emitter: &mut Emitter) -> EmitResult;
}

pub type EmitSetupFn = fn(&mut Emitter) -> EmitResult;

impl EmitSetup for EmitSetupFn {
    fn setup(&self, emitter: &mut Emitter) -> EmitResult {
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
        if self.plan.stdin {
            println!("(stdin) -> {}", self.plan.start);
        } else {
            println!("start: {}", self.plan.start);
        }
        for (op, file) in self.plan.steps {
            println!("{}: {} -> {}", op, self.driver.ops[op].name, file);
        }
        if self.plan.stdout {
            println!("-> (stdout)");
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
        let first_op = self.plan.steps[0].0;
        states.insert(
            self.driver.ops[first_op].input,
            self.plan.start.to_string(),
        );
        for (op, file) in &self.plan.steps {
            states.insert(self.driver.ops[*op].output, file.to_string());
            ops.insert(*op);
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
            print!("  {} -> {} [label=\"{}\"", op.input, op.output, op.name);
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
    pub fn emit_to_dir(&self, dir: &Utf8Path) -> EmitResult {
        std::fs::create_dir_all(dir)?;
        let ninja_path = dir.join("build.ninja");
        let ninja_file = std::fs::File::create(ninja_path)?;

        self.emit(ninja_file)
    }

    /// Emit `build.ninja` to a temporary directory and then actually execute ninja.
    pub fn emit_and_run(&self, dir: &Utf8Path) -> EmitResult {
        // Emit the Ninja file.
        let stale_dir = dir.exists();
        self.emit_to_dir(dir)?;

        // Capture stdin.
        if self.plan.stdin {
            let stdin_file = std::fs::File::create(
                self.plan.workdir.join(&self.plan.start),
            )?;
            std::io::copy(
                &mut std::io::stdin(),
                &mut std::io::BufWriter::new(stdin_file),
            )?;
        }

        // Run `ninja` in the working directory.
        let mut cmd = Command::new(&self.global_config.ninja);
        cmd.current_dir(dir);
        if self.plan.stdout && !self.global_config.verbose {
            // When we're printing to stdout, suppress Ninja's output by default.
            cmd.stdout(std::process::Stdio::null());
        }
        cmd.status()?;

        // Emit stdout.
        if self.plan.stdout {
            let stdout_file =
                std::fs::File::open(self.plan.workdir.join(self.plan.end()))?;
            std::io::copy(
                &mut std::io::BufReader::new(stdout_file),
                &mut std::io::stdout(),
            )?;
        }

        // Remove the temporary directory unless it already existed at the start *or* the user specified `--keep`.
        if !self.global_config.keep_build_dir && !stale_dir {
            std::fs::remove_dir_all(dir)?;
        }

        Ok(())
    }

    pub fn emit<T: Write + 'a>(&self, out: T) -> EmitResult {
        let mut emitter = Emitter::new(
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
        for (op, _) in &self.plan.steps {
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
        let mut last_file = &self.plan.start;
        for (op, out_file) in &self.plan.steps {
            let op = &self.driver.ops[*op];
            op.emit.build(
                &mut emitter,
                last_file.as_str(),
                out_file.as_str(),
            )?;
            last_file = out_file;
        }
        writeln!(emitter.out)?;

        // Mark the last file as the default target.
        writeln!(emitter.out, "default {}", last_file)?;

        Ok(())
    }
}

pub struct Emitter<'a> {
    pub out: Box<dyn Write + 'a>,
    pub config_data: figment::Figment,
    pub workdir: Utf8PathBuf,
}

impl<'a> Emitter<'a> {
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

    /// Fetch a configuration value, or panic if it's missing.
    pub fn config_val(&self, key: &str) -> Result<String, EmitError> {
        self.config_data
            .extract_inner::<String>(key)
            .map_err(|_| EmitError::MissingConfig(key.to_string()))
    }

    /// Fetch a configuration value, using a default if it's missing.
    pub fn config_or(&self, key: &str, default: &str) -> String {
        self.config_data
            .extract_inner::<String>(key)
            .unwrap_or_else(|_| default.into())
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
