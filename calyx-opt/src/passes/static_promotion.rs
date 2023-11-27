use crate::analysis::{GraphAnalysis, ReadWriteSet};
use crate::traversal::{
    Action, ConstructVisitor, Named, Order, VisResult, Visitor,
};
use calyx_ir::{self as ir, LibrarySignatures, RRC};
use calyx_utils::{CalyxResult, Error};
use ir::GetAttributes;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::num::NonZeroU64;
use std::rc::Rc;

const APPROX_ENABLE_SIZE: u64 = 1;
const APPROX_IF_SIZE: u64 = 3;
const APPROX_WHILE_REPEAT_SIZE: u64 = 3;

/// Struct to store information about the go-done interfaces defined by a primitive.
#[derive(Default, Debug)]
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

impl From<&ir::Primitive> for GoDone {
    fn from(prim: &ir::Primitive) -> Self {
        let done_ports: HashMap<_, _> = prim
            .find_all_with_attr(ir::NumAttr::Done)
            .map(|pd| (pd.attributes.get(ir::NumAttr::Done), pd.name()))
            .collect();

        let go_ports = prim
            .find_all_with_attr(ir::NumAttr::Go)
            .filter_map(|pd| {
                pd.attributes.get(ir::NumAttr::Static).and_then(|st| {
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
                port.attributes.get(ir::NumAttr::Static).and_then(|st| {
                    done_ports
                        .get(&port.attributes.get(ir::NumAttr::Go))
                        .map(|done_port| (port.name, *done_port, st))
                })
            })
            .collect_vec();
        GoDone::new(go_ports)
    }
}

/// Infer "promote_static" annotation for groups and promote control to static when
/// (conservatively) possible.
///
/// Promotion follows the current policies:
/// 1. if multiple groups enables aligned inside a seq are marked with the "promote_static"
///     attribute, then promote all promotable enables to static enables, meanwhile,
///     wrap them into a static seq
///     for example:
/// ```
///     seq {
///         a1;
///         @promote_static a2; @promote_static a3; }
/// ```
///     becomes
/// ```
///     seq {
///         a1;
///         static seq {a2; a3;}}
/// ```
/// 2. if all control statements under seq are either static statements or group enables
///     with `promote_static` annotation, then promote all group enables and turn
///     seq into static seq
/// 3. Under a par control op, all group enables marked with `promote_static` will be promoted.
///     all control statements that are either static or group enables with `promote_static` annotation
///     are wrapped inside a static par.
/// ```
/// par {@promote_static a1; a2; @promote_static a3;}
/// ```
/// becomes
/// ```
/// par {
/// static par { a1; a3; }
/// a2;
/// }
/// ```
pub struct StaticPromotion {
    /// component name -> vec<(go signal, done signal, latency)>
    latency_data: HashMap<ir::Id, GoDone>,
    /// dynamic group Id -> promoted static group Id
    static_group_name: HashMap<ir::Id, ir::Id>,
    /// Maps static component names to their latencies
    static_component_latencies: HashMap<ir::Id, NonZeroU64>,
    /// Threshold for promotion
    threshold: u64,
    /// Whether we should stop promoting when we see a loop.
    cycle_limit: Option<u64>,
}

// Override constructor to build latency_data information from the primitives
// library.
impl ConstructVisitor for StaticPromotion {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let mut latency_data = HashMap::new();
        //let mut comp_latency = HashMap::new();
        // Construct latency_data for each primitive
        for prim in ctx.lib.signatures() {
            let done_ports: HashMap<_, _> = prim
                .find_all_with_attr(ir::NumAttr::Done)
                .map(|pd| (pd.attributes.get(ir::NumAttr::Done), pd.name()))
                .collect();

            let go_ports = prim
                .find_all_with_attr(ir::NumAttr::Go)
                .filter_map(|pd| {
                    pd.attributes.get(ir::NumAttr::Static).and_then(|st| {
                        done_ports
                            .get(&pd.attributes.get(ir::NumAttr::Go))
                            .map(|done_port| (pd.name(), *done_port, st))
                    })
                })
                .collect_vec();

            // If this primitive has exactly one (go, done, static) pair, we
            // can infer the latency of its invokes.
            if go_ports.len() == 1 {
                //comp_latency.insert(prim.name, go_ports[0].2);
            }
            latency_data.insert(prim.name, GoDone::new(go_ports));
        }
        let (threshold, cycle_limit) = Self::get_threshold(ctx);
        Ok(StaticPromotion {
            latency_data,
            static_group_name: HashMap::new(),
            static_component_latencies: HashMap::new(),
            threshold,
            cycle_limit,
        })
    }

    // This pass shared information between components
    fn clear_data(&mut self) {
        self.static_group_name = HashMap::new();
    }
}

impl Named for StaticPromotion {
    fn name() -> &'static str {
        "static-promotion"
    }

    fn description() -> &'static str {
        "promote groups and controls whose latency can be inferred to static groups and controls"
    }
}

impl StaticPromotion {
    // Looks through ctx to get the given command line threshold.
    // Default threshold = 1
    fn get_threshold(ctx: &ir::Context) -> (u64, Option<u64>)
    where
        Self: Named,
    {
        let n = Self::name();

        // getting the given opts for -x cell-share:__
        let given_opts: HashSet<_> = ctx
            .extra_opts
            .iter()
            .filter_map(|opt| {
                let mut splits = opt.split(':');
                if splits.next() == Some(n) {
                    splits.next()
                } else {
                    None
                }
            })
            .collect();

        // searching for "-x static-promotion:cycle-limit=200" and getting back "200"
        let cycle_limit_str: Option<&str> = given_opts.iter().find_map(|arg| {
            let split: Vec<&str> = arg.split('=').collect();
            if let Some(str) = split.first() {
                if str == &"cycle-limit" {
                    return Some(split[1]);
                }
            }
            None
        });

        // Default to None. There may be a more idiomatic way to do this.
        let cycle_limit = match cycle_limit_str.unwrap_or("None").parse::<u64>()
        {
            Ok(n) => Some(n),
            Err(_) => None,
        };

        // searching for "-x static-promotion:threshold=1" and getting back "1"
        let threshold: Option<&str> = given_opts.iter().find_map(|arg| {
            let split: Vec<&str> = arg.split('=').collect();
            if let Some(str) = split.first() {
                if str == &"threshold" {
                    return Some(split[1]);
                }
            }
            None
        });

        // Need to convert string argument into int argument
        // Always default to threshold=1
        // Default cycle limit = 2^25 = 33554432
        (
            threshold.unwrap_or("1").parse::<u64>().unwrap_or(1),
            cycle_limit,
        )
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

    /// Gets the inferred latency, which should either be from being a static
    /// control operator or the promote_static attribute.
    /// Will raise an error if neither of these is true.
    fn get_inferred_latency(c: &ir::Control) -> u64 {
        let ir::Control::Static(sc) = c else {
            let Some(latency) = c.get_attribute(ir::NumAttr::PromoteStatic) else {
                unreachable!("Called get_latency on control that is neither static nor promotable")
            };
            return latency;
        };
        sc.get_latency()
    }

    fn check_latencies_match(actual: u64, inferred: u64) {
        assert_eq!(actual, inferred, "Inferred and Annotated Latencies do not match. Latency: {}. Inferred: {}", actual, inferred);
    }

    /// Returns true if a control statement is already static, or has the static
    /// attributes
    fn can_be_promoted(c: &ir::Control) -> bool {
        c.is_static() || c.has_attribute(ir::NumAttr::PromoteStatic)
    }

    fn within_cycle_limit(&self, latency: u64) -> bool {
        if self.cycle_limit.is_none() {
            return true;
        }
        latency < self.cycle_limit.unwrap()
    }

    /// If we've already constructed the static group then use the already existing
    /// group. Otherwise construct `static group` and then return that.
    fn construct_static_group(
        &mut self,
        builder: &mut ir::Builder,
        group: ir::RRC<ir::Group>,
        latency: u64,
    ) -> ir::RRC<ir::StaticGroup> {
        if let Some(s_name) = self.static_group_name.get(&group.borrow().name())
        {
            builder.component.find_static_group(*s_name).unwrap()
        } else {
            let sg = builder.add_static_group(group.borrow().name(), latency);
            self.static_group_name
                .insert(group.borrow().name(), sg.borrow().name());
            for assignment in group.borrow().assignments.iter() {
                if !(assignment.dst.borrow().is_hole()
                    && assignment.dst.borrow().name == "done")
                {
                    let static_s = ir::Assignment::from(assignment.clone());
                    sg.borrow_mut().assignments.push(static_s);
                }
            }
            Rc::clone(&sg)
        }
    }

    /// Converts control to static control.
    /// Control must already be static or have the `promote_static` attribute.
    fn convert_to_static(
        &mut self,
        c: &mut ir::Control,
        builder: &mut ir::Builder,
    ) -> ir::StaticControl {
        assert!(
            c.has_attribute(ir::NumAttr::PromoteStatic) || c.is_static(),
            "Called convert_to_static control that is neither static nor promotable"
        );
        // Need to get bound_attribute here, because we cannot borrow `c` within the
        // pattern match.
        let bound_attribute = c.get_attribute(ir::NumAttr::Bound);
        // Inferred latency of entire control block. Used to double check our
        // function is correct.
        let inferred_latency = Self::get_inferred_latency(c);
        match c {
            ir::Control::Empty(_) => ir::StaticControl::empty(),
            ir::Control::Enable(ir::Enable { group, attributes }) => {
                // Removing the `promote_static` attribute bc we don't need it anymore.
                attributes.remove(ir::NumAttr::PromoteStatic);
                let enable = ir::StaticControl::Enable(ir::StaticEnable {
                    // upgrading group to static group
                    group: self.construct_static_group(
                        builder,
                        Rc::clone(group),
                        group
                            .borrow()
                            .get_attributes()
                            .unwrap()
                            .get(ir::NumAttr::PromoteStatic)
                            .unwrap(),
                    ),
                    attributes: std::mem::take(attributes),
                });
                enable
            }
            ir::Control::Seq(ir::Seq { stmts, attributes }) => {
                // Removing the `promote_static` attribute bc we don't need it anymore
                attributes.remove(ir::NumAttr::PromoteStatic);
                // The resulting static seq should be compactable.
                attributes.insert(ir::NumAttr::Compactable, 1);
                let static_stmts =
                    self.convert_vec_to_static(builder, std::mem::take(stmts));
                let latency =
                    static_stmts.iter().map(|s| s.get_latency()).sum();
                Self::check_latencies_match(latency, inferred_latency);
                ir::StaticControl::Seq(ir::StaticSeq {
                    stmts: static_stmts,
                    attributes: std::mem::take(attributes),
                    latency,
                })
            }
            ir::Control::Par(ir::Par { stmts, attributes }) => {
                // Removing the `promote_static` attribute bc we don't need it anymore
                attributes.remove(ir::NumAttr::PromoteStatic);
                // Convert stmts to static
                let static_stmts =
                    self.convert_vec_to_static(builder, std::mem::take(stmts));
                // Calculate latency
                let latency = static_stmts
                    .iter()
                    .map(|s| s.get_latency())
                    .max()
                    .unwrap_or_else(|| unreachable!("Empty Par Block"));
                Self::check_latencies_match(latency, inferred_latency);
                ir::StaticControl::Par(ir::StaticPar {
                    stmts: static_stmts,
                    attributes: ir::Attributes::default(),
                    latency,
                })
            }
            ir::Control::Repeat(ir::Repeat {
                body,
                num_repeats,
                attributes,
            }) => {
                // Removing the `promote_static` attribute bc we don't need it anymore
                attributes.remove(ir::NumAttr::PromoteStatic);
                let sc = self.convert_to_static(body, builder);
                let latency = (*num_repeats) * sc.get_latency();
                Self::check_latencies_match(latency, inferred_latency);
                ir::StaticControl::Repeat(ir::StaticRepeat {
                    attributes: std::mem::take(attributes),
                    body: Box::new(sc),
                    num_repeats: *num_repeats,
                    latency,
                })
            }
            ir::Control::While(ir::While {
                body, attributes, ..
            }) => {
                // Removing the `promote_static` attribute bc we don't need it anymore
                attributes.remove(ir::NumAttr::PromoteStatic);
                // Removing the `bound` attribute bc we don't need it anymore
                attributes.remove(ir::NumAttr::Bound);
                let sc = self.convert_to_static(body, builder);
                let num_repeats = bound_attribute.unwrap_or_else(|| unreachable!("Called convert_to_static on a while loop without a bound"));
                let latency = num_repeats * sc.get_latency();
                Self::check_latencies_match(latency, inferred_latency);
                ir::StaticControl::Repeat(ir::StaticRepeat {
                    attributes: std::mem::take(attributes),
                    body: Box::new(sc),
                    num_repeats,
                    latency,
                })
            }
            ir::Control::If(ir::If {
                port,
                tbranch,
                fbranch,
                attributes,
                ..
            }) => {
                // Removing the `promote_static` attribute bc we don't need it anymore
                attributes.remove(ir::NumAttr::PromoteStatic);
                let static_tbranch = self.convert_to_static(tbranch, builder);
                let static_fbranch = self.convert_to_static(fbranch, builder);
                let latency = std::cmp::max(
                    static_tbranch.get_latency(),
                    static_fbranch.get_latency(),
                );
                Self::check_latencies_match(latency, inferred_latency);
                ir::StaticControl::static_if(
                    Rc::clone(port),
                    Box::new(static_tbranch),
                    Box::new(static_fbranch),
                    latency,
                )
            }
            ir::Control::Static(_) => c.take_static_control(),
            ir::Control::Invoke(ir::Invoke {
                comp,
                inputs,
                outputs,
                attributes,
                comb_group,
                ref_cells,
            }) => {
                assert!(
                    comb_group.is_none(),
                    "Shouldn't Promote to Static if there is a Comb Group",
                );
                attributes.remove(ir::NumAttr::PromoteStatic);
                Self::check_latencies_match(self.static_component_latencies.get(
                    &comp.borrow().type_name().unwrap_or_else(|| {
                        unreachable!(
                            "Already checked that comp is component"
                        )
                    }),
                ).unwrap_or_else(|| unreachable!("Called convert_to_static for static invoke that does not have a static component")).get(), inferred_latency);
                let s_inv = ir::StaticInvoke {
                    comp: Rc::clone(comp),
                    inputs: std::mem::take(inputs),
                    outputs: std::mem::take(outputs),
                    latency: inferred_latency,
                    attributes: std::mem::take(attributes),
                    ref_cells: std::mem::take(ref_cells),
                    comb_group: std::mem::take(comb_group),
                };
                ir::StaticControl::Invoke(s_inv)
            }
        }
    }

    /// Converts vec of control to vec of static control.
    /// All control statements in the vec must be promotable or already static.
    fn convert_vec_to_static(
        &mut self,
        builder: &mut ir::Builder,
        control_vec: Vec<ir::Control>,
    ) -> Vec<ir::StaticControl> {
        control_vec
            .into_iter()
            .map(|mut c| self.convert_to_static(&mut c, builder))
            .collect()
    }

    /// Calculates the approximate "size" of the control statements.
    /// Tries to approximate the number of dynamic FSM transitions that will occur
    fn approx_size(c: &ir::Control) -> u64 {
        match c {
            ir::Control::Empty(_) => 0,
            ir::Control::Enable(_) => APPROX_ENABLE_SIZE,
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                stmts.iter().map(Self::approx_size).sum()
            }
            ir::Control::Repeat(ir::Repeat { body, .. })
            | ir::Control::While(ir::While { body, .. }) => {
                Self::approx_size(body) + APPROX_WHILE_REPEAT_SIZE
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                Self::approx_size(tbranch)
                    + Self::approx_size(fbranch)
                    + APPROX_IF_SIZE
            }
            ir::Control::Static(_) => {
                // static control appears as one big group to the dynamic FSM
                1
            }
            ir::Control::Invoke(_) => 1,
        }
    }

    /// Uses `approx_size` function to sum the sizes of the control statements
    /// in the given vector
    fn approx_control_vec_size(v: &[ir::Control]) -> u64 {
        v.iter().map(Self::approx_size).sum()
    }

    /// First checks if the vec of control statements satsifies the threshold
    /// and cycle count threshold
    /// (That is, whether the combined approx_size of the static_vec is greater)
    /// than the threshold and cycle count is less than cycle limit).
    /// If so, converts vec of control to a static seq, and returns a vec containing
    /// the static seq.
    /// Otherwise, just returns the vec without changing it.
    fn convert_vec_seq_if_sat(
        &mut self,
        builder: &mut ir::Builder,
        control_vec: Vec<ir::Control>,
    ) -> Vec<ir::Control> {
        if Self::approx_control_vec_size(&control_vec) <= self.threshold
            || !self.within_cycle_limit(
                control_vec.iter().map(Self::get_inferred_latency).sum(),
            )
        {
            // Return unchanged vec
            return control_vec;
        }
        // Convert vec to static seq
        let s_seq_stmts = self.convert_vec_to_static(builder, control_vec);
        let latency = s_seq_stmts.iter().map(|sc| sc.get_latency()).sum();
        let mut sseq =
            ir::Control::Static(ir::StaticControl::seq(s_seq_stmts, latency));
        sseq.get_mut_attributes()
            .insert(ir::NumAttr::Compactable, 1);
        vec![sseq]
    }

    /// First checks if the vec of control statements meets the self.threshold
    /// and is within self.cycle_limit
    /// If so, converts vec of control to a static par, and returns a vec containing
    /// the static par.
    /// Otherwise, just returns the vec without changing it.
    fn convert_vec_par_if_sat(
        &mut self,
        builder: &mut ir::Builder,
        control_vec: Vec<ir::Control>,
    ) -> Vec<ir::Control> {
        if Self::approx_control_vec_size(&control_vec) <= self.threshold
            || !self.within_cycle_limit(
                control_vec
                    .iter()
                    .map(Self::get_inferred_latency)
                    .max()
                    .unwrap_or_else(|| unreachable!("Non Empty Par Block")),
            )
        {
            // Return unchanged vec
            return control_vec;
        }
        // Convert vec to static seq
        let s_par_stmts = self.convert_vec_to_static(builder, control_vec);
        let latency = s_par_stmts
            .iter()
            .map(|sc| sc.get_latency())
            .max()
            .unwrap_or_else(|| unreachable!("empty par block"));
        let spar =
            ir::Control::Static(ir::StaticControl::par(s_par_stmts, latency));
        vec![spar]
    }
}

impl Visitor for StaticPromotion {
    // Require post order traversal of components to ensure `invoke` nodes
    // get timing information for components.
    fn iteration_order() -> Order {
        Order::Post
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _lib: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if comp.name != "main" && comp.control.borrow().is_static() {
            if let Some(lat) = comp.control.borrow().get_latency() {
                if !comp.is_static() {
                    comp.attributes.insert(ir::BoolAttr::Promoted, 1);
                }
                comp.latency = Some(NonZeroU64::new(lat).unwrap());
                let comp_sig = comp.signature.borrow();
                let mut done_ports: Vec<_> =
                    comp_sig.find_all_with_attr(ir::NumAttr::Done).collect();
                let mut go_ports: Vec<_> =
                    comp_sig.find_all_with_attr(ir::NumAttr::Go).collect();
                if done_ports.len() == 1 && go_ports.len() == 1 {
                    let go_done = GoDone::new(vec![(
                        go_ports.pop().unwrap().borrow().name,
                        done_ports.pop().unwrap().borrow().name,
                        lat,
                    )]);
                    self.latency_data.insert(comp.name, go_done);
                }
            }
        }
        if comp.is_static() {
            self.static_component_latencies
                .insert(comp.name, comp.latency.unwrap());
        }
        Ok(Action::Continue)
    }

    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let builder = ir::Builder::new(comp, sigs);
        let mut latency_result: Option<u64>;
        for group in builder.component.get_groups() {
            if let Some(latency) = self.infer_latency(&group.borrow()) {
                let grp = group.borrow();
                if let Some(curr_lat) = grp.attributes.get(ir::NumAttr::Static)
                {
                    // Inferred latency is not the same as the provided latency annotation.
                    if curr_lat != latency {
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

            if let Some(latency) = latency_result {
                group
                    .borrow_mut()
                    .attributes
                    .insert(ir::NumAttr::PromoteStatic, latency);
            }
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
        if let Some(latency) = s
            .group
            .borrow()
            .get_attributes()
            .unwrap()
            .get(ir::NumAttr::PromoteStatic)
        {
            s.attributes.insert(ir::NumAttr::PromoteStatic, latency);
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
        // Shouldn't promote to static invoke if we have a comb group
        if s.comp.borrow().is_component() {
            if let Some(latency) = self
                .static_component_latencies
                .get(&s.comp.borrow().type_name().unwrap())
            {
                s.attributes
                    .insert(ir::NumAttr::PromoteStatic, latency.get());
            }
        }
        Ok(Action::Continue)
    }

    fn finish_seq(
        &mut self,
        s: &mut ir::Seq,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        let old_stmts = std::mem::take(&mut s.stmts);
        let mut new_stmts: Vec<ir::Control> = Vec::new();
        let mut cur_vec: Vec<ir::Control> = Vec::new();
        for stmt in old_stmts {
            if Self::can_be_promoted(&stmt) {
                cur_vec.push(stmt);
            } else {
                // Accumualte cur_vec into a static seq if it meets threshold
                let possibly_promoted_stmts =
                    self.convert_vec_seq_if_sat(&mut builder, cur_vec);
                new_stmts.extend(possibly_promoted_stmts);
                // Add the current (non-promotable) stmt
                new_stmts.push(stmt);
                // New cur_vec
                cur_vec = Vec::new();
            }
        }
        if new_stmts.is_empty() {
            // The entire seq can be promoted
            let approx_size: u64 = cur_vec.iter().map(Self::approx_size).sum();
            if approx_size > self.threshold
                && self.within_cycle_limit(
                    cur_vec.iter().map(Self::get_inferred_latency).sum(),
                )
            {
                // Promote entire seq to a static seq
                let s_seq_stmts =
                    self.convert_vec_to_static(&mut builder, cur_vec);
                let latency =
                    s_seq_stmts.iter().map(|sc| sc.get_latency()).sum();
                let mut sseq = ir::Control::Static(ir::StaticControl::seq(
                    s_seq_stmts,
                    latency,
                ));
                sseq.get_mut_attributes()
                    .insert(ir::NumAttr::Compactable, 1);
                return Ok(Action::change(sseq));
            } else {
                // Doesn't meet threshold.
                // Add attribute to seq so parent might promote it.
                let inferred_latency =
                    cur_vec.iter().map(Self::get_inferred_latency).sum();
                s.attributes
                    .insert(ir::NumAttr::PromoteStatic, inferred_latency);
                s.stmts = cur_vec;
                return Ok(Action::Continue);
            }
        }
        // Entire seq is not static, so we're only promoting the last
        // bit of it if possible.
        let possibly_promoted_stmts =
            self.convert_vec_seq_if_sat(&mut builder, cur_vec);
        new_stmts.extend(possibly_promoted_stmts);

        let new_seq = ir::Control::Seq(ir::Seq {
            stmts: new_stmts,
            attributes: ir::Attributes::default(),
        });
        Ok(Action::change(new_seq))
    }

    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        let mut new_stmts: Vec<ir::Control> = Vec::new();
        // Split the par into static and dynamic stmts
        let (s_stmts, d_stmts): (Vec<ir::Control>, Vec<ir::Control>) =
            s.stmts.drain(..).partition(|s| {
                s.is_static()
                    || s.get_attributes().has(ir::NumAttr::PromoteStatic)
            });
        if d_stmts.is_empty() {
            // Entire par block can be promoted to static
            if Self::approx_control_vec_size(&s_stmts) > self.threshold
                && self.within_cycle_limit(
                    s_stmts
                        .iter()
                        .map(Self::get_inferred_latency)
                        .max()
                        .unwrap_or_else(|| unreachable!("Empty Par Block")),
                )
            {
                // Promote entire par block to static
                let static_par_stmts =
                    self.convert_vec_to_static(&mut builder, s_stmts);
                let latency = static_par_stmts
                    .iter()
                    .map(|sc| sc.get_latency())
                    .max()
                    .unwrap_or_else(|| unreachable!("empty par block"));
                return Ok(Action::change(ir::Control::Static(
                    ir::StaticControl::par(static_par_stmts, latency),
                )));
            } else {
                // Doesn't meet threshold, but add promotion attribute since
                // parent might want to promote it.
                let inferred_latency = s_stmts
                    .iter()
                    .map(Self::get_inferred_latency)
                    .max()
                    .unwrap_or_else(|| unreachable!("empty par block"));
                s.get_mut_attributes()
                    .insert(ir::NumAttr::PromoteStatic, inferred_latency);
                s.stmts = s_stmts;
                return Ok(Action::Continue);
            }
        }
        // Otherwise just promote the par threads that we can into a static par
        let s_stmts_possibly_promoted =
            self.convert_vec_par_if_sat(&mut builder, s_stmts);
        new_stmts.extend(s_stmts_possibly_promoted);
        new_stmts.extend(d_stmts);
        let new_par = ir::Control::Par(ir::Par {
            stmts: new_stmts,
            attributes: ir::Attributes::default(),
        });
        Ok(Action::change(new_par))
    }

    fn finish_if(
        &mut self,
        s: &mut ir::If,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        if Self::can_be_promoted(&s.tbranch)
            && Self::can_be_promoted(&s.fbranch)
        {
            // Both branches can be promoted
            let approx_size_if = Self::approx_size(&s.tbranch)
                + Self::approx_size(&s.fbranch)
                + APPROX_IF_SIZE;
            let latency = std::cmp::max(
                Self::get_inferred_latency(&s.tbranch),
                Self::get_inferred_latency(&s.fbranch),
            );
            if approx_size_if > self.threshold
                && self.within_cycle_limit(latency)
            {
                // Meets size threshold so promote to static
                let static_tbranch =
                    self.convert_to_static(&mut s.tbranch, &mut builder);
                let static_fbranch =
                    self.convert_to_static(&mut s.fbranch, &mut builder);
                return Ok(Action::change(ir::Control::Static(
                    ir::StaticControl::static_if(
                        Rc::clone(&s.port),
                        Box::new(static_tbranch),
                        Box::new(static_fbranch),
                        latency,
                    ),
                )));
            } else {
                // Doesn't meet size threshold, so attach attribute
                // so parent might be able to promote it.
                let inferred_max_latency = std::cmp::max(
                    Self::get_inferred_latency(&s.tbranch),
                    Self::get_inferred_latency(&s.fbranch),
                );
                s.get_mut_attributes()
                    .insert(ir::NumAttr::PromoteStatic, inferred_max_latency)
            }
        }
        Ok(Action::Continue)
    }

    // upgrades @bound while loops to static repeats
    fn finish_while(
        &mut self,
        s: &mut ir::While,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        // First check that while loop is bounded
        if let Some(num_repeats) = s.get_attributes().get(ir::NumAttr::Bound) {
            // Then check that body is static/promotable
            if Self::can_be_promoted(&s.body) {
                let approx_size =
                    Self::approx_size(&s.body) + APPROX_WHILE_REPEAT_SIZE;
                let latency = Self::get_inferred_latency(&s.body) * num_repeats;
                // Then check that it reaches the threshold
                if approx_size > self.threshold
                    && self.within_cycle_limit(latency)
                {
                    // Turn repeat into static repeat
                    let sc = self.convert_to_static(&mut s.body, &mut builder);
                    let static_repeat = ir::StaticControl::repeat(
                        num_repeats,
                        latency,
                        Box::new(sc),
                    );
                    return Ok(Action::Change(Box::new(ir::Control::Static(
                        static_repeat,
                    ))));
                } else {
                    // Attach static_promote attribute since parent control may
                    // want to promote
                    s.attributes.insert(
                        ir::NumAttr::PromoteStatic,
                        num_repeats * Self::get_inferred_latency(&s.body),
                    )
                }
            }
        }
        Ok(Action::Continue)
    }

    // upgrades repeats with static bodies to static repeats
    fn finish_repeat(
        &mut self,
        s: &mut ir::Repeat,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        if Self::can_be_promoted(&s.body) {
            // Body can be promoted
            let approx_size =
                Self::approx_size(&s.body) + APPROX_WHILE_REPEAT_SIZE;
            let latency = Self::get_inferred_latency(&s.body) * s.num_repeats;
            if approx_size > self.threshold && self.within_cycle_limit(latency)
            {
                // Meets size threshold, so turn repeat into static repeat
                let sc = self.convert_to_static(&mut s.body, &mut builder);
                let static_repeat = ir::StaticControl::repeat(
                    s.num_repeats,
                    latency,
                    Box::new(sc),
                );
                return Ok(Action::Change(Box::new(ir::Control::Static(
                    static_repeat,
                ))));
            } else {
                // Doesn't meet threshold.
                // Attach static_promote attribute since parent control may
                // want to promote
                s.attributes.insert(
                    ir::NumAttr::PromoteStatic,
                    s.num_repeats * Self::get_inferred_latency(&s.body),
                )
            }
        }
        Ok(Action::Continue)
    }
}
