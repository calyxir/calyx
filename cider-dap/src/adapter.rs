use crate::error::AdapterResult;
use dap::types::{Breakpoint, Source, SourceBreakpoint, StackFrame, Thread};
use interp::debugger::source::structures::NewSourceMap;
use interp::debugger::Debugger;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct MyAdapter {
    #[allow(dead_code)]
    debugger: Debugger,
    break_count: Counter,
    thread_count: Counter,
    stack_count: Counter,
    breakpoints: Vec<(Source, i64)>, // This field is a placeholder
    stack_frames: Vec<StackFrame>,   // This field is a placeholder
    threads: Vec<Thread>,            // This field is a placeholder
    source: String,
    ids: NewSourceMap,
}

// New metadata in interp/debugger

impl MyAdapter {
    // Look at Rust File implementation
    // Pass in path, easier
    // Change to take in the file path
    // Create open file function
    pub fn new(path: &str) -> AdapterResult<Self> {
        Ok(MyAdapter {
            debugger: Debugger::from_file(
                &PathBuf::from(path),
                // Hard code for now, change path as necessary
                &PathBuf::from("/home/elias/calyx/calyx-stdlib"),
            )
            .unwrap(),
            break_count: Counter::new(),
            thread_count: Counter::new(),
            stack_count: Counter::new(),
            breakpoints: Vec::new(),
            stack_frames: Vec::new(),
            threads: Vec::new(),
            source: path.to_string(),
            ids: create_map(),
        })
    }
    ///Set breakpoints for adapter
    pub fn set_breakpoint(
        &mut self,
        path: Source,
        source: &Vec<SourceBreakpoint>,
    ) -> Vec<Breakpoint> {
        //Keep all the new breakpoints made
        let mut out_vec: Vec<Breakpoint> = vec![];

        //Loop over all breakpoints
        for source_point in source {
            self.breakpoints.push((path.clone(), source_point.line));
            //Create new Breakpoint instance
            let breakpoint = make_breakpoint(
                self.break_count.increment().into(),
                true,
                Some(path.clone()),
                Some(source_point.line),
            );

            out_vec.push(breakpoint);
        }

        out_vec
    }

    ///Creates a thread using the parameter name.
    pub fn create_thread(&mut self, name: String) -> Thread {
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

    //Returns a dummy stack frame, set to change.
    pub fn create_stack(&mut self) -> Vec<StackFrame> {
        let frame = StackFrame {
            id: self.stack_count.increment(),
            // TODO: edit name field
            name: String::from("Hi"),
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
            line: 1,
            column: 0,
            end_line: None,
            end_column: None,
            can_restart: None,
            instruction_pointer_reference: None,
            module_id: None,
            presentation_hint: None,
        };
        self.stack_frames.push(frame);
        // Return all stack frames
        self.stack_frames.clone()
    }

    pub fn clone_stack(&self) -> Vec<StackFrame> {
        self.stack_frames.clone()
    }

    pub fn next_line(&mut self, _thread: i64) -> bool {
        let status = self.debugger.step(1).unwrap();

        // Check if done:
        if status.get_done().clone() {
            true
        } else {
            let map = status.get_status().clone();
            // Declare line number beforehand
            let mut line_number = 0;
            // Return -1 should a lookup not be found. This really shouldn't
            // happen though
            for id in map {
                let value = match self.ids.lookup(id.to_string()) {
                    Some(val) => val,
                    None => &(-1),
                };
                line_number = value.clone();

                // Only get first Id for now
                break;
            }
            self.stack_frames[0].line = line_number;
            false
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
}

// Hardcode mapping for now, this mapping is for reg_seq.futil
fn create_map() -> NewSourceMap {
    let mut hashmap = HashMap::new();
    // Hardcode
    hashmap.insert(String::from("wr_reg0"), 10);
    hashmap.insert(String::from("wr_reg1"), 15);
    NewSourceMap::from(hashmap)
}
