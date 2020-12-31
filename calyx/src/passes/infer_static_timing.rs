use std::collections::HashMap;

use crate::analysis::{GraphAnalysis, ReadWriteSet};
use crate::errors::Error;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::RRC;
use crate::ir::{self, LibrarySignatures};
use std::rc::Rc;

/// Infer "static" annotation for groups.
///
/// Statically infers the number of cycles for groups where the `done`
/// signal relies only on other `done` signals, and then inserts "static"
/// annotations with those inferred values. If there is an existing
/// annotation in a group that differs from an inferred value, this
/// pass will throw an error. If a group's `done` signal relies on signals
/// that are not only `done` signals, this pass will ignore that group.
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
        let prim_latency_data = [
            ("std_reg", ("write_en", "done", 1)),
            ("std_mem_d1", ("write_en", "done", 1)),
            ("std_mem_d1_ext", ("write_en", "done", 1)),
            ("std_mem_d2", ("write_en", "done", 1)),
            ("std_mem_d2_ext", ("write_en", "done", 1)),
            ("std_mem_d3", ("write_en", "done", 1)),
            ("std_mem_d3_ext", ("write_en", "done", 1)),
            ("std_mem_d4", ("write_en", "done", 1)),
            ("std_mem_d4_ext", ("write_en", "done", 1)),
        ]
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
                &dst_cell.upgrade().borrow().prototype,
                &src_cell.upgrade().borrow().prototype,
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
                &dst_cell.upgrade().borrow().prototype,
                &src_cell.upgrade().borrow().prototype,
            ) {
                let data = latency_data.get(dst_cell_prim_type.as_ref());
                if let Some((go, _, _)) = data {
                    if dst.name == *go {
                        return true;
                    }
                }
            }

            false
        }

        // Something is written to a group: to be added to the graph, this needs to be a "done" port.
        (_, ir::PortParent::Group(_)) => dst.name == "done",

        // If we encounter anything else, no need to add it to the graph.
        _ => false,
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
            if let Some((go, done, _)) = latency_data.get(cell_type.as_ref()) {
                let go_port =
                    &cell.ports.iter().find(|p| p.borrow().name == *go);
                let done_port =
                    &cell.ports.iter().find(|p| p.borrow().name == *done);

                if let (Some(g), Some(d)) = (go_port, done_port) {
                    go_done_edges.push((Rc::clone(&g), Rc::clone(&d)));
                }
            }
        }
    }
    go_done_edges
}

/// Returns true if `port` is a "done" port, and we know the latency data
/// about `port`, or is a constant.
fn is_done_port_or_const<'a>(
    port: &ir::Port,
    latency_data: &HashMap<&'a str, (&'a str, &'a str, u64)>,
) -> bool {
    if let ir::PortParent::Cell(cell) = &port.parent {
        if let ir::CellType::Primitive {
            name: cell_type, ..
        } = &cell.upgrade().borrow().prototype
        {
            if let Some((_, done, _)) = latency_data.get(cell_type.as_ref()) {
                if port.name == *done {
                    return true;
                }
            }
        }

        if let ir::CellType::Constant { val, .. } =
            &cell.upgrade().borrow().prototype
        {
            if *val > 0 {
                return true;
            }
        }
    }
    false
}

/// Returns true if `graph` contains writes to "done" ports
/// that could have dynamic latencies, false otherwise.
fn contains_dyn_writes<'a>(
    graph: GraphAnalysis,
    latency_data: &HashMap<&'a str, (&'a str, &'a str, u64)>,
) -> bool {
    for port in &graph.ports() {
        match &port.borrow().parent {
            ir::PortParent::Cell(cell) => {
                if let ir::CellType::Primitive {
                    name: cell_type, ..
                } = &cell.upgrade().borrow().prototype
                {
                    if let Some((go, _, _)) =
                        latency_data.get(cell_type.as_ref())
                    {
                        if port.borrow().name == *go {
                            for write_port in graph.writes_to(&*port.borrow()) {
                                if !is_done_port_or_const(
                                    &*write_port.borrow(),
                                    latency_data,
                                ) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }

            ir::PortParent::Group(_) => {
                if port.borrow().name == "done" {
                    for write_port in graph.writes_to(&*port.borrow()) {
                        if !is_done_port_or_const(
                            &*write_port.borrow(),
                            latency_data,
                        ) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Returns true if `graph` contains any nodes with degree > 1.
fn contains_node_deg_gt_one(graph: GraphAnalysis) -> bool {
    for port in graph.ports() {
        if graph.writes_to(&*port.borrow()).count() > 1 {
            return true;
        }
    }
    false
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
    let graph_unprocessed = GraphAnalysis::from(group);
    if contains_dyn_writes(graph_unprocessed.clone(), latency_data) {
        return None;
    }

    let go_done_edges = find_go_done_edges(group, latency_data);
    let graph = graph_unprocessed
        .edge_induced_subgraph(|src, dst| {
            mem_wrt_dep_graph(src, dst, latency_data)
        })
        .add_edges(&go_done_edges)
        .remove_isolated_vertices();

    // Give up if a port has multiple writes to it.
    if contains_node_deg_gt_one(graph.clone()) {
        return None;
    }

    let mut tsort = graph.toposort();
    let start = tsort.next().unwrap();
    let finish = tsort.last().unwrap();

    let paths = graph.paths(&*start.borrow(), &*finish.borrow());
    // If there are no paths, give up.
    if paths.is_empty() {
        return None;
    }
    let first_path = paths.get(0).unwrap();

    // Sum the latencies of each primitive along the path.
    let mut latency_sum = 0;
    for port in first_path {
        if let ir::PortParent::Cell(cell) = &port.borrow().parent {
            if let ir::CellType::Primitive { name, .. } =
                &cell.upgrade().borrow().prototype
            {
                if let Some((go, _, latency)) = latency_data.get(name.as_ref())
                {
                    if port.borrow().name == go {
                        latency_sum += latency;
                    }
                }
            }
        }
    }

    Some(latency_sum)
}

impl Visitor for InferStaticTiming<'_> {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _c: &LibrarySignatures,
    ) -> VisResult {
        let mut latency_result: Option<u64>;
        for group in &comp.groups {
            if let Some(latency) =
                infer_latency(&group.borrow(), &self.prim_latency_data)
            {
                let grp = group.borrow();
                if let Some(curr_lat) = grp.get_attribute("static") {
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
                    group.borrow_mut().add_attribute("static".to_string(), res);
                }
                None => continue,
            }
        }
        Ok(Action::Stop)
    }
}
