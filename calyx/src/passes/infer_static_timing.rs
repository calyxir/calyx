use std::collections::HashMap;

use crate::frontend::library::ast as lib;
use crate::guard;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};

#[derive(Default)]
pub struct InferStaticTiming {}

impl Named for InferStaticTiming {
    fn name() -> &'static str {
        "infer-static-timing"
    }

    fn description() -> &'static str {
        "infers and annotates static timing for groups when possible"
    }
}

fn infer_cycles(dst_name: String, assignments: &Vec<ir::Assignment>) -> u64 {
    for assign in assignments {
        if assign.dst.borrow().get_parent_name() == dst_name {
            match &assign.guard {
                ir::Guard::Port(port) => {
                    if port.borrow().name == "done" {
                        return 1 + infer_cycles(port.borrow().get_parent_name().to_string(), assignments);
                    }
                }
                _ => continue
            }
        }
    }
    return 1;
}

impl Visitor for InferStaticTiming {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _c: &lib::LibrarySignatures,
    ) -> VisResult {

        let mut attrs: HashMap<String, u64> = HashMap::new();
        for group in &comp.groups {
            let assignments = &group.borrow().assignments;
            for assign in assignments {
                if assign.dst.borrow().is_hole() && assign.dst.borrow().name == "done" {
                    if assign.src.borrow().name == "done" {
                        attrs.insert(
                            group.borrow().name.to_string(),
                            infer_cycles(assign.src.borrow().get_parent_name().to_string(), assignments)
                        );
                    }
                }
            }
        }
        
        for group in &comp.groups {
            let group_name = group.borrow().name.to_string().clone();
            if attrs.contains_key(&group_name) {
                    group.borrow_mut().attributes.insert(
                        "static".to_string(),
                        *attrs.get(&group_name).unwrap()
                    );
            }
        }

        Ok(Action::Stop)
    }
}
