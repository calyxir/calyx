use super::AssignmentAnalysis;
use crate::analysis::{compute_static::WithStatic, GraphAnalysis};
use calyx_ir::{self as ir, GetAttributes, RRC};
use ir::CellType;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

/// Struct to store information about the go-done interfaces defined by a primitive.
/// There is no default implementation because it will almost certainly be very
/// unhelpful: you will want to use `from_ctx`.
#[derive(Debug)]
pub struct GoDone {
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

    /// Iterate over the defined ports
    pub fn is_empty(&self) -> bool {
        self.ports.is_empty()
    }

    /// Iterate over the defined ports
    pub fn len(&self) -> usize {
        self.ports.len()
    }

    /// Iterate over the defined ports
    pub fn get_ports(&self) -> &Vec<(ir::Id, ir::Id, u64)> {
        &self.ports
    }
}

impl From<&ir::Primitive> for GoDone {
    fn from(prim: &ir::Primitive) -> Self {
        let done_ports: HashMap<_, _> = prim
            .find_all_with_attr(ir::NumAttr::Done)
            .map(|pd| (pd.attributes.get(ir::NumAttr::Done), pd.name()))
            .collect();

        let go_ports = prim
            .find_all_with_attr(ir::NumAttr::Go)
            .filter_map(|pd| {
                // Primitives only have @interval.
                pd.attributes.get(ir::NumAttr::Interval).and_then(|st| {
                    done_ports
                        .get(&pd.attributes.get(ir::NumAttr::Go))
                        .map(|done_port| (pd.name(), *done_port, st))
                })
            })
            .collect_vec();
        GoDone::new(go_ports)
    }
}

impl From<&ir::Cell> for GoDone {
    fn from(cell: &ir::Cell) -> Self {
        let done_ports: HashMap<_, _> = cell
            .find_all_with_attr(ir::NumAttr::Done)
            .map(|pr| {
                let port = pr.borrow();
                (port.attributes.get(ir::NumAttr::Done), port.name)
            })
            .collect();

        let go_ports = cell
            .find_all_with_attr(ir::NumAttr::Go)
            .filter_map(|pr| {
                let port = pr.borrow();
                // Get static interval thru either @interval or @promotable.
                let st = match port.attributes.get(ir::NumAttr::Interval) {
                    Some(st) => Some(st),
                    None => port.attributes.get(ir::NumAttr::Promotable),
                };
                if let Some(static_latency) = st {
                    return done_ports
                        .get(&port.attributes.get(ir::NumAttr::Go))
                        .map(|done_port| {
                            (port.name, *done_port, static_latency)
                        });
                }
                None
            })
            .collect_vec();
        GoDone::new(go_ports)
    }
}

/// Default implemnetation is almost certainly not helpful.
/// You should probably use `from_ctx` instead.
pub struct InferenceAnalysis {
    /// component name -> vec<(go signal, done signal, latency)>
    pub latency_data: HashMap<ir::Id, GoDone>,
    /// Maps static component names to their latencies, but there can only
    /// be one go port on the component. (This is a subset of the information
    /// given by latency_data), and is helpful for inferring invokes.
    /// Perhaps someday we should get rid of it and only make it one field.
    pub static_component_latencies: HashMap<ir::Id, u64>,

    updated_components: HashSet<ir::Id>,
}

impl InferenceAnalysis {
    /// Builds FixUp struct from a ctx. Looks at all primitives and component
    /// signatures to get latency information.
    pub fn from_ctx(ctx: &ir::Context) -> Self {
        let mut latency_data = HashMap::new();
        let mut static_component_latencies = HashMap::new();
        // Construct latency_data for each primitive
        for prim in ctx.lib.signatures() {
            let prim_go_done = GoDone::from(prim);
            if prim_go_done.len() == 1 {
                static_component_latencies
                    .insert(prim.name, prim_go_done.get_ports()[0].2);
            }
            latency_data.insert(prim.name, GoDone::from(prim));
        }
        for comp in &ctx.components {
            let comp_sig = comp.signature.borrow();

            let done_ports: HashMap<_, _> = comp_sig
                .find_all_with_attr(ir::NumAttr::Done)
                .map(|pd| {
                    let pd_ref = pd.borrow();
                    (pd_ref.attributes.get(ir::NumAttr::Done), pd_ref.name)
                })
                .collect();

            let go_ports = comp_sig
                .find_all_with_attr(ir::NumAttr::Go)
                .filter_map(|pd| {
                    let pd_ref = pd.borrow();
                    // Get static interval thru either @interval or @promotable.
                    let st = match pd_ref.attributes.get(ir::NumAttr::Interval)
                    {
                        Some(st) => Some(st),
                        None => pd_ref.attributes.get(ir::NumAttr::Promotable),
                    };
                    if let Some(static_latency) = st {
                        return done_ports
                            .get(&pd_ref.attributes.get(ir::NumAttr::Go))
                            .map(|done_port| {
                                (pd_ref.name, *done_port, static_latency)
                            });
                    }
                    None
                })
                .collect_vec();

            let go_done_comp = GoDone::new(go_ports);

            if go_done_comp.len() == 1 {
                static_component_latencies
                    .insert(comp.name, go_done_comp.get_ports()[0].2);
            }
            latency_data.insert(comp.name, go_done_comp);
        }
        InferenceAnalysis {
            latency_data,
            static_component_latencies,
            updated_components: HashSet::new(),
        }
    }

    /// Updates the component, given a component name and a new latency and GoDone object.
    pub fn add_component(
        &mut self,
        (comp_name, latency, go_done): (ir::Id, u64, GoDone),
    ) {
        self.latency_data.insert(comp_name, go_done);
        self.static_component_latencies.insert(comp_name, latency);
    }

    /// Updates the component, given a component name and a new latency.
    /// Note that this expects that the component already is accounted for
    /// in self.latency_data and self.static_component_latencies.
    pub fn remove_component(&mut self, comp_name: ir::Id) {
        if self.latency_data.contains_key(&comp_name) {
            // To make inference as strong as possible, only update updated_components
            // if we actually updated it.
            self.updated_components.insert(comp_name);
        }
        self.latency_data.remove(&comp_name);
        self.static_component_latencies.remove(&comp_name);
    }

    /// Updates the component, given a component name and a new latency.
    /// Note that this expects that the component already is accounted for
    /// in self.latency_data and self.static_component_latencies.
    pub fn adjust_component(
        &mut self,
        (comp_name, adjusted_latency): (ir::Id, u64),
    ) {
        // Check whether we actually updated the component's latency.
        let mut updated = false;
        self.latency_data.entry(comp_name).and_modify(|go_done| {
            for (_, _, cur_latency) in &mut go_done.ports {
                // Updating components with latency data.
                if *cur_latency != adjusted_latency {
                    *cur_latency = adjusted_latency;
                    updated = true;
                }
            }
        });
        self.static_component_latencies
            .insert(comp_name, adjusted_latency);
        if updated {
            self.updated_components.insert(comp_name);
        }
    }

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
        let rw_set = group.assignments.iter().analysis().cell_uses();
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

              ir::PortParent::StaticGroup(_) => // done ports of static groups should clearly NOT have static latencies
              panic!("Have not decided how to handle static groups in infer-static-timing"),
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
        let start = tsort.next()?;
        let finish = tsort.last()?;

        let paths = graph.paths(&start.borrow(), &finish.borrow());
        // If there are no paths, give up.
        if paths.is_empty() {
            log::debug!("FAIL: No path between @go and @done port");
            return None;
        }
        let first_path = paths.first().unwrap();

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

    /// Returns Some(latency) if a control statement has a latency, because
    /// it is static or is has the @promotable attribute
    pub fn get_possible_latency(c: &ir::Control) -> Option<u64> {
        match c {
            ir::Control::Static(sc) => Some(sc.get_latency()),
            _ => c.get_attribute(ir::NumAttr::Promotable),
        }
    }

    pub fn remove_promotable_from_seq(seq: &mut ir::Seq) {
        for stmt in &mut seq.stmts {
            Self::remove_promotable_attribute(stmt);
        }
        seq.get_mut_attributes().remove(ir::NumAttr::Promotable);
    }

    /// Removes the @promotable attribute from the control program.
    /// Recursively visits the children of the control.
    pub fn remove_promotable_attribute(c: &mut ir::Control) {
        c.get_mut_attributes().remove(ir::NumAttr::Promotable);
        match c {
            ir::Control::Empty(_)
            | ir::Control::Invoke(_)
            | ir::Control::Enable(_)
            | ir::Control::Static(_) => (),
            ir::Control::While(ir::While { body, .. })
            | ir::Control::Repeat(ir::Repeat { body, .. }) => {
                Self::remove_promotable_attribute(body);
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                Self::remove_promotable_attribute(tbranch);
                Self::remove_promotable_attribute(fbranch);
            }
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                for stmt in stmts {
                    Self::remove_promotable_attribute(stmt);
                }
            }
        }
    }

    pub fn fixup_seq(&self, seq: &mut ir::Seq) {
        seq.update_static(&self.static_component_latencies);
    }

    pub fn fixup_par(&self, par: &mut ir::Par) {
        par.update_static(&self.static_component_latencies);
    }

    pub fn fixup_if(&self, _if: &mut ir::If) {
        _if.update_static(&self.static_component_latencies);
    }

    pub fn fixup_while(&self, _while: &mut ir::While) {
        _while.update_static(&self.static_component_latencies);
    }

    pub fn fixup_repeat(&self, repeat: &mut ir::Repeat) {
        repeat.update_static(&self.static_component_latencies);
    }

    pub fn fixup_ctrl(&self, ctrl: &mut ir::Control) {
        ctrl.update_static(&self.static_component_latencies);
    }

    /// "Fixes Up" the component. In particular:
    /// 1. Removes @promotable annotations for any groups that write to any
    /// `updated_components`.
    /// 2. Try to re-infer groups' latencies.
    /// 3. Removes all @promotable annotation from the control program.
    /// 4. Re-infers the @promotable annotations for any groups or control.
    /// Note that this only fixes up the component's ``internals''.
    /// It does *not* fix the component's signature.
    pub fn fixup_timing(&self, comp: &mut ir::Component) {
        // Removing @promotable annotations for any groups that write to an updated_component,
        // then try to re-infer the latency.
        for group in comp.groups.iter() {
            // This checks any group that writes to the component:
            // We can probably switch this to any group that writes to the component's
            // `go` port to be more precise analysis.
            if group
                .borrow_mut()
                .assignments
                .iter()
                .analysis()
                .cell_writes()
                .any(|cell| match cell.borrow().prototype {
                    CellType::Component { name } => {
                        self.updated_components.contains(&name)
                    }
                    _ => false,
                })
            {
                // Remove attribute from group.
                group
                    .borrow_mut()
                    .attributes
                    .remove(ir::NumAttr::Promotable);
            }
        }

        for group in &mut comp.groups.iter() {
            // Immediately try to re-infer the latency of the group.
            let latency_result = self.infer_latency(&group.borrow());
            if let Some(latency) = latency_result {
                group
                    .borrow_mut()
                    .attributes
                    .insert(ir::NumAttr::Promotable, latency);
            }
        }

        // Removing @promotable annotations for the control flow, then trying
        // to re-infer them.
        Self::remove_promotable_attribute(&mut comp.control.borrow_mut());
        comp.control
            .borrow_mut()
            .update_static(&self.static_component_latencies);
    }
}
