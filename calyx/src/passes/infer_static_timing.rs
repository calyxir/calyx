use std::collections::HashMap;

use crate::analysis::{GraphAnalysis, ReadWriteSet};
use crate::errors::{CalyxResult, Error};
use crate::ir::traversal::{
    Action, ConstructVisitor, Named, VisResult, Visitor,
};
use crate::ir::{self, LibrarySignatures, RRC};
use itertools::Itertools;
use std::{cmp, ops::Add};

/// Struct to store information about the go-done interfaces defined by a primitive.
#[derive(Default)]
struct GoDone {
    ports: Vec<(ir::Id, ir::Id, u64)>,
}

impl GoDone {
    pub fn new(ports: Vec<(ir::Id, ir::Id, u64)>) -> Self {
        Self { ports }
    }

    /// Returns true if this is @go port
    pub fn is_go(&self, name: &ir::Id) -> bool {
        self.ports.iter().any(|(go, _, _)| name == go)
    }

    /// Returns true if this is a @done port
    pub fn is_done(&self, name: &ir::Id) -> bool {
        self.ports.iter().any(|(_, done, _)| name == done)
    }

    /// Returns the latency associated with the provided @go port if present
    pub fn get_latency(&self, go_port: &ir::Id) -> Option<u64> {
        self.ports.iter().find_map(|(go, _, lat)| {
            if go == go_port {
                Some(*lat)
            } else {
                None
            }
        })
    }

    /// Iterate over the defined ports
    pub fn iter(&self) -> impl Iterator<Item = &(ir::Id, ir::Id, u64)> {
        self.ports.iter()
    }
}

/// Infer "static" annotation for groups and add "@static" annotation when
/// (conservatively) possible.
///
/// Infers the number of cycles for groups where the `done`
/// signal relies only on other `done` signals, and then inserts "static"
/// annotations with those inferred values. If there is an existing
/// annotation in a group that differs from an inferred value, this
/// pass will throw an error. If a group's `done` signal relies on signals
/// that are not only `done` signals, this pass will ignore that group.
pub struct InferStaticTiming {
    /// primitive name -> vec<(go signal, done signal, latency)>
    latency_data: HashMap<ir::Id, GoDone>,
    /// static timing information for invokable components and primitives
    invoke_latency: HashMap<ir::Id, u64>,
}

// Override constructor to build latency_data information from the primitives
// library.
impl ConstructVisitor for InferStaticTiming {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let mut latency_data = HashMap::new();
        let mut comp_latency = HashMap::new();
        // Construct latency_data for each primitive
        for prim in ctx.lib.signatures() {
            let done_ports: HashMap<_, _> = prim
                .find_all_with_attr("done")
                .map(|pd| (pd.attributes[&"done"], pd.name.clone()))
                .collect();

            let go_ports = prim
                .find_all_with_attr("go")
                .filter_map(|pd| {
                    pd.attributes.get("static").and_then(|st| {
                        done_ports.get(&pd.attributes[&"go"]).map(|done_port| {
                            (pd.name.clone(), done_port.clone(), *st)
                        })
                    })
                })
                .collect_vec();

            // If this primitive has exactly one (go, done, static) pair, we
            // can infer the latency of its invokes.
            if go_ports.len() == 1 {
                comp_latency.insert(prim.name.clone(), go_ports[0].2);
            }
            latency_data.insert(prim.name.clone(), GoDone::new(go_ports));
        }
        Ok(InferStaticTiming {
            latency_data,
            invoke_latency: comp_latency,
        })
    }

    // This pass shared information between components
    fn clear_data(&mut self) {
        /* All data is transferred between components */
    }
}

/// Function to iterate over a vector of control statements and collect
/// the "static" attribute using the `acc` function.
/// Returns None if any of of the Control statements is a compound statement.
fn accumulate_static_time<F>(
    stmts: &[ir::Control],
    start: u64,
    acc: F,
) -> Option<u64>
where
    F: FnMut(u64, u64) -> u64,
{
    stmts
        .iter()
        .map(|con| con.get_attribute("static").copied())
        .fold_options(start, acc)
}

impl Named for InferStaticTiming {
    fn name() -> &'static str {
        "infer-static-timing"
    }

    fn description() -> &'static str {
        "infers and annotates static timing for groups when possible"
    }
}

impl InferStaticTiming {
    /// Return true if the edge (`src`, `dst`) meet one these criteria, and false otherwise:
    ///   - `src` is an "out" port of a constant, and `dst` is a "go" port
    ///   - `src` is a "done" port, and `dst` is a "go" port
    ///   - `src` is a "done" port, and `dst` is the "done" port of a group
    fn mem_wrt_dep_graph(&self, src: &ir::Port, dst: &ir::Port) -> bool {
        match (&src.parent, &dst.parent) {
            (
                ir::PortParent::Cell(src_cell),
                ir::PortParent::Cell(dst_cell),
            ) => {
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
                    let data_dst = self.latency_data.get(dst_cell_prim_type);
                    let data_src = self.latency_data.get(src_cell_prim_type);
                    if let (Some(dst_ports), Some(src_ports)) =
                        (data_dst, data_src)
                    {
                        return src_ports.is_done(&src.name)
                            && dst_ports.is_go(&dst.name);
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
                    if let Some(ports) =
                        self.latency_data.get(dst_cell_prim_type)
                    {
                        return ports.is_go(&dst.name);
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
    fn find_go_done_edges(
        &self,
        group: &ir::Group,
    ) -> Vec<(RRC<ir::Port>, RRC<ir::Port>)> {
        let rw_set = ReadWriteSet::uses(group.assignments.iter());
        let mut go_done_edges: Vec<(RRC<ir::Port>, RRC<ir::Port>)> = Vec::new();

        for cell_ref in rw_set {
            let cell = cell_ref.borrow();
            if let ir::CellType::Primitive {
                name: cell_type, ..
            } = &cell.prototype
            {
                if let Some(ports) = self.latency_data.get(cell_type) {
                    go_done_edges.extend(
                        ports.iter().map(|(go, done, _)| {
                            (cell.get(go), cell.get(done))
                        }),
                    )
                }
            }
        }
        go_done_edges
    }

    /// Returns true if `port` is a "done" port, and we know the latency data
    /// about `port`, or is a constant.
    fn is_done_port_or_const(&self, port: &ir::Port) -> bool {
        if let ir::PortParent::Cell(cell) = &port.parent {
            if let ir::CellType::Primitive {
                name: cell_type, ..
            } = &cell.upgrade().borrow().prototype
            {
                if let Some(ports) = self.latency_data.get(cell_type) {
                    return ports.is_done(&port.name);
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
    fn contains_dyn_writes(&self, graph: &GraphAnalysis) -> bool {
        for port in &graph.ports() {
            match &port.borrow().parent {
                ir::PortParent::Cell(cell) => {
                    if let ir::CellType::Primitive {
                        name: cell_type, ..
                    } = &cell.upgrade().borrow().prototype
                    {
                        if let Some(ports) = self.latency_data.get(cell_type) {
                            let name = &port.borrow().name;
                            if ports.is_go(name) {
                                for write_port in
                                    graph.writes_to(&*port.borrow())
                                {
                                    if !self.is_done_port_or_const(
                                        &*write_port.borrow(),
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
                            if !self
                                .is_done_port_or_const(&*write_port.borrow())
                            {
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
    fn contains_node_deg_gt_one(graph: &GraphAnalysis) -> bool {
        for port in graph.ports() {
            if graph.writes_to(&*port.borrow()).count() > 1 {
                return true;
            }
        }
        false
    }

    /// Attempts to infer the number of cycles starting when
    /// `group[go]` is high, and port is high. If inference is
    /// not possible, returns None.
    fn infer_latency(&self, group: &ir::Group) -> Option<u64> {
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
        if self.contains_dyn_writes(&graph_unprocessed) {
            return None;
        }

        let go_done_edges = self.find_go_done_edges(group);
        let graph = graph_unprocessed
            .edge_induced_subgraph(|src, dst| self.mem_wrt_dep_graph(src, dst))
            .add_edges(&go_done_edges)
            .remove_isolated_vertices();

        // Give up if a port has multiple writes to it.
        if Self::contains_node_deg_gt_one(&graph) {
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
                    if let Some(ports) = self.latency_data.get(name) {
                        if let Some(latency) =
                            ports.get_latency(&port.borrow().name)
                        {
                            latency_sum += latency;
                        }
                    }
                }
            }
        }

        Some(latency_sum)
    }
}

impl Visitor for InferStaticTiming {
    // Require post order traversal of components to ensure `invoke` nodes
    // get timing information for components.
    fn require_postorder() -> bool {
        true
    }

    fn start(
        &mut self,
        comp: &mut ir::Component,
        _lib: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut latency_result: Option<u64>;
        for group in comp.groups.iter() {
            if let Some(latency) = self.infer_latency(&group.borrow()) {
                let grp = group.borrow();
                if let Some(curr_lat) = grp.attributes.get("static") {
                    // Inferred latency is not the same as the provided latency annotation.
                    if *curr_lat != latency {
                        let msg1 = format!("Annotated latency: {}", curr_lat);
                        let msg2 = format!("Inferred latency: {}", latency);
                        let msg = format!(
                            "Impossible \"static\" latency annotation for group {}.\n{}\n{}",
                            grp.name(),
                            msg1,
                            msg2
                        );
                        return Err(Error::malformed_structure(msg)
                            .with_pos(&grp.attributes));
                    }
                }
                latency_result = Some(latency);
            } else {
                latency_result = None;
            }

            match latency_result {
                Some(res) => {
                    group.borrow_mut().attributes.insert("static", res);
                }
                None => continue,
            }
        }
        Ok(Action::Continue)
    }

    fn finish_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let (Some(bound), Some(body_time)) = (
            s.attributes.get("bound").cloned(),
            s.body.get_attribute("static"),
        ) {
            s.attributes.insert("static", bound * body_time);
        }
        Ok(Action::Continue)
    }

    fn finish_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let (Some(ttime), Some(ftime)) = (
            s.tbranch.get_attribute("static"),
            s.fbranch.get_attribute("static"),
        ) {
            s.attributes.insert("static", 1 + cmp::max(ttime, ftime));
        }

        Ok(Action::Continue)
    }

    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(time) = accumulate_static_time(&s.stmts, 0, cmp::max) {
            s.attributes.insert("static", time);
        }
        Ok(Action::Continue)
    }

    fn finish_seq(
        &mut self,
        s: &mut ir::Seq,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(time) = accumulate_static_time(&s.stmts, 0, Add::add) {
            s.attributes.insert("static", time);
        }
        Ok(Action::Continue)
    }

    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(time) = s.group.borrow().attributes.get("static") {
            s.attributes.insert("static", *time);
        }

        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // If we've found static timing for the invoked component, add
        // this information to invoke.
        if let Some(time) = &s
            .comp
            .borrow()
            .type_name()
            .and_then(|name| self.invoke_latency.get(name))
        {
            s.attributes.insert("static", **time);
        }
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _lib: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(time) = comp.control.borrow().get_attribute("static") {
            let go_port =
                comp.signature.borrow().find_with_attr("go").ok_or_else(
                    || {
                        Error::malformed_structure(format!(
                            "Component {} does not have a @go port",
                            comp.name
                        ))
                    },
                )?;
            go_port.borrow_mut().attributes.insert("static", *time);
            self.invoke_latency.insert(comp.name.clone(), *time);
        }
        Ok(Action::Continue)
    }
}
