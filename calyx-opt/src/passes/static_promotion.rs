use crate::analysis::{GraphAnalysis, IntoStatic, ReadWriteSet};
use crate::traversal::{
    Action, ConstructVisitor, Named, Order, VisResult, Visitor,
};
use calyx_ir::{self as ir, LibrarySignatures, RRC};
use calyx_utils::{CalyxResult, Error};
use ir::{GetAttributes, StaticControl};
use itertools::Itertools;
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::rc::Rc;

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
            .map(|pd| (pd.attributes.get(ir::NumAttr::Done), pd.name))
            .collect();

        let go_ports = prim
            .find_all_with_attr(ir::NumAttr::Go)
            .filter_map(|pd| {
                pd.attributes.get(ir::NumAttr::Static).and_then(|st| {
                    done_ports
                        .get(&pd.attributes.get(ir::NumAttr::Go))
                        .map(|done_port| (pd.name, *done_port, st))
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
    static_group_name: HashMap<ir::Id, ir::Id>,
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
                .map(|pd| (pd.attributes.get(ir::NumAttr::Done), pd.name))
                .collect();

            let go_ports = prim
                .find_all_with_attr(ir::NumAttr::Go)
                .filter_map(|pd| {
                    pd.attributes.get(ir::NumAttr::Static).and_then(|st| {
                        done_ports
                            .get(&pd.attributes.get(ir::NumAttr::Go))
                            .map(|done_port| (pd.name, *done_port, st))
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
        Ok(StaticPromotion {
            latency_data,
            static_group_name: HashMap::new(),
        })
    }

    // This pass shared information between components
    fn clear_data(&mut self) {
        /* All data is transferred between components */
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

    fn construct_static_enable(
        &mut self,
        builder: &mut ir::Builder,
        en: ir::Enable,
    ) -> ir::StaticControl {
        if let Some(s_name) =
            self.static_group_name.get(&en.group.borrow().name())
        {
            ir::StaticControl::Enable(ir::StaticEnable {
                group: builder.component.find_static_group(*s_name).unwrap(),
                attributes: ir::Attributes::default(),
            })
        } else {
            let sg = builder.add_static_group(
                en.group.borrow().name(),
                en.get_attributes().get(ir::NumAttr::PromoteStatic).unwrap(),
            );
            self.static_group_name
                .insert(en.group.borrow().name(), sg.borrow().name());
            for assignment in en.group.borrow().assignments.iter() {
                if !(assignment.dst.borrow().is_hole()
                    && assignment.dst.borrow().name == "done")
                {
                    let static_s = ir::Assignment::from(assignment.clone());
                    sg.borrow_mut().assignments.push(static_s);
                }
            }
            ir::StaticControl::Enable(ir::StaticEnable {
                group: Rc::clone(&sg),
                attributes: ir::Attributes::default(),
            })
        }
    }

    fn construct_static_seq(
        &mut self,
        builder: &mut ir::Builder,
        static_vec: &mut Vec<ir::Control>,
    ) -> ir::Control {
        let mut latency = 0;
        let mut static_seq_st: Vec<ir::StaticControl> = Vec::new();
        for s in std::mem::take(static_vec) {
            match s {
            ir::Control::Static(sc) => {
                latency += sc.get_latency();
                static_seq_st.push(sc);
            }
            ir::Control::Enable(en) => {
                let sen = self.construct_static_enable(builder, en);
                latency += sen.get_latency();
                static_seq_st.push(sen);
            }
            _ => unreachable!("We do not insert non-static controls other than group enables with `promote_static` attribute")
        }
        }
        ir::Control::Static(StaticControl::Seq(ir::StaticSeq {
            stmts: static_seq_st,
            attributes: ir::Attributes::default(),
            latency,
        }))
    }

    fn construct_static_par(
        &mut self,
        builder: &mut ir::Builder,
        s_stmts: &mut Vec<ir::Control>,
    ) -> ir::Control {
        let mut latency = 0;
        let mut static_par_st: Vec<ir::StaticControl> = Vec::new();
        for s in std::mem::take(s_stmts) {
            match s {
                ir::Control::Static(sc) => {
                    latency = std::cmp::max(latency, sc.get_latency());
                    static_par_st.push(sc);
                }
                ir::Control::Enable(en) => {
                    let sen = self.construct_static_enable(builder, en);
                    latency = std::cmp::max(latency, sen.get_latency());
                    static_par_st.push(sen);
                }
                _ => unreachable!("We do not insert non-static controls other than group enables with `promote_static` attribute")
            }
        }
        ir::Control::Static(StaticControl::Par(ir::StaticPar {
            stmts: static_par_st,
            attributes: ir::Attributes::default(),
            latency,
        }))
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
        if comp.control.borrow().is_static() {
            if let Some(lat) = comp.control.borrow().get_latency() {
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
        comps: &[ir::Component],
    ) -> VisResult {
        if s.comp.borrow().is_component() {
            let name = s.comp.borrow().type_name().unwrap();
            for c in comps {
                if c.name == name && c.is_static() {
                    let emp = ir::Invoke {
                        comp: Rc::clone(&s.comp),
                        inputs: Vec::new(),
                        outputs: Vec::new(),
                        attributes: ir::Attributes::default(),
                        comb_group: None,
                        ref_cells: Vec::new(),
                    };
                    let actual_invoke = std::mem::replace(s, emp);
                    let s_inv = ir::StaticInvoke {
                        comp: Rc::clone(&actual_invoke.comp),
                        inputs: actual_invoke.inputs,
                        outputs: actual_invoke.outputs,
                        latency: c.latency.unwrap().get(),
                        attributes: ir::Attributes::default(),
                        ref_cells: actual_invoke.ref_cells,
                    };
                    return Ok(Action::change(ir::Control::Static(
                        ir::StaticControl::Invoke(s_inv),
                    )));
                }
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
        let mut new_stmts: Vec<ir::Control> = Vec::new();
        let mut static_vec: Vec<ir::Control> = Vec::new();
        let mut promote_control = true;
        for stmt in std::mem::take(&mut s.stmts) {
            if stmt.is_static()
                || stmt.has_attribute(ir::NumAttr::PromoteStatic)
            {
                static_vec.push(stmt);
            } else {
                promote_control = false;
                if static_vec.len() == 1 {
                    new_stmts.extend(static_vec);
                } else if static_vec.len() > 1 {
                    let sseq = self
                        .construct_static_seq(&mut builder, &mut static_vec);
                    new_stmts.push(sseq);
                }
                new_stmts.push(stmt);
                static_vec = Vec::new();
            }
        }
        if !static_vec.is_empty() {
            if static_vec.len() == 1 {
                new_stmts.extend(static_vec);
            } else {
                let sseq =
                    self.construct_static_seq(&mut builder, &mut static_vec);
                if promote_control {
                    return Ok(Action::change(sseq));
                } else {
                    new_stmts.push(sseq);
                }
            }
        }
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
        let (mut s_stmts, d_stmts): (Vec<ir::Control>, Vec<ir::Control>) =
            std::mem::take(&mut s.stmts).into_iter().partition(|s| {
                s.is_static()
                    || s.get_attributes().has(ir::NumAttr::PromoteStatic)
            });
        if !s_stmts.is_empty() {
            let s_par = self.construct_static_par(&mut builder, &mut s_stmts);
            if d_stmts.is_empty() {
                return Ok(Action::change(s_par));
            } else {
                new_stmts.push(s_par);
            }
        }
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
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(sif) = s.make_static() {
            return Ok(Action::change(ir::Control::Static(
                ir::StaticControl::If(sif),
            )));
        }
        Ok(Action::Continue)
    }

    // upgrades @bound while loops to static repeats
    fn finish_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if s.body.is_static() {
            // checks body is static and we have an @bound annotation
            if let Some(num_repeats) = s.attributes.get(ir::NumAttr::Bound) {
                // need to do this weird thing to get the while body
                let empty = Box::new(ir::Control::empty());
                let while_body = std::mem::replace(&mut s.body, empty);
                if let ir::Control::Static(sc) = *while_body {
                    let static_repeat =
                        ir::StaticControl::Repeat(ir::StaticRepeat {
                            latency: num_repeats * sc.get_latency(),
                            attributes: s.attributes.clone(),
                            body: Box::new(sc),
                            num_repeats,
                        });
                    return Ok(Action::Change(Box::new(ir::Control::Static(
                        static_repeat,
                    ))));
                } else {
                    unreachable!("already checked that body is static");
                }
            }
        }
        Ok(Action::Continue)
    }
}
