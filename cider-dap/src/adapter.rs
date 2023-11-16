use dap::types::{Breakpoint, Source, SourceBreakpoint, Thread};
use std::fs::File;
pub struct MyAdapter {
    #[allow(dead_code)]
    file: File,
    breakpoints: Vec<(Source, i64)>, //This field is a placeholder
    break_count: Counter,
    threads: Vec<Thread>, //This field is a placeholder
}

impl MyAdapter {
    pub fn new(file: File) -> Self {
        MyAdapter {
            file,
            breakpoints: Vec::new(),
            break_count: Counter::new(),
            threads: Vec::new(),
        }
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
            );

            out_vec.push(breakpoint);
        }

        out_vec
    }

    /// Clone threads
    pub fn clone_threads(&self) -> Vec<Thread> {
        self.threads.clone()
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
) -> Breakpoint {
    Breakpoint {
        id,
        verified,
        message: None,
        source,
        line: None,
        column: None,
        end_line: None,
        end_column: None,
        instruction_reference: None,
        offset: None,
    }
}
