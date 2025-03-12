use crate::error::AdapterResult;
use baa::BitVecOps;
use cider::debugger::commands::ParsedControlName;
use cider::debugger::source::structures::NewSourceMap;
use cider::debugger::{OwnedDebugger, StoppedReason};
use cider::flatten::flat_ir::base::{GlobalCellIdx, PortValue};
use dap::events::{Event, OutputEventBody, StoppedEventBody};
use dap::types::{
    self, Breakpoint, Scope, Source, SourceBreakpoint, StackFrame, Thread,
    Variable,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub struct MyAdapter {
    #[allow(dead_code)]
    debugger: OwnedDebugger,
    _break_count: Counter,
    thread_count: Counter,
    stack_count: Counter,
    breakpoints: HashSet<i64>,
    stack_frames: Vec<StackFrame>,
    threads: Vec<Thread>, // This field is a placeholder
    object_references: HashMap<i64, Vec<(String, PortValue)>>,
    source: String,
    ids: NewSourceMap,
    frames_to_cmpts: HashMap<i64, GlobalCellIdx>, //stores mapping from frame ids to component idx
}

impl MyAdapter {
    pub fn new(path: &str, std_path: PathBuf) -> AdapterResult<Self> {
        let (debugger, metadata) =
            OwnedDebugger::from_file(&PathBuf::from(path), &std_path).unwrap();
        Ok(MyAdapter {
            debugger,
            _break_count: Counter::new(),
            thread_count: Counter::new(),
            stack_count: Counter::new(),
            breakpoints: HashSet::new(),
            stack_frames: Vec::new(),
            threads: Vec::new(),
            object_references: HashMap::new(),
            source: path.to_string(),
            ids: metadata,
            frames_to_cmpts: HashMap::new(),
        })
    }
    /// function to deal with setting breakpoints and updating debugger accordingly
    pub fn handle_breakpoint(
        &mut self,
        path: Source,
        points: &Vec<SourceBreakpoint>,
    ) -> Vec<Breakpoint> {
        // helper method to get diffs since it yelled at me about borrows when i had it in main method
        fn calc_diffs(
            new_set: &HashSet<i64>,
            old_set: &HashSet<i64>,
        ) -> (HashSet<i64>, HashSet<i64>) {
            let to_set: HashSet<i64> =
                new_set.difference(old_set).copied().collect();
            let to_delete: HashSet<i64> =
                old_set.difference(new_set).copied().collect();
            (to_set, to_delete)
        }

        //check diffs
        let mut new_point_set = HashSet::new();
        for p in points {
            new_point_set.insert(p.line);
        }
        let (to_set, to_delete) = calc_diffs(&new_point_set, &self.breakpoints);

        //update adapter
        self.breakpoints.clear();

        let mut to_debugger_set: Vec<ParsedControlName> = vec![];
        let mut to_client: Vec<Breakpoint> = vec![];

        // iterate over points received in request
        for source_point in points {
            self.breakpoints.insert(source_point.line);
            let name = self.ids.lookup_line(source_point.line as u64);

            let breakpoint = make_breakpoint(
                Some(source_point.line),
                name.is_some(),
                Some(path.clone()),
                Some(source_point.line),
            );
            to_client.push(breakpoint);

            if let Some((component, group)) = name {
                if to_set.contains(&source_point.line) {
                    to_debugger_set.push(
                        ParsedControlName::from_comp_and_control(
                            component.clone(),
                            group.clone(),
                        ),
                    )
                }
            }
        }
        //send ones to set to debugger
        self.debugger.set_breakpoints(to_debugger_set);
        //delete from debugger
        self.delete_breakpoints(to_delete);

        //return list of created points to client
        to_client
    }
    /// handles deleting breakpoints in the debugger
    fn delete_breakpoints(&mut self, to_delete: HashSet<i64>) {
        let mut to_debugger: Vec<ParsedControlName> = vec![];
        for point in to_delete {
            let name = self.ids.lookup_line(point as u64);
            if let Some((component, group)) = name {
                to_debugger.push(ParsedControlName::from_comp_and_control(
                    component.clone(),
                    group.clone(),
                ))
            }
        }
        self.debugger.delete_breakpoints(to_debugger);
    }

    /// Creates a thread using the parameter name.
    pub fn create_thread(&mut self, name: String) -> Thread {
        //how do we attach the thread to the program
        let thread = Thread {
            id: self.thread_count.increment(),
            name,
        };
        self.threads.push(thread.clone());
        thread
    }

    /// Clone threads
    pub fn clone_threads(&self) -> Vec<Thread> {
        self.threads.clone()
    }

    /// returns all frames (components) in program
    pub fn get_stack(&mut self) -> Vec<StackFrame> {
        if self.stack_frames.is_empty() {
            self.create_stack();
        }
        self.stack_frames.clone()
    }

    /// creates call stack where each frame is a component. Adds frames to current
    /// call stack
    fn create_stack(&mut self) {
        let components = self.debugger.get_components();
        //turn the names into stack frames, ignore lines for right now
        for (idx, comp) in components {
            let frame = StackFrame {
                id: self.stack_count.increment(),
                // Maybe automate the name in the future?
                name: String::from(comp),
                source: Some(Source {
                    name: None,
                    path: Some(self.source.clone()),
                    source_reference: None,
                    presentation_hint: None,
                    origin: None,
                    sources: None,
                    adapter_data: None,
                    checksums: None,
                }),
                line: 1, // need to get this to be line component starts on
                column: 0,
                end_line: None,
                end_column: None,
                can_restart: None,
                instruction_pointer_reference: None,
                module_id: None,
                presentation_hint: None,
            };
            self.frames_to_cmpts.insert(frame.id, idx);
            self.stack_frames.push(frame);
        }
    }

    pub fn next_line(&mut self, _thread: i64) -> bool {
        self.object_references.clear();
        //return a more informative enum
        // Step through once
        let status = self.debugger.step(1).unwrap(); //need to unwrap a different way

        // Check if done:
        if status.get_done() {
            // Give bool to exit the debugger
            true
        } else {
            let map = status.get_status();
            let mut line_number = 0;
            // Implemented for loop for when more than 1 group is running,
            // the code for now goes to the line of the last group running in the map, should deal
            // with this in the future for when groups run in parallel.
            for id in map {
                let value = self.ids.lookup(id).unwrap().start_line;
                line_number = value;
            }
            // Set line of the stack frame and tell debugger we're not finished.
            self.stack_frames[0].line = line_number as i64;
            false
        }
    }

    //display ports of each cell
    pub fn get_variables(&self, var_ref: i64) -> Vec<Variable> {
        let ports = self.object_references.get(&var_ref);
        match ports {
            None => Vec::default(),
            Some(p) => {
                let out: Vec<Variable> = p
                    .iter()
                    .map(|(nam, val)| {
                        let valu = val
                            .as_option()
                            .map(|x| x.val().to_u64().unwrap())
                            .unwrap_or_default();
                        Variable {
                            name: String::from(nam),
                            value: valu.to_string(),
                            type_field: None,
                            presentation_hint: None,
                            evaluate_name: None,
                            variables_reference: 0,
                            named_variables: None,
                            indexed_variables: None,
                            memory_reference: None,
                        }
                    })
                    .collect();
                out
            }
        }
    }
    // return cells in calyx context
    // todo: return only cells in current stack frame (component)
    pub fn get_scopes(&mut self, frame: i64) -> Vec<Scope> {
        let mut out_vec = vec![];
        let component = self.frames_to_cmpts[&frame];
        let cell_names = self.debugger.get_comp_cells(component);
        let mut var_ref_count = 1;
        for (name, ports) in cell_names {
            self.object_references.insert(var_ref_count, ports);
            let scope = Scope {
                name,
                presentation_hint: Some(
                    dap::types::ScopePresentationhint::Locals,
                ),
                variables_reference: var_ref_count,
                named_variables: None,
                indexed_variables: None,
                expensive: false,
                source: None,
                line: None,
                column: None,
                end_line: None,
                end_column: None,
            };
            var_ref_count += 1;
            out_vec.push(scope);
        }
        out_vec
    }

    pub fn on_pause(&mut self) {
        //self.debugger.pause();
        self.object_references.clear();
    }

    pub fn on_continue(&mut self, thread_id: i64) -> Event {
        dbg!("continue - adapter");
        let result = self.debugger.cont();
        match result {
            // honestly not sure if this is right behavior, still unsure what an output event IS lol.
            Err(e) => Event::Output(OutputEventBody {
                category: Some(types::OutputEventCategory::Stderr),
                output: e.to_string(),
                group: Some(types::OutputEventGroup::Start),
                variables_reference: None,
                source: None,
                line: None,
                column: None,
                data: None,
            }),
            Ok(reason) => match reason {
                StoppedReason::Done => Event::Terminated(None),
                StoppedReason::Breakpoint(names) => {
                    let bp_lines: Vec<i64> = names
                        .into_iter()
                        .map(|x| self.ids.lookup(&x).unwrap().start_line as i64)
                        .collect();
                    dbg!(&bp_lines);
                    //in map add adjusting stack frame lines
                    Event::Stopped(StoppedEventBody {
                        reason: types::StoppedEventReason::Breakpoint,
                        description: Some(String::from("hit breakpoint")),
                        thread_id: Some(thread_id),
                        preserve_focus_hint: None,
                        all_threads_stopped: Some(true),
                        text: None,
                        hit_breakpoint_ids: Some(bp_lines),
                    })
                }
                StoppedReason::PauseReq => Event::Stopped(StoppedEventBody {
                    reason: types::StoppedEventReason::Pause,
                    description: Some(String::from("Paused")),
                    thread_id: Some(thread_id),
                    preserve_focus_hint: None,
                    all_threads_stopped: Some(true),
                    text: None,
                    hit_breakpoint_ids: None,
                }),
            },
        }
    }
}

/// Simple struct used to keep an index of the breakpoints used.
pub struct Counter {
    value: i64,
}

impl Counter {
    pub fn new() -> Self {
        Counter { value: 0 }
    }

    /// Increment the counter by 1 and return the OLD value
    pub fn increment(&mut self) -> i64 {
        let out = self.value;
        self.value += 1;
        out
    }
}

/// Returns a Breakpoint object.
///
/// This function takes in relevant fields in Breakpoint that are used
/// by the adapter. This is subject to change.
pub fn make_breakpoint(
    id: Option<i64>,
    verified: bool,
    source: Option<Source>,
    line: Option<i64>,
) -> Breakpoint {
    if verified {
        Breakpoint {
            id,
            verified,
            message: None,
            source,
            line,
            column: None,
            end_line: None,
            end_column: None,
            instruction_reference: None,
            offset: None,
        }
    } else {
        Breakpoint {
            id,
            verified,
            message: Some(String::from("Invalid placement for breakpoint")),
            source,
            line,
            column: None,
            end_line: None,
            end_column: None,
            instruction_reference: None,
            offset: None,
        }
    }
}
