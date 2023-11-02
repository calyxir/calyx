use dap::types::{Breakpoint, Source, SourceBreakpoint};
use std::fs::File;
pub struct MyAdapter {
    #[allow(dead_code)]
    file: File,
    breakpoints: Vec<(Source, i64)>,
    break_count: Counter,
}

impl MyAdapter {
    pub fn new(file: File) -> Self {
        MyAdapter {
            file,
            breakpoints: Vec::new(),
            break_count: Counter::new(),
        }
    }

    //Set breakpoints for adapter
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
            let breakpoint = Breakpoint {
                id: self.break_count.increment().into(),
                verified: true,
                message: None,
                source: Some(path.clone()),
                line: None,
                column: None,
                end_line: None,
                end_column: None,
                instruction_reference: None,
                offset: None,
            };

            out_vec.push(breakpoint);
        }

        out_vec
    }

    pub fn step(&self) {}

    pub fn cont(&self) {}
}

//Simple struct used to keep an index of the breakpoints used.
pub struct Counter {
    value: i64,
}

impl Counter {
    pub fn new() -> Self {
        Counter { value: 0 }
    }
    //Inc the counter, return the OLD value
    pub fn increment(&mut self) -> i64 {
        let out = self.value;
        self.value += 1;
        out
    }
}
