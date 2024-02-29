use super::{OpRef, Operation, Request, Setup, SetupRef, State, StateRef};
use crate::{run, utils};
use camino::{Utf8Path, Utf8PathBuf};
use cranelift_entity::{PrimaryMap, SecondaryMap};
use std::collections::HashMap;

#[derive(PartialEq)]
enum Destination {
    State(StateRef),
    Op(OpRef),
}

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
    /// Find a chain of Operations from the `start` state to the `end`, which may be a state or the
    /// final operation in the chain.
    fn find_path_segment(
        &self,
        start: StateRef,
        end: Destination,
    ) -> Option<Vec<OpRef>> {
        // Our start state is the input.
        let mut visited = SecondaryMap::<StateRef, bool>::new();
        visited[start] = true;

        // Build the incoming edges for each vertex.
        let mut breadcrumbs = SecondaryMap::<StateRef, Option<OpRef>>::new();

        // Breadth-first search.
        let mut state_queue: Vec<StateRef> = vec![start];
        while !state_queue.is_empty() {
            let cur_state = state_queue.remove(0);

            // Finish when we reach the goal vertex.
            if end == Destination::State(cur_state) {
                break;
            }

            // Traverse any edge from the current state to an unvisited state.
            for (op_ref, op) in self.ops.iter() {
                if op.input == cur_state && !visited[op.output] {
                    state_queue.push(op.output);
                    visited[op.output] = true;
                    breadcrumbs[op.output] = Some(op_ref);
                }

                // Finish when we reach the goal edge.
                if end == Destination::Op(op_ref) {
                    break;
                }
            }
        }

        // Traverse the breadcrumbs backward to build up the path back from output to input.
        let mut op_path: Vec<OpRef> = vec![];
        let mut cur_state = match end {
            Destination::State(state) => state,
            Destination::Op(op) => {
                op_path.push(op);
                self.ops[op].input
            }
        };
        while cur_state != start {
            match breadcrumbs[cur_state] {
                Some(op) => {
                    op_path.push(op);
                    cur_state = self.ops[op].input;
                }
                None => return None,
            }
        }
        op_path.reverse();

        Some(op_path)
    }

    /// Find a chain of operations from the `start` state to the `end` state, passing through each
    /// `through` operation in order.
    pub fn find_path(
        &self,
        start: StateRef,
        end: StateRef,
        through: &[OpRef],
    ) -> Option<Vec<OpRef>> {
        let mut cur_state = start;
        let mut op_path: Vec<OpRef> = vec![];

        // Build path segments through each through required operation.
        for op in through {
            let segment =
                self.find_path_segment(cur_state, Destination::Op(*op))?;
            op_path.extend(segment);
            cur_state = self.ops[*op].output;
        }

        // Build the final path segment to the destination state.
        let segment =
            self.find_path_segment(cur_state, Destination::State(end))?;
        op_path.extend(segment);

        Some(op_path)
    }

    /// Generate a filename with an extension appropriate for the given State.
    fn gen_name(&self, stem: &str, state: StateRef) -> Utf8PathBuf {
        let state = &self.states[state];
        if state.is_pseudo() {
            Utf8PathBuf::from(format!("_pseudo_{}", state.name))
        } else {
            // TODO avoid collisions in case we reuse extensions...
            Utf8PathBuf::from(stem).with_extension(&state.extensions[0])
        }
    }

    /// Concot a plan to carry out the requested build.
    ///
    /// This works by searching for a path through the available operations from the input state
    /// to the output state. If no such path exists in the operation graph, we return None.
    pub fn plan(&self, req: Request) -> Option<Plan> {
        // Find a path through the states.
        let path =
            self.find_path(req.start_state, req.end_state, &req.through)?;

        let mut steps: Vec<(OpRef, Utf8PathBuf)> = vec![];

        // Get the initial input filename and the stem to use to generate all intermediate filenames.
        let (stdin, start_file) = match req.start_file {
            Some(path) => (false, utils::relative_path(&path, &req.workdir)),
            None => (true, "stdin".into()),
        };
        let stem = start_file.file_stem().unwrap();

        // Generate filenames for each step.
        steps.extend(path.into_iter().map(|op| {
            let filename = self.gen_name(stem, self.ops[op].output);
            (op, filename)
        }));

        // If we have a specified output filename, use that instead of the generated one.
        let stdout = if let Some(end_file) = req.end_file {
            // TODO Can we just avoid generating the unused filename in the first place?
            let last_step = steps.last_mut().expect("no steps");
            last_step.1 = utils::relative_path(&end_file, &req.workdir);
            false
        } else {
            // Print to stdout if the last state is a real (non-pseudo) state.
            !self.states[req.end_state].is_pseudo()
        };

        Some(Plan {
            start: start_file,
            steps,
            workdir: req.workdir,
            stdin,
            stdout,
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

    /// The working directory to use when running a build.
    pub fn default_workdir(&self) -> Utf8PathBuf {
        format!(".{}", &self.name).into()
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
                self.states[op.input].name,
                self.states[op.output].name
            );
        }
    }
}

pub struct DriverBuilder {
    name: String,
    setups: PrimaryMap<SetupRef, Setup>,
    states: PrimaryMap<StateRef, State>,
    ops: PrimaryMap<OpRef, Operation>,
    rsrc_dir: Option<Utf8PathBuf>,
    rsrc_files: Option<FileData>,
}

impl DriverBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            setups: Default::default(),
            states: Default::default(),
            ops: Default::default(),
            rsrc_dir: None,
            rsrc_files: None,
        }
    }

    pub fn state(&mut self, name: &str, extensions: &[&str]) -> StateRef {
        self.states.push(State {
            name: name.to_string(),
            extensions: extensions.iter().map(|s| s.to_string()).collect(),
        })
    }

    fn add_op<T: run::EmitBuild + 'static>(
        &mut self,
        name: &str,
        setups: &[SetupRef],
        input: StateRef,
        output: StateRef,
        emit: T,
    ) -> OpRef {
        self.ops.push(Operation {
            name: name.into(),
            setups: setups.into(),
            input,
            output,
            emit: Box::new(emit),
        })
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

    pub fn op(
        &mut self,
        name: &str,
        setups: &[SetupRef],
        input: StateRef,
        output: StateRef,
        build: run::EmitBuildFn,
    ) -> OpRef {
        self.add_op(name, setups, input, output, build)
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
            input,
            output,
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

#[derive(Debug)]
pub struct Plan {
    /// The input to the first step.
    pub start: Utf8PathBuf,

    /// The chain of operations to run and each step's output file.
    pub steps: Vec<(OpRef, Utf8PathBuf)>,

    /// The directory that the build will happen in.
    pub workdir: Utf8PathBuf,

    /// Read the first input from stdin.
    pub stdin: bool,

    /// Write the final output to stdout.
    pub stdout: bool,
}

impl Plan {
    pub fn end(&self) -> &Utf8Path {
        match self.steps.last() {
            Some((_, path)) => path,
            None => &self.start,
        }
    }
}
