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
#[derive(Default, Debug)]
struct GoDone {
    ports: Vec<(ir::Id, ir::Id, u64)>,
}

impl GoDone {
    pub fn new(ports: Vec<(ir::Id, ir::Id, u64)>) -> Self {
        Self { ports }
    }

    // Returns the latency associated with invoking this primitive.
    // The component is not invokable if it has multiple go-done ports.
    pub fn invoke_latency(&self) -> Option<u64> {
        if self.ports.len() == 1 {
            Some(self.ports[0].2)
        } else {
            None
        }
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

impl From<&ir::Primitive> for GoDone {
    fn from(prim: &ir::Primitive) -> Self {
        let done_ports: HashMap<_, _> = prim
            .find_all_with_attr("done")
            .map(|pd| (pd.attributes["done"], pd.name))
            .collect();

        let go_ports = prim
            .find_all_with_attr("go")
            .filter_map(|pd| {
                pd.attributes.get("static").and_then(|st| {
                    done_ports
                        .get(&pd.attributes["go"])
                        .map(|done_port| (pd.name, *done_port, *st))
                })
            })
            .collect_vec();
        GoDone::new(go_ports)
    }
}

impl From<&ir::Cell> for GoDone {
    fn from(cell: &ir::Cell) -> Self {
        let done_ports: HashMap<_, _> = cell
            .find_all_with_attr("done")
            .map(|pr| {
                let port = pr.borrow();
                (port.attributes["done"], port.name)
            })
            .collect();

        let go_ports = cell
            .find_all_with_attr("go")
            .filter_map(|pr| {
                let port = pr.borrow();
                port.attributes.get("static").and_then(|st| {
                    done_ports
                        .get(&port.attributes["go"])
                        .map(|done_port| (port.name, *done_port, *st))
                })
            })
            .collect_vec();
        GoDone::new(go_ports)
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
    /// component name -> vec<(go signal, done signal, latency)>
    latency_data: HashMap<ir::Id, GoDone>,
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
                .map(|pd| (pd.attributes["done"], pd.name))
                .collect();

            let go_ports = prim
                .find_all_with_attr("go")
                .filter_map(|pd| {
                    pd.attributes.get("static").and_then(|st| {
                        done_ports
                            .get(&pd.attributes["go"])
                            .map(|done_port| (pd.name, *done_port, *st))
                    })
                })
                .collect_vec();

            // If this primitive has exactly one (go, done, static) pair, we
            // can infer the latency of its invokes.
            if go_ports.len() == 1 {
                comp_latency.insert(prim.name, go_ports[0].2);
            }
            latency_data.insert(prim.name, GoDone::new(go_ports));
        }
        Ok(InferStaticTiming { latency_data })
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
                ir::PortParent::Cell(src_cell_wrf),
                ir::PortParent::Cell(dst_cell_wrf),
            ) => {
                let src_rf = src_cell_wrf.upgrade();
                let src_cell = src_rf.borrow();
                let dst_rf = dst_cell_wrf.upgrade();
                let dst_cell = dst_rf.borrow();
                if let (Some(s_name), Some(d_name)) =
                    (src_cell.type_name(), dst_cell.type_name())
                {
                    let data_src = self.latency_data.get(&s_name);
                    let data_dst = self.latency_data.get(&d_name);
                    if let (Some(dst_ports), Some(src_ports)) =
                        (data_dst, data_src)
                    {
                        return src_ports.is_done(&src.name)
                            && dst_ports.is_go(&dst.name);
                    }
                }

                // A constant writes to a cell: to be added to the graph, the cell needs to be a "done" port.
                if let (Some(d_name), ir::CellType::Constant { .. }) =
                    (dst_cell.type_name(), &src_cell.prototype)
                {
                    if let Some(ports) = self.latency_data.get(&d_name) {
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
            if let Some(ports) =
                cell.type_name().and_then(|c| self.latency_data.get(&c))
            {
                go_done_edges.extend(
                    ports
                        .iter()
                        .map(|(go, done, _)| (cell.get(go), cell.get(done))),
                )
            }
        }
        go_done_edges
    }

    /// Returns true if `port` is a "done" port, and we know the latency data
    /// about `port`, or is a constant.
    fn is_done_port_or_const(&self, port: &ir::Port) -> bool {
        if let ir::PortParent::Cell(cwrf) = &port.parent {
            let cr = cwrf.upgrade();
            let cell = cr.borrow();
            if let ir::CellType::Constant { val, .. } = &cell.prototype {
                if *val > 0 {
                    return true;
                }
            } else if let Some(ports) =
                cell.type_name().and_then(|c| self.latency_data.get(&c))
            {
                return ports.is_done(&port.name);
            }
        }
        false
    }

    /// Returns true if `graph` contains writes to "done" ports
    /// that could have dynamic latencies, false otherwise.
    fn contains_dyn_writes(&self, graph: &GraphAnalysis) -> bool {
        for port in &graph.ports() {
            match &port.borrow().parent {
                ir::PortParent::Cell(cell_wrf) => {
                    let cr = cell_wrf.upgrade();
                    let cell = cr.borrow();
                    if let Some(ports) =
                        cell.type_name().and_then(|c| self.latency_data.get(&c))
                    {
                        let name = &port.borrow().name;
                        if ports.is_go(name) {
                            for write_port in graph.writes_to(&port.borrow()) {
                                if !self
                                    .is_done_port_or_const(&write_port.borrow())
                                {
                                    log::debug!(
                                        "`{}` is not a done port",
                                        write_port.borrow().canonical(),
                                    );
                                    return true;
                                }
                            }
                        }
                    }
                }

                ir::PortParent::Group(_) => {
                    if port.borrow().name == "done" {
                        for write_port in graph.writes_to(&port.borrow()) {
                            if !self.is_done_port_or_const(&write_port.borrow())
                            {
                                log::debug!(
                                    "`{}` is not a done port",
                                    write_port.borrow().canonical(),
                                );
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
            if graph.writes_to(&port.borrow()).count() > 1 {
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
        log::debug!("Checking group `{}`", group.name());
        let graph_unprocessed = GraphAnalysis::from(group);
        if self.contains_dyn_writes(&graph_unprocessed) {
            log::debug!("FAIL: contains dynamic writes");
            return None;
        }

        let go_done_edges = self.find_go_done_edges(group);
        let graph = graph_unprocessed
            .edge_induced_subgraph(|src, dst| self.mem_wrt_dep_graph(src, dst))
            .add_edges(&go_done_edges)
            .remove_isolated_vertices();

        // Give up if a port has multiple writes to it.
        if Self::contains_node_deg_gt_one(&graph) {
            log::debug!("FAIL: Group contains multiple writes");
            return None;
        }

        let mut tsort = graph.toposort();
        let start = tsort.next().unwrap();
        let finish = tsort.last().unwrap();

        let paths = graph.paths(&start.borrow(), &finish.borrow());
        // If there are no paths, give up.
        if paths.is_empty() {
            log::debug!("FAIL: No path between @go and @done port");
            return None;
        }
        let first_path = paths.get(0).unwrap();

        // Sum the latencies of each primitive along the path.
        let mut latency_sum = 0;
        for port in first_path {
            if let ir::PortParent::Cell(cwrf) = &port.borrow().parent {
                let cr = cwrf.upgrade();
                let cell = cr.borrow();
                if let Some(ports) =
                    cell.type_name().and_then(|c| self.latency_data.get(&c))
                {
                    if let Some(latency) =
                        ports.get_latency(&port.borrow().name)
                    {
                        latency_sum += latency;
                    }
                }
            }
        }

        log::debug!("SUCCESS: Latency = {}", latency_sum);
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
                            "Invalid \"static\" latency annotation for group {}.\n{}\n{}",
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
        if s.cond.is_some() {
            return Err(Error::pass_assumption(
                Self::name(),
                format!(
                    "Cannot infer timing for while-with. Run {} before this pass",
                    super::RemoveCombGroups::name()
                ),
            )
            .with_pos(&s.attributes));
        }
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
        if s.cond.is_some() {
            return Err(Error::pass_assumption(
                Self::name(),
                format!(
                    "Cannot infer timing for if-with. Run {} before this pass",
                    super::RemoveCombGroups::name()
                ),
            )
            .with_pos(&s.attributes));
        }
        if let (Some(ttime), Some(ftime)) = (
            s.tbranch.get_attribute("static"),
            s.fbranch.get_attribute("static"),
        ) {
            s.attributes.insert("static", *cmp::max(ttime, ftime));
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
            .and_then(|name| self.latency_data.get(&name))
            .and_then(|ports| ports.invoke_latency())
        {
            s.attributes.insert("static", *time);
        }
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _lib: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut go_ports = comp
            .signature
            .borrow()
            .find_all_with_attr("go")
            .collect_vec();

        // There is exactly one go port.
        if go_ports.len() == 1 {
            let go_port = go_ports.pop().unwrap();
            // If the control program has a static time, make sure the go port's static annotation matches it
            if let Some(&time) = comp.control.borrow().get_attribute("static") {
                let maybe_go_time = {
                    let gp = go_port.borrow();
                    gp.attributes.get("static").copied()
                };
                if let Some(go_time) = maybe_go_time {
                    if go_time != time {
                        let msg1 = format!("Annotated latency: {}", go_time);
                        let msg2 = format!("Inferred latency: {}", time);
                        let msg = format!(
                        "Impossible \"static\" latency annotation for component {}.\n{}\n{}",
                        comp.name,
                        msg1,
                        msg2
                    );
                        return Err(Error::malformed_structure(msg)
                            .with_pos(&go_port.borrow().attributes));
                    }
                } else {
                    go_port.borrow_mut().attributes.insert("static", time);
                }
                log::info!(
                    "Component `{}` has static time {}",
                    comp.name,
                    time
                );
            }
        }

        // Add all go-done latencies to the context
        let sig = &*comp.signature.borrow();
        let ports: GoDone = sig.into();

        self.latency_data.insert(comp.name, ports);

        Ok(Action::Continue)
    }
}
