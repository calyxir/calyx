//! Statically infers the number of cycles for groups where the `done`
//! signal relies only on other `done` signals, and then inserts "static"
//! annotations with those inferred values. If there is an existing
//! annotation in a group that differs from an inferred value, this
//! pass will throw an error. If a group's `done` signal relies on signals
//! that are not only `done` signals, this pass will ignore that group.
use std::collections::HashMap;

use crate::errors::Error;
use crate::frontend::library::ast as lib;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};

pub struct InferStaticTiming<'a> {
    /// primitive name -> (go signal, done signal, latency)
    prim_latency_data: HashMap<&'a str, (&'a str, &'a str, u64)>,
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
        let prim_latency_data = [("std_reg", ("write_en", "done", 1))]
            .iter()
            .cloned()
            .collect();
        InferStaticTiming { prim_latency_data }
    }
}

/// Attempts to infer the number of cycles starting when
/// group[go] is high, and port is high. If inference is
/// not possible, returns None.
fn infer_latency<'a>(
    group: &ir::Group,
    latency_data: &HashMap<&'a str, (&'a str, &'a str, u64)>,
    comp: &ir::Component,
) -> Option<u64> {

    let mut writes: HashMap<String, String> = HashMap::new();
    let mut cell2latency: HashMap<String, u64> = HashMap::new();

    // Build a write dependency graph. An edge (`a`, `b`) exists in this graph
    // if port `a` writes to port `b`, `a` is a "done" port or constant, and `b`
    // is a "go" port. For example, if `a` and `b` are registers, these assignments
    // would correspond to such an edge:
    //
    // ```
    // b.write_en = a.done;
    // ```
    for asgn in &group.assignments {
        
        let src_parent_name: String = asgn
            .src
            .borrow()
            .get_parent_name()
            .to_string();
        
        let dst_parent_name: String = asgn
            .dst
            .borrow()
            .get_parent_name()
            .to_string();

        // If there are multiple signals writing to one "go" signal, we can't infer static timing.
        if writes.contains_key(&src_parent_name) {
            return None
        }

        match (&asgn.dst.borrow().parent, &asgn.src.borrow().parent) {

            (ir::PortParent::Cell(dst_cell), ir::PortParent::Cell(src_cell)) =>  {
                // A cell writes to a cell: to be added to the graph, the source needs to be a "done" port and the dest needs to be a "go" port.
                // If any other type of port else writes to a "go" port, we can't infer static timing.
                if let (ir::CellType::Primitive { name: dst_cell_prim_type, .. }, ir::CellType::Primitive {name: src_cell_prim_type, .. }) 
                    = (&dst_cell.upgrade().unwrap().borrow().prototype, &src_cell.upgrade().unwrap().borrow().prototype) {
                    let data_dst = latency_data.get(dst_cell_prim_type.as_ref());
                    let data_src = latency_data.get(src_cell_prim_type.as_ref());
                    if let (Some((go_dst, _, _)), Some((_, done_src, _))) = (data_dst, data_src) {

                        if asgn.dst.borrow().name == *go_dst && asgn.src.borrow().name == *done_src {
                            writes.insert(src_parent_name.to_string(), dst_parent_name.to_string());
                        }

                        if asgn.dst.borrow().name == *go_dst && asgn.src.borrow().name != *done_src {
                            return None
                        }
                    }
                }

                // A constant writes to a cell: to be added to the graph, the cell needs to be a "done" port.
                if let (ir::CellType::Primitive { name: dst_cell_prim_type, .. }, ir::CellType::Constant { val: _, width: _})
                    = (&dst_cell.upgrade().unwrap().borrow().prototype, &src_cell.upgrade().unwrap().borrow().prototype) {
                    let data = latency_data.get(dst_cell_prim_type.as_ref());
                    if let Some((go, _, _)) = data {
                        if asgn.dst.borrow().name == *go {
                            writes.insert(src_parent_name.to_string(), dst_parent_name.to_string());
                        }
                    }
                }
            }

            // Something is written to a group: to be added to the graph, this needs to be a "done" port.
            (ir::PortParent::Group(_), _) => {
                if asgn.dst.borrow().name == "done" {
                    writes.insert(src_parent_name, dst_parent_name);
                }

            }
            
            // If we encounter anything else, no need to add it to the graph.
            _ => ()
        }
    }

    // Starting at the constant written to a "go" port, walk the write dependency graph.
    // Populate cell2latency with the latency of a node of the graph, plus the latency
    // of the port that's written to the "go" signal of the node.
    let start: String = "_1_1".to_string();
    cell2latency.insert("_1_1".to_string(), 0);
    let mut curr_node = start;
    while curr_node != group.name.to_string() {
        let w = writes.get(&curr_node).unwrap().to_string();
        if w != group.name.to_string() {
            if let ir::CellType::Primitive { name, .. } = &comp.find_cell(&w).unwrap().borrow().prototype {
                let data = latency_data.get(name.as_ref());
                if let Some((_go, _done, latency)) = data {
                    cell2latency.insert(writes.get(&curr_node).unwrap().to_string(),*latency + cell2latency.get(&curr_node).unwrap());
                }
            }
        } else {
            // If the cell is the containing group, no need to add any more latency.
            cell2latency.insert(w,*cell2latency.get(&curr_node.to_string()).unwrap());
        }
        curr_node = writes.get(&curr_node).unwrap().to_string();
    }

    Some(*cell2latency.get(group.name.as_ref()).unwrap())
}

impl Visitor<()> for InferStaticTiming<'_> {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _c: &lib::LibrarySignatures,
    ) -> VisResult<()> {

        let mut latency_result: Option<u64> = None;
        for group in &comp.groups {
            for asgn in &group.borrow().assignments {
                let asgn_dst = asgn.dst.borrow();
                if asgn_dst.name == "done"
                    && asgn_dst.get_parent_name() == group.borrow().name
                {
                    if let Some(latency) = infer_latency(
                        &group.borrow(),
                        &self.prim_latency_data,
                        comp
                    ) {
                        let grp = group.borrow();
                        if let Some(curr_lat) = grp.attributes.get("static") {
                            if *curr_lat != latency {
                                return Err(
                                    Error::ImpossibleLatencyAnnotation(
                                        grp.name.to_string(),
                                        *curr_lat,
                                        latency,
                                    ),
                                );
                            }
                        }
                        latency_result = Some(latency);
                    } else {
                        latency_result = None;
                    }
                }
            }

            match latency_result {
                Some(res) => {
                    group
                        .borrow_mut()
                        .attributes
                        .insert("static".to_string(), res);
                }
                None => continue,
            }
        }
        Ok(Action::stop_default())
    }
}
