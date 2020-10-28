use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use crate::frontend::library::ast as lib;
use crate::guard;
use crate::ir;
use crate::ir::analysis::GraphAnalysis;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};

pub struct InferStaticTiming<'a> {
    /// primitive name -> (go signal, done signal, latency)
    prim_latency_data: HashMap<&'a str, (&'a str, &'a str, u64)>
}

impl Named for InferStaticTiming<'_> {
    fn name() -> &'static str {
        "infer-static-timing"
    }

    fn description() -> &'static str {
        "infers and annotates static timing for groups when possible"
    }
}

impl Default for InferStaticTiming<'_> {
    fn default() -> Self {
        let prim_latency_data = [
            ("std_reg", ("write_en", "done", 1))
        ]
        .iter()
        .cloned()
        .collect();
        InferStaticTiming { prim_latency_data }
    }
}

/// Attempts to infer the number of cycles starting when
/// group[go] is high, and port is high.
fn infer_latency<'a>(port: &ir::Port,
                 group: &ir::Group,
                 analysis: &GraphAnalysis,
                 cells: &Vec<Rc<RefCell<ir::Cell>>>,
                 latency_data: &HashMap<&'a str, (&'a str, &'a str, u64)>) -> Option<u64> {
    for write in analysis.writes_to(port) {
        let cell_type: ir::CellType = &cells
            .iter()
            .find(|c| c.borrow().name == write.borrow().get_parent_name())
            .unwrap()
            .borrow()
            .prototype
        println!("write: {:?}", write);
        if let ir::CellType::Primitive{name, param_binding} = &cells.iter().find(|c| c.borrow().name == write.borrow().get_parent_name()).unwrap().borrow().prototype {
            println!("reg type: {:?}", name);
            let (go, done, latency) = latency_data.get(name.to_string().as_str()).unwrap();
            println!("{}, {}, {}", go, done, latency);

            // if write is to a "done" port, then return latency + infer_latency("go" port)
            if write.borrow().name == done {
                // Find port with name == parent, signal == g
                let p: &ir::Port = &group.assignments.iter().find(|a| a.dst.borrow().name.to_string() == *go && a.dst.borrow().get_parent_name() == write.borrow().get_parent_name()).unwrap().dst.borrow();
                return Some(latency + infer_latency(p, group, analysis, cells, latency_data).unwrap())
            }
        }

        println!("found done")
    }
    return Some(0);
    /*
    for write in analysis.writes_to(port) {
        // 1? return 1
        // "done" port? prim_latency_data[port] + infer_latency("go" port)
        // else: return None
    }
    */
}


impl Visitor for InferStaticTiming<'_> {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _c: &lib::LibrarySignatures,
    ) -> VisResult {

        let analysis = GraphAnalysis::from(&comp);

        for group in &comp.groups {
            for asgn in &group.borrow().assignments {
                if asgn.dst.borrow().name == "done" && asgn.dst.borrow().get_parent_name() == group.borrow().name {
                    println!("name: {}", asgn.dst.borrow().get_parent_name());
                    println!("latency: {}", infer_latency(&asgn.dst.borrow(), &group.borrow(), &analysis, &comp.cells, &self.prim_latency_data).unwrap());

                    /*
                    for read in analysis.writes_to(&asgn.dst.borrow()) {
                        println!("read: {:?}", read);
                    }
                    */
                }
            }
        }
        Ok(Action::Stop)
    }
}