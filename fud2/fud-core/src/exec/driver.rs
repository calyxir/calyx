use super::{OpRef, Operation, Request, Setup, SetupRef, State, StateRef};
use crate::{flang::PathRef, run, script, utils};
use camino::{Utf8Path, Utf8PathBuf};
use cranelift_entity::PrimaryMap;
use rand::distributions::{Alphanumeric, DistString};
use std::{collections::HashMap, error::Error, ffi::OsStr, fmt::Display};

type FileData = HashMap<&'static str, &'static [u8]>;

/// A Driver encapsulates a set of States and the Operations that can transform between them. It
/// contains all the machinery to perform builds in a given ecosystem.
pub struct Driver {
    pub name: String,
    pub setups: PrimaryMap<SetupRef, Setup>,
    pub states: PrimaryMap<StateRef, State>,
    pub ops: PrimaryMap<OpRef, Operation>,
    pub rsrc_dir: Option<Utf8PathBuf>,
    pub rsrc_files: Option<FileData>,
}

impl Driver {
    /// Concoct a plan to carry out the requested build.
    ///
    /// This works by searching for a path through the available operations from the input state
    /// to the output state. If no such path exists in the operation graph, we return None.
    pub fn plan(&self, req: &Request) -> Option<Plan> {
        // Find a plan through the states.
        let resp =
            req.planner
                .find_plan(&req.into(), &self.ops, &self.states)?;

        // Input and output files should have their paths relative to the user running `fud2`
        // instead of the working directory. This gets the path of any `PathRef` `r` from `resp`
        // and if that `r` is an input or output it augments the path to be relative to the user.
        let get_path = |r: &PathRef| {
            let p = resp.path(*r).clone();
            if (resp.inputs().contains(r) || resp.outputs().contains(r))
                && (!resp.stdins().contains(r) && !resp.stdouts().contains(r))
            {
                utils::relative_path(&p, &req.workdir)
            } else {
                p
            }
        };

        // Convert response in flang into a list of ops an their input/output file paths.
        let steps = resp
            .iter()
            .map(|assign| {
                (
                    assign.op_ref(),
                    assign.args().iter().map(get_path).collect(),
                    assign.rets().iter().map(get_path).collect(),
                )
            })
            .collect();

        // Gets the path of an input and tags it if it is a file or is read/written to stdio. If
        // the path represents a file, it should be relative to the user, so this it's path is
        // modified to be relative to the user instead of the working directory.
        let get_io = |r: PathRef, stdios: &[PathRef]| {
            let p = resp.path(r).clone();
            if stdios.contains(&r) {
                IO::StdIO(p)
            } else {
                IO::File(utils::relative_path(&p, &req.workdir))
            }
        };

        let inputs = resp
            .inputs()
            .iter()
            .map(|&r| get_io(r, resp.stdins()))
            .collect();

        let results = resp
            .outputs()
            .iter()
            .map(|&r| get_io(r, resp.stdouts()))
            .collect();
        Some(Plan {
            steps,
            inputs,
            results,
            workdir: req.workdir.clone(),
        })
    }

    /// Infer the state of a file based on its extension.
    ///
    /// Multiple states can use the same extension. The first state registered "wins."
    pub fn guess_state(&self, path: &Utf8Path) -> Option<StateRef> {
        let ext = path.extension()?;
        self.states
            .iter()
            .find(|(_, state_data)| state_data.ext_matches(ext))
            .map(|(state, _)| state)
    }

    /// Look up a state by its name.
    pub fn get_state(&self, name: &str) -> Option<StateRef> {
        self.states
            .iter()
            .find(|(_, state_data)| state_data.name == name)
            .map(|(state, _)| state)
    }

    /// Look an operation by its name.
    pub fn get_op(&self, name: &str) -> Option<OpRef> {
        self.ops
            .iter()
            .find(|(_, op_data)| op_data.name == name)
            .map(|(op, _)| op)
    }

    /// The default working directory name when we want the same directory on every run.
    pub fn stable_workdir(&self) -> Utf8PathBuf {
        format!(".{}", &self.name).into()
    }

    /// A new working directory that does not yet exist on the filesystem, for when we
    /// want to avoid collisions.
    pub fn fresh_workdir(&self) -> Utf8PathBuf {
        loop {
            let rand_suffix =
                Alphanumeric.sample_string(&mut rand::thread_rng(), 8);
            let path: Utf8PathBuf =
                format!(".{}-{}", &self.name, rand_suffix).into();
            if !path.exists() {
                return path;
            }
        }
    }

    /// Print a list of registered states and operations to stdout.
    pub fn print_info(&self) {
        println!("States:");
        for (_, state) in self.states.iter() {
            print!("  {}:", state.name);
            for ext in &state.extensions {
                print!(" .{ext}");
            }
            if let Some(src) = &state.source {
                print!(" ({src})")
            }
            println!();
        }

        println!();
        println!("Operations:");
        for (_, op) in self.ops.iter() {
            let dev_info = op
                .source
                .as_ref()
                .map(|src| format!(" ({src})"))
                .unwrap_or_default();
            println!(
                "  {}: {} -> {}{}",
                op.name,
                self.states[op.input[0]].name,
                self.states[op.output[0]].name,
                dev_info
            );
        }
    }
}

pub struct DriverBuilder {
    pub name: String,
    setups: PrimaryMap<SetupRef, Setup>,
    states: PrimaryMap<StateRef, State>,
    ops: PrimaryMap<OpRef, Operation>,
    rsrc_dir: Option<Utf8PathBuf>,
    rsrc_files: Option<FileData>,
    scripts_dir: Option<Utf8PathBuf>,
    script_files: Option<FileData>,
}

#[derive(Debug)]
pub enum DriverError {
    UnknownState(String),
    UnknownSetup(String),
}

impl Display for DriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriverError::UnknownState(state) => {
                write!(f, "Unknown state: {state}")
            }
            DriverError::UnknownSetup(setup) => {
                write!(f, "Unknown state: {setup}")
            }
        }
    }
}

impl Error for DriverError {}

impl DriverBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            setups: Default::default(),
            states: Default::default(),
            ops: Default::default(),
            rsrc_dir: None,
            rsrc_files: None,
            scripts_dir: None,
            script_files: None,
        }
    }

    pub fn state(&mut self, name: &str, extensions: &[&str]) -> StateRef {
        self.states.push(State {
            name: name.to_string(),
            extensions: extensions.iter().map(|s| s.to_string()).collect(),
            source: None,
        })
    }

    pub fn state_source<S: ToString>(&mut self, state: StateRef, src: S) {
        self.states[state].source = Some(src.to_string());
    }

    pub fn find_state(&self, needle: &str) -> Result<StateRef, DriverError> {
        self.states
            .iter()
            .find(|(_, State { name, .. })| needle == name)
            .map(|(state_ref, _)| state_ref)
            .ok_or_else(|| DriverError::UnknownState(needle.to_string()))
    }

    pub fn add_setup<T: run::EmitSetup + 'static>(
        &mut self,
        name: &str,
        emit: T,
    ) -> SetupRef {
        self.setups.push(Setup {
            name: name.into(),
            emit: Box::new(emit),
        })
    }

    pub fn setup(&mut self, name: &str, func: run::EmitSetupFn) -> SetupRef {
        self.add_setup(name, func)
    }

    pub fn find_setup(&self, needle: &str) -> Result<SetupRef, DriverError> {
        self.setups
            .iter()
            .find(|(_, Setup { name, .. })| needle == name)
            .map(|(setup_ref, _)| setup_ref)
            .ok_or_else(|| DriverError::UnknownSetup(needle.to_string()))
    }

    pub fn add_op<T: run::EmitBuild + 'static>(
        &mut self,
        name: &str,
        setups: &[SetupRef],
        input: &[StateRef],
        output: &[StateRef],
        emit: T,
    ) -> OpRef {
        self.ops.push(Operation {
            name: name.into(),
            setups: setups.into(),
            input: input.into(),
            output: output.into(),
            emit: Box::new(emit),
            source: None,
        })
    }

    pub fn op(
        &mut self,
        name: &str,
        setups: &[SetupRef],
        input: StateRef,
        output: StateRef,
        build: run::EmitBuildFn,
    ) -> OpRef {
        self.add_op(name, setups, &[input], &[output], build)
    }

    pub fn op_source<S: ToString>(&mut self, op: OpRef, src: S) {
        self.ops[op].source = Some(src.to_string());
    }

    pub fn rule(
        &mut self,
        setups: &[SetupRef],
        input: StateRef,
        output: StateRef,
        rule_name: &str,
    ) -> OpRef {
        self.add_op(
            rule_name,
            setups,
            &[input],
            &[output],
            run::EmitRuleBuild {
                rule_name: rule_name.to_string(),
            },
        )
    }

    pub fn rsrc_dir(&mut self, path: &str) {
        self.rsrc_dir = Some(path.into());
    }

    pub fn rsrc_files(&mut self, files: FileData) {
        self.rsrc_files = Some(files);
    }

    pub fn scripts_dir(&mut self, path: &str) {
        self.scripts_dir = Some(path.into());
    }

    pub fn script_files(&mut self, files: FileData) {
        self.script_files = Some(files);
    }

    /// Load any plugin scripts specified in the configuration file.
    pub fn load_plugins(
        mut self,
        config_data: &figment::Figment,
    ) -> anyhow::Result<Self> {
        // pull out things from self that we need
        let plugin_dir = self.scripts_dir.take();
        let plugin_files = self.script_files.take();

        let mut runner = script::ScriptRunner::new(self);

        // add system plugins
        if let Some(plugin_dir) = plugin_dir {
            runner.add_files(
                std::fs::read_dir(plugin_dir)
                    .unwrap()
                    // filter out invalid paths
                    .filter_map(|dir_entry| dir_entry.map(|p| p.path()).ok())
                    // filter out paths that don't have `.rhai` extension
                    .filter(|p| p.extension() == Some(OsStr::new("rhai"))),
            )?;
        }

        // add static plugins (where string is included in binary)
        if let Some(plugin_files) = plugin_files {
            runner.add_static_files(plugin_files.into_iter());
        }

        // add user plugins defined in config
        if let Ok(plugins) =
            config_data.extract_inner::<Vec<std::path::PathBuf>>("plugins")
        {
            runner.add_files(plugins.into_iter())?;
        }

        Ok(runner.run())
    }

    pub fn build(self) -> Driver {
        Driver {
            name: self.name,
            setups: self.setups,
            states: self.states,
            ops: self.ops,
            rsrc_dir: self.rsrc_dir,
            rsrc_files: self.rsrc_files,
        }
    }
}

/// A file tagged with its input source.
#[derive(Debug, Clone)]
pub enum IO {
    /// A file at a given path which is to be read from stdin or output to stdout.
    StdIO(Utf8PathBuf),
    /// A file at a given path which need not be read from stdin or output ot stdout.
    File(Utf8PathBuf),
}

impl IO {
    /// Returns the filename of the file `self` represents
    pub fn filename(&self) -> &Utf8PathBuf {
        match self {
            Self::StdIO(p) => p,
            Self::File(p) => p,
        }
    }

    /// Returns if `self` is a `StdIO`.
    pub fn is_from_stdio(&self) -> bool {
        match self {
            Self::StdIO(_) => true,
            Self::File(_) => false,
        }
    }
}

impl Display for IO {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.filename())
    }
}

#[derive(Debug)]
pub struct Plan {
    /// The chain of operations to run and each step's input and output files.
    pub steps: Vec<(OpRef, Vec<Utf8PathBuf>, Vec<Utf8PathBuf>)>,

    /// The inputs used to generate the results.
    /// Earlier elements of inputs should be read before later ones.
    pub inputs: Vec<IO>,

    /// The resulting files of the plan.
    /// Earlier elements of inputs should be written before later ones.
    pub results: Vec<IO>,

    /// The directory that the build will happen in.
    pub workdir: Utf8PathBuf,
}
