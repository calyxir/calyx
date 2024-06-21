use super::{OpRef, Operation, Request, Setup, SetupRef, State, StateRef};
use crate::{config, run, script, utils};
use camino::{Utf8Path, Utf8PathBuf};
use cranelift_entity::PrimaryMap;
use rand::distributions::{Alphanumeric, DistString};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    error::Error,
    ffi::OsStr,
    fmt::Display,
};

type FileData = HashMap<&'static str, &'static [u8]>;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
enum Node {
    State(StateRef),
    Op(OpRef),
}

/// A Driver encapsulates a set of States and the Operations that can transform between them. It
/// contains all the machinery to perform builds in a given ecosystem.
pub struct Driver {
    pub name: String,
    pub setups: PrimaryMap<SetupRef, Setup>,
    pub states: PrimaryMap<StateRef, State>,
    pub ops: PrimaryMap<OpRef, Operation>,
    pub rsrc_dir: Option<Utf8PathBuf>,
    pub rsrc_files: Option<FileData>,
    /// Maps from a node to a tuple (nodes with edge into node, nodes with edge from node)
    op_graph: HashMap<Node, (Vec<Node>, Vec<Node>)>,
}

impl Driver {
    fn merge_plans(
        p1: Option<Vec<(OpRef, Vec<StateRef>)>>,
        p2: Option<Vec<(OpRef, Vec<StateRef>)>>,
    ) -> Option<Vec<(OpRef, Vec<StateRef>)>> {
        match (p1, p2) {
            (Some(p1), Some(p2)) => {
                let mut res = p1.clone();
                for (o, used) in p2 {
                    if let Some(i) = res.iter_mut().find(|(p, _)| *p == o) {
                        let used = used
                            .iter()
                            .filter(|r| !i.1.contains(r))
                            .collect::<Vec<_>>();
                        i.1.extend(used);
                    } else {
                        res.push((o, used));
                    }
                }
                Some(res)
            }
            _ => None,
        }
    }

    fn find_tree_from_op(
        &self,
        from: OpRef,
        start: &[StateRef],
        last: Node,
    ) -> Option<Vec<(OpRef, Vec<StateRef>)>> {
        fn dfs(
            driver: &Driver,
            from: Node,
            last: Node,
            start: &[StateRef],
            visited: &mut HashMap<Node, u32>,
        ) -> Option<Vec<(OpRef, Vec<StateRef>)>> {
            // update visiteds
            visited.entry(from).or_insert(0);
            if visited[&from] > 0 {
                return None;
            }
            visited.insert(from, visited[&from] + 1);

            // base case of just a single state
            if let Node::State(state_ref) = from {
                if start.contains(&state_ref) {
                    return Some(vec![]);
                }
            }

            match from {
                // in the case of a state we just need one of the ops to work
                Node::State(_) => {
                    for &op in &driver.op_graph[&from].0 {
                        if let Some(plan) =
                            dfs(driver, op, from, start, visited)
                        {
                            return Some(plan);
                        }
                    }
                    None
                }
                // in the case of an op we need to get plans from all inputs
                Node::Op(op) => {
                    let mut res = vec![];
                    if let Node::State(state) = last {
                        res.push((op, vec![state]));
                    } else {
                        panic!("invariant violated: all ops should only have edges to states");
                    }
                    let mut res = Some(res);
                    for &state in &driver.op_graph[&from].0 {
                        let plan = dfs(driver, state, from, start, visited);
                        res = Driver::merge_plans(res, plan);
                        if res.is_none() {
                            return res;
                        }
                    }
                    res
                }
            }
        }
        dfs(self, Node::Op(from), last, start, &mut HashMap::new()).map(
            |mut v| {
                v.reverse();
                v
            },
        )
    }

    fn find_path_generating_state(
        &self,
        from: OpRef,
        target: StateRef,
        start: &[StateRef],
    ) -> Option<Vec<(OpRef, Vec<StateRef>)>> {
        let mut visited = HashSet::new();
        let mut q = VecDeque::new();
        let mut par: HashMap<Node, Node> = HashMap::new();
        q.push_back(Node::Op(from));
        visited.insert(Node::Op(from));
        while !q.is_empty() {
            let mut v = q.pop_front().unwrap();
            if let Node::Op(op) = v {
                if self.ops[op].output.contains(&target) {
                    // retrieve the solution
                    let mut res = Some(vec![]);
                    loop {
                        if let Some(&n) = par.get(&v) {
                            if let Node::Op(t) = n {
                                let plan = self
                                    .find_tree_from_op(t, start, v)
                                    .map(|mut v| {
                                        v.push((op, vec![target]));
                                        v
                                    });
                                res = Self::merge_plans(res, plan);
                            }
                            v = n;
                        } else {
                            let op = match v {
                                    Node::Op(op) => op,
                                    _ => panic!("invariant violated: all ops should only have edges to states"),
                            };
                            let plan = self
                                .find_tree_from_op(
                                    op,
                                    start,
                                    Node::State(target),
                                )
                                .map(|mut v| {
                                    v.push((op, vec![target]));
                                    v
                                });
                            return Self::merge_plans(res, plan);
                        }
                    }
                }
            }
            for &u in &self.op_graph[&v].1 {
                if let Node::Op(o) = u {
                    if self.find_tree_from_op(o, start, v).is_none() {
                        continue;
                    }
                }
                if !visited.contains(&u) {
                    par.insert(u, v);
                    visited.insert(u);
                    q.push_back(u);
                }
            }
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
        let mut plan = Some(vec![]);
        for i in 0..through.len() {
            let comp =
                self.find_path_generating_state(through[i], end[i], start);
            plan = Self::merge_plans(plan, comp);
        }
        for &target in end.iter().skip(through.len()) {
            let mut path_found = false;
            for &n in &self.op_graph[&Node::State(target)].0 {
                if let Node::Op(op) = n {
                    if let Some(comp) =
                        self.find_path_generating_state(op, target, start)
                    {
                        plan = Self::merge_plans(plan, Some(comp));
                        path_found = true;
                        break;
                    }
                } else {
                    panic!("invariant violated: all ops should only have edges to states");
                }
            }
            if !path_found {
                return None;
            }
        }
        plan
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
        let mut op_graph = HashMap::new();
        for (state_ref, _) in &self.states {
            op_graph.insert(Node::State(state_ref), (Vec::new(), Vec::new()));
        }
        for (op_ref, op) in &self.ops {
            op_graph.insert(
                Node::Op(op_ref),
                (
                    op.input
                        .iter()
                        .map(|&state_ref| Node::State(state_ref))
                        .collect(),
                    op.output
                        .iter()
                        .map(|&state_ref| Node::State(state_ref))
                        .collect(),
                ),
            );
            for &state_ref in &op.input {
                if let Some(v) = op_graph.get_mut(&Node::State(state_ref)) {
                    v.1.push(Node::Op(op_ref))
                }
            }
            for &state_ref in &op.output {
                if let Some(v) = op_graph.get_mut(&Node::State(state_ref)) {
                    v.0.push(Node::Op(op_ref));
                }
            }
        }

        //TODO: validate the built graph

        Driver {
            name: self.name,
            setups: self.setups,
            states: self.states,
            ops: self.ops,
            rsrc_dir: self.rsrc_dir,
            rsrc_files: self.rsrc_files,
            op_graph,
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
