use crate::error::AdapterResult;
use dap::types::{Breakpoint, Source, SourceBreakpoint, StackFrame, Thread};
use interp::debugger::source::{NamedTag, SourceMap};
use interp::debugger::Debugger;
use std::collections::HashMap;
use std::path::PathBuf;

// TODO:
// 1) Update SourceMap
// 2) Replace Generic param in ProgramStatus
// 3) add getter for id, get_suffix
// 4) New fields for program: hashmap, bool

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
    ids: SourceMap,
}

impl MyAdapter {
    // Look at Rust File implementation
    // Pass in path, easier
    // Change to take in the file path
    // Create open file function
    pub fn new(path: &str) -> AdapterResult<Self> {
        Ok(MyAdapter {
            debugger: Debugger::from_file(
                &PathBuf::from(path),
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

    pub fn next_line(&mut self, _thread: i64) -> () {
        let status = self.debugger.step(1).unwrap();
        let map = status.get_status().clone();
        // Declare line number beforehand
        let mut line_number = 0;
        for id in map {
            let value = (self.ids.lookup((0, id.to_string()))).unwrap();
            let num = match value.parse() {
                Ok(int) => int,
                _ => 0,
            };
            line_number = num;
        }
        self.stack_frames[0].line = line_number;
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

// Hardcode mapping for now
fn create_map() -> SourceMap {
    let mut hashmap = HashMap::new();
    // Hardcode
    hashmap.insert(
        NamedTag::from((0, String::from("main.wr_reg0"))),
        String::from("9"),
    );
    hashmap.insert(
        NamedTag::from((0, String::from("main.wr_reg1"))),
        String::from("14"),
    );
    SourceMap::from(hashmap)
}
