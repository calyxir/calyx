//! Statically infers the number of cycles for groups where the `done`
//! signal relies only on other `done` signals, and then inserts "static"
//! annotations with those inferred values. If there is an existing
//! annotation in a group that differs from an inferred value, this
//! pass will throw an error. If a group's `done` signal relies on signals
//! that are not only `done` signals, this pass will ignore that group.
use std::collections::HashMap;

use crate::analysis::{GraphAnalysis, ReadWriteSet};
use crate::errors::Error;
use crate::frontend::library::ast as lib;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::RRC;
use std::rc::Rc;

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

/// Return true if the edge (`src`, `dst`) meet one these criteria, and false otherwise:
///   - `src` is an "out" port of a constant, and `dst` is a "go" port
///   - `src` is a "done" port, and `dst` is a "go" port
///   - `src` is a "done" port, and `dst` is the "done" port of a group
fn mem_wrt_dep_graph<'a>(
    src: &ir::Port,
    dst: &ir::Port,
    latency_data: &HashMap<&'a str, (&'a str, &'a str, u64)>,
) -> bool {
    match (&src.parent, &dst.parent) {
        (ir::PortParent::Cell(src_cell), ir::PortParent::Cell(dst_cell)) => {
            if let (
                ir::CellType::Primitive {
                    name: dst_cell_prim_type,
                    ..
                },
                ir::CellType::Primitive {
                    name: src_cell_prim_type,
                    ..
                },
            ) = (
                &dst_cell.upgrade().unwrap().borrow().prototype,
                &src_cell.upgrade().unwrap().borrow().prototype,
            ) {
                let data_dst = latency_data.get(dst_cell_prim_type.as_ref());
                let data_src = latency_data.get(src_cell_prim_type.as_ref());
                if let (Some((go_dst, _, _)), Some((_, done_src, _))) =
                    (data_dst, data_src)
                {
                    if dst.name == *go_dst && src.name == *done_src {
                        return true;
                    }
                }
            }

            // A constant writes to a cell: to be added to the graph, the cell needs to be a "done" port.
            if let (
                ir::CellType::Primitive {
                    name: dst_cell_prim_type,
                    ..
                },
                ir::CellType::Constant { .. },
            ) = (
                &dst_cell.upgrade().unwrap().borrow().prototype,
                &src_cell.upgrade().unwrap().borrow().prototype,
            ) {
                let data = latency_data.get(dst_cell_prim_type.as_ref());
                if let Some((go, _, _)) = data {
                    if dst.name == *go {
                        return true;
                    }
                }
            }

            return false;
        }

        // Something is written to a group: to be added to the graph, this needs to be a "done" port.
        (_, ir::PortParent::Group(_)) => {
            if dst.name == "done" {
                return true;
            } else {
                return false;
            }
        }

        // If we encounter anything else, no need to add it to the graph.
        _ => return false,
    }
}

/// Return a Vec of edges (`a`, `b`), where `a` is a "go" port and `b`
/// is a "done" port, and `a` and `b` have the same parent cell.
fn find_go_done_edges<'a>(
    group: &ir::Group,
    latency_data: &HashMap<&'a str, (&'a str, &'a str, u64)>,
) -> Vec<(RRC<ir::Port>, RRC<ir::Port>)> {
    let rw_set = ReadWriteSet::uses(&group.assignments);
    let mut go_done_edges: Vec<(RRC<ir::Port>, RRC<ir::Port>)> = Vec::new();
    for cell_ref in rw_set {
        let cell = cell_ref.borrow();
        if let ir::CellType::Primitive {
            name: cell_type, ..
        } = &cell.prototype
        {
            let (go, done, _) = latency_data.get(cell_type.as_ref()).unwrap();
            let go_port = &cell.ports.iter().find(|p| p.borrow().name == *go);
            let done_port =
                &cell.ports.iter().find(|p| p.borrow().name == *done);

            if let (Some(g), Some(d)) = (go_port, done_port) {
                go_done_edges.push((Rc::clone(&g), Rc::clone(&d)));
            }
        }
    }
    return go_done_edges;
}

/// Attempts to infer the number of cycles starting when
/// group[go] is high, and port is high. If inference is
/// not possible, returns None.
fn infer_latency<'a>(
    group: &ir::Group,
    latency_data: &HashMap<&'a str, (&'a str, &'a str, u64)>,
) -> Option<u64> {
    // Creates a write dependency graph, which contains an edge (`a`, `b`) if:
    //   - `a` is a "done" port, and writes to `b`, which is a "go" port
    //   - `a` is a "done" port, and writes to `b`, which is the "done" port of this group
    //   - `a` is an "out" port, and is a constant, and writes to `b`, a "go" port
    //   - `a` is a "go" port, and `b` is a "done" port, and `a` and `b` share a parent cell
    // Nodes that are not part of any edges that meet these criteria are excluded.
    //
    // For example, this group:
    // ```
    // group g1 {
    //   a.in = 32'd1;
    //   a.write_en = 1'd1;
    //   g1[done] = a.done;
    // }
    // ```
    // corresponds to this graph:
    // ```
    // constant(1) -> a.write_en
    // a.write_en -> a.done
    // a.done -> g1[done]
    // ```
    let go_done_edges = find_go_done_edges(group, latency_data);
    let graph = GraphAnalysis::from(group)
        .edge_induced_subgraph(|src, dst| {
            mem_wrt_dep_graph(src, dst, latency_data)
        })
        .add_edges(&go_done_edges)
        .remove_isolated_vertices();

    let mut tsort = graph.toposort();
    let start = tsort.next().unwrap();
    let finish = tsort.last().unwrap();

    let paths = graph.paths(&*start.borrow(), &*finish.borrow());
    // If there are no paths, give up.
    if paths.len() == 0 {
        return None;
    }
    let first_path = paths.get(0).unwrap();

    // Sum the latencies of each primitive along the path.
    let mut latency_sum = 0;
    for port in first_path {
        if let ir::PortParent::Cell(cell) = &port.borrow().parent {
            if let ir::CellType::Primitive { name, .. } =
                &cell.upgrade().unwrap().borrow().prototype
            {
                let (go, _, latency) = latency_data.get(name.as_ref()).unwrap();
                if port.borrow().name == go {
                    latency_sum += latency;
                }
            }
        }
    }

    Some(latency_sum)
}

impl Visitor<()> for InferStaticTiming<'_> {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _c: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        let mut latency_result: Option<u64>;
        for group in &comp.groups {
            if let Some(latency) =
                infer_latency(&group.borrow(), &self.prim_latency_data)
            {
                let grp = group.borrow();
                if let Some(curr_lat) = grp.attributes.get("static") {
                    if *curr_lat != latency {
                        return Err(Error::ImpossibleLatencyAnnotation(
                            grp.name.to_string(),
                            *curr_lat,
                            latency,
                        ));
                    }
                }
                latency_result = Some(latency);
            } else {
                latency_result = None;
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
