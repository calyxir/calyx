use super::{OpRef, Operation, Request, Setup, SetupRef, State, StateRef};
use crate::{run, script, utils};
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
    /// Generate a filename with an extension appropriate for the given State, `state_ref` relative
    /// to `workdir`.
    ///
    /// If `user_visible`, then the path should not be in the working directory because the user
    /// should see the generated file (or provides the file).
    ///
    /// If `used` is false, the state is neither an output to the user, or used as input an op. In
    /// this case, the filename associated with the state will be prefixed by `_unused_`.
    fn gen_name(
        &self,
        state_ref: StateRef,
        used: bool,
        user_visible: bool,
        workdir: &Utf8PathBuf,
    ) -> IO {
        let state = &self.states[state_ref];

        let prefix = if !used { "_unused_" } else { "" };
        let extension = if !state.extensions.is_empty() {
            &state.extensions[0]
        } else {
            ""
        };

        // Only make the path relative, i.e. not in the workdir if the file should be user visible.
        let post_process = if user_visible {
            utils::relative_path
        } else {
            |x: &Utf8Path, _y: &Utf8Path| x.to_path_buf()
        };

        IO::File(if state.is_pseudo() {
            post_process(
                &Utf8PathBuf::from(format!("{}pseudo_{}", prefix, state.name)),
                workdir,
            )
        } else {
            // TODO avoid collisions in case of reused extensions...
            post_process(
                &Utf8PathBuf::from(format!("{}{}", prefix, state.name))
                    .with_extension(extension),
                workdir,
            )
        })
    }

    /// Generates a filename for a state tagged with if the file should be read from StdIO and path
    /// name relative to `workdir`.
    ///
    /// The state is searched for in `states`. If it is found, the name at the same index in `files` is
    /// returned, else `stdio_name` is returned.
    ///
    /// If the state is not in states, new name is generated.
    /// This name will be prefixed by `_unused_` if unused is `true`. This signifies the file is
    /// neither requested as an output by the user nor used as input to any op.
    fn gen_name_or_use_given(
        &self,
        state_ref: StateRef,
        states: &[StateRef],
        files: &[Utf8PathBuf],
        stdio_name: &str,
        used: bool,
        workdir: &Utf8PathBuf,
    ) -> IO {
        let state = &self.states[state_ref];
        let extension = if !state.extensions.is_empty() {
            &state.extensions[0]
        } else {
            ""
        };

        // If the state is found in `states` the user should see this file. Amoung other things,
        // this means it should be in their directory and not the working directory.
        let user_visible = states.contains(&state_ref);

        if let Some(idx) = states.iter().position(|&s| s == state_ref) {
            if let Some(filename) = files.get(idx) {
                if user_visible {
                    IO::File(utils::relative_path(&filename.clone(), workdir))
                } else {
                    IO::File(filename.clone())
                }
            } else {
                IO::StdIO(
                    Utf8PathBuf::from(stdio_name).with_extension(extension),
                )
            }
        } else {
            self.gen_name(state_ref, used, user_visible, workdir)
        }
    }

    /// Generates a vector contianing filenames for files of each state in `states`.
    ///
    /// `req` is used to generate filenames as inputs and outputs may want to takes names from
    /// `req.end_files` or `req.start_files`.
    ///
    /// `input` is true if all states in `states` are an input to an op.
    /// `used` is the states in `states` which are an input to another op or in `req.end_states`.
    fn gen_names(
        &self,
        states: &[StateRef],
        req: &Request,
        input: bool,
        used: &[StateRef],
    ) -> Vec<IO> {
        // Inputs cannot be results, so look at starting states, else look at ending states.
        let req_states = if input {
            &req.start_states
        } else {
            &req.end_states
        };

        // Inputs cannot be results, so look at starting files, else look at ending files.
        let req_files = if input {
            &req.start_files
        } else {
            &req.end_files
        };
        // The above lists can't be the concatination of the two branches because start and end
        // states are not necessarily disjoint, but they could still have different files assigned
        // to each state.

        states
            .iter()
            .map(|&state| {
                let stdio_name = if input {
                    format!("_from_stdin_{}", self.states[state].name)
                } else {
                    format!("_to_stdout_{}", self.states[state].name)
                };

                self.gen_name_or_use_given(
                    state,
                    req_states,
                    req_files,
                    &stdio_name,
                    input || used.contains(&state),
                    &req.workdir,
                )
            })
            .collect()
    }

    /// Concoct a plan to carry out the requested build.
    ///
    /// This works by searching for a path through the available operations from the input state
    /// to the output state. If no such path exists in the operation graph, we return None.
    pub fn plan(&self, req: Request) -> Option<Plan> {
        // Find a plan through the states.
        let path = req.planner.find_plan(
            &req.start_states,
            &req.end_states,
            &req.through,
            &self.ops,
            &self.states,
        )?;

        // Generate filenames for each step.
        let steps = path
            .into_iter()
            .map(|(op, used)| {
                let input_filenames =
                    self.gen_names(&self.ops[op].input, &req, true, &used);
                let output_filenames =
                    self.gen_names(&self.ops[op].output, &req, false, &used);
                (op, input_filenames, output_filenames)
            })
            .collect::<Vec<_>>();

        // Collect filenames of inputs and outputs
        let results =
            self.gen_names(&req.end_states, &req, false, &req.end_states);
        let inputs =
            self.gen_names(&req.start_states, &req, true, &req.start_states);

        Some(Plan {
            steps,
            inputs,
            results,
            workdir: req.workdir,
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
                print!(" .{}", ext);
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
    pub steps: Vec<(OpRef, Vec<IO>, Vec<IO>)>,

    /// The inputs used to generate the results.
    /// Earlier elements of inputs should be read before later ones.
    pub inputs: Vec<IO>,

    /// The resulting files of the plan.
    /// Earlier elements of inputs should be written before later ones.
    pub results: Vec<IO>,

    /// The directory that the build will happen in.
    pub workdir: Utf8PathBuf,
}
