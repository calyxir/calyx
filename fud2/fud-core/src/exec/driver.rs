use super::{OpRef, Operation, Request, Setup, SetupRef, State, StateRef};
use crate::{config, run, script, utils};
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
    const MAX_PATH_LEN: u32 = 6;

    fn try_paths_of_length<F>(
        &self,
        plan: &mut Vec<(OpRef, Vec<StateRef>)>,
        len: u32,
        start: &[StateRef],
        end: &[StateRef],
        good: &F,
    ) -> Option<Vec<(OpRef, Vec<StateRef>)>>
    where
        F: Fn(&[(OpRef, Vec<StateRef>)]) -> bool,
    {
        // check if the plan of given length is valid
        if len == 0 {
            return if good(&plan) {
                Some(plan.clone())
            } else {
                None
            };
        }

        // generate new plans over every loop
        for op_ref in self.ops.keys() {
            // make sure this op has its inputs created at some point
            // that op is also marked as used, later added ops prefered
            // TODO: consider just gening names here, might be easier
            let mut all_generated = true;
            for input in &self.ops[op_ref].input {
                let mut input_generated = false;
                for (o, outs) in plan.iter_mut().rev() {
                    if self.ops[*o].output.contains(input) {
                        input_generated = true;
                        if !outs.contains(input) {
                            outs.push(*input);
                        }
                        break;
                    }
                }
                all_generated &= input_generated || start.contains(input);
            }
            if !all_generated {
                continue;
            }

            // insert the op
            let outputs = self.ops[op_ref].output.clone().into_iter();
            let used_outputs =
                outputs.filter(|s| end.contains(s)).collect::<Vec<_>>();
            plan.push((op_ref, used_outputs));
            if let Some(plan) =
                self.try_paths_of_length(plan, len - 1, start, end, good)
            {
                return Some(plan);
            }
            plan.pop();
        }

        None
    }

    /// creates a sequence of ops and used states from each op
    /// each element of end and through are associated based on their index
    /// currently we assume the amount of items passed is no greater than the states in end
    pub fn find_path(
        &self,
        start: &[StateRef],
        end: &[StateRef],
        through: &[OpRef],
    ) -> Option<Vec<(OpRef, Vec<StateRef>)>> {
        let good = |plan: &[(OpRef, Vec<StateRef>)]| {
            let end_created = end.iter().all(|s| {
                for (_, states) in plan {
                    if states.contains(s) {
                        return true;
                    }
                }
                false
            });
            let through_used = through.iter().all(|t| {
                for (op, _) in plan {
                    if op == t {
                        return true;
                    }
                }
                false
            });
            end_created && through_used
        };

        for len in 1..Self::MAX_PATH_LEN {
            if let Some(plan) =
                self.try_paths_of_length(&mut vec![], len, start, end, &good)
            {
                return Some(plan);
            }
        }
        None
    }

    /// Generate a filename with an extension appropriate for the given State.
    /// We assume states are only used once
    fn gen_name(&self, state_ref: StateRef, used: bool, req: &Request) -> IO {
        let state = &self.states[state_ref];
        let extension = if !state.extensions.is_empty() {
            &state.extensions[0]
        } else {
            ""
        };

        // make sure we have correct input/output filenames and mark if we read from stdio
        if req.start_state.contains(&state_ref) {
            let idx = req.start_state.partition_point(|&r| r == state_ref);
            if let Some(start_file) = req.start_file.get(idx) {
                return IO::File(utils::relative_path(
                    start_file,
                    &req.workdir,
                ));
            } else {
                return IO::StdIO(
                    idx,
                    utils::relative_path(
                        &Utf8PathBuf::from(format!(
                            "_from_stdin_{}",
                            state.name
                        ))
                        .with_extension(extension),
                        &req.workdir,
                    ),
                );
            }
        }

        if req.end_state.contains(&state_ref) {
            let idx = req.end_state.partition_point(|&r| r == state_ref);
            if let Some(end_file) = req.end_file.get(idx) {
                return IO::File(utils::relative_path(end_file, &req.workdir));
            } else {
                return IO::StdIO(
                    idx,
                    utils::relative_path(
                        &Utf8PathBuf::from(format!(
                            "_to_stdout_{}",
                            state.name
                        ))
                        .with_extension(extension),
                        &req.workdir,
                    ),
                );
            }
        }

        // okay, wild west filename time (make them unique and hopefully helpful)
        let prefix = if !used { "_unused_" } else { "" };
        IO::File(if state.is_pseudo() {
            Utf8PathBuf::from(format!("{}from_stdin_{}", prefix, state.name))
                .with_extension(extension)
        } else {
            // TODO avoid collisions in case we reuse extensions...
            Utf8PathBuf::from(format!("{}{}", prefix, state.name))
                .with_extension(extension)
        })
    }

    /// Concoct a plan to carry out the requested build.
    ///
    /// This works by searching for a path through the available operations from the input state
    /// to the output state. If no such path exists in the operation graph, we return None.
    pub fn plan(&self, req: Request) -> Option<Plan> {
        // Find a path through the states.
        let path =
            self.find_path(&req.start_state, &req.end_state, &req.through)?;

        // Generate filenames for each step.
        let steps = path
            .into_iter()
            .map(|(op, used_states)| {
                let input_filenames = self
                    .ops
                    .get(op)
                    .unwrap()
                    .input
                    .iter()
                    .map(|&state| self.gen_name(state, true, &req))
                    .collect::<Vec<_>>();
                let output_filenames = self
                    .ops
                    .get(op)
                    .unwrap()
                    .output
                    .iter()
                    .map(|&state| {
                        self.gen_name(state, used_states.contains(&state), &req)
                    })
                    .collect();
                (op, input_filenames, output_filenames)
            })
            .collect::<Vec<_>>();

        // get filesnames of outputs
        let results = req
            .end_state
            .iter()
            .map(|&s| self.gen_name(s, true, &req))
            .collect::<Vec<_>>();

        Some(Plan {
            steps,
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
            println!();
        }

        println!();
        println!("Operations:");
        for (_, op) in self.ops.iter() {
            println!(
                "  {}: {} -> {}",
                op.name,
                self.states[op.input[0]].name,
                self.states[op.output[0]].name
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
        })
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
    pub fn load_plugins(mut self) -> Self {
        // pull out things from self that we need
        let plugin_dir = self.scripts_dir.take();
        let plugin_files = self.script_files.take();

        // TODO: Let's try to avoid loading/parsing the configuration file here and
        // somehow reusing it from wherever we do that elsewhere.
        let config = config::load_config(&self.name);

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
            );
        }

        // add static plugins (where string is included in binary)
        if let Some(plugin_files) = plugin_files {
            runner.add_static_files(plugin_files.into_iter());
        }

        // add user plugins defined in config
        if let Ok(plugins) =
            config.extract_inner::<Vec<std::path::PathBuf>>("plugins")
        {
            runner.add_files(plugins.into_iter());
        }

        runner.run()
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

#[derive(Debug, PartialEq, PartialOrd)]
pub enum IO {
    // we order these so when they are sorted, StdIO comes first
    // the u32 is a rank used to know which files should be read from stdin first
    StdIO(usize, Utf8PathBuf),
    File(Utf8PathBuf),
}

#[derive(Debug)]
pub struct Plan {
    /// The chain of operations to run and each step's input and output files.
    pub steps: Vec<(OpRef, Vec<IO>, Vec<IO>)>,

    // The final resulting files of the plan.
    pub results: Vec<IO>,

    /// The directory that the build will happen in.
    pub workdir: Utf8PathBuf,
}
