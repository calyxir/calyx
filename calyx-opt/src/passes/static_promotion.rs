use crate::analysis::{FixUp, GoDone};
use crate::traversal::{
    Action, ConstructVisitor, Named, Order, ParseVal, PassOpt, VisResult,
    Visitor,
};
use calyx_ir::{self as ir, LibrarySignatures};
use calyx_utils::CalyxResult;
use ir::{GetAttributes, NumAttr};
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::rc::Rc;

const APPROX_ENABLE_SIZE: u64 = 1;
const APPROX_IF_SIZE: u64 = 3;
const APPROX_WHILE_REPEAT_SIZE: u64 = 3;

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
    /// Components whose timing information has been changed by this pass.
    /// For StaticPromotion, this is when we decide not to promote certain components.
    updated_components: HashMap<ir::Id, Option<u64>>,
    // XXX(Caleb): To do;
    static_info: FixUp,
    /// dynamic group Id -> promoted static group Id
    static_group_name: HashMap<ir::Id, ir::Id>,
    /// Threshold for promotion
    threshold: u64,
    /// Threshold for difference in latency for if statements
    if_diff_limit: Option<u64>,
    /// Whether we should stop promoting when we see a loop.
    cycle_limit: Option<u64>,
}

// Override constructor to build latency_data information from the primitives
// library.
impl ConstructVisitor for StaticPromotion {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let opts = Self::get_opts(ctx);
        Ok(StaticPromotion {
            updated_components: HashMap::new(),
            static_info: FixUp::from_ctx(ctx),
            static_group_name: HashMap::new(),
            threshold: opts["threshold"].pos_num().unwrap(),
            if_diff_limit: opts["if-diff-limit"].pos_num(),
            cycle_limit: opts["cycle-limit"].pos_num(),
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
        "promote dynamic control programs to static when possible"
    }

    fn opts() -> Vec<PassOpt> {
        vec![
            PassOpt::new(
                "threshold",
                "minimum number of groups needed to promote a dynamic control program to static",
                ParseVal::Num(1),
                PassOpt::parse_num,
            ),
            PassOpt::new(
                "cycle-limit",
                "maximum number of cycles to promote a dynamic control program to static",
                ParseVal::Num(33554432),
                PassOpt::parse_num,
            ),
            PassOpt::new(
                "if-diff-limit",
                "the maximum difference between if branches that we tolerate for promotion",
                ParseVal::Num(1),
                PassOpt::parse_num,
            )
        ]
    }
}

impl StaticPromotion {
    /// Gets the inferred latency, which should either be from being a static
    /// control operator or the promote_static attribute.
    /// Will raise an error if neither of these is true.
    fn get_inferred_latency(c: &ir::Control) -> u64 {
        let ir::Control::Static(sc) = c else {
            let Some(latency) = c.get_attribute(ir::NumAttr::PromoteStatic)
            else {
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

    fn within_if_diff_limit(&self, diff: u64) -> bool {
        if self.if_diff_limit.is_none() {
            return true;
        }
        diff <= self.if_diff_limit.unwrap()
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
                // Don't need to include assignment to done hole.
                if !(assignment.dst.borrow().is_hole()
                    && assignment.dst.borrow().name == "done")
                {
                    sg.borrow_mut()
                        .assignments
                        .push(ir::Assignment::from(assignment.clone()));
                }
            }
            Rc::clone(&sg)
        }
    }

    // Converts dynamic enable to static
    fn convert_enable_to_static(
        &mut self,
        s: &mut ir::Enable,
        builder: &mut ir::Builder,
    ) -> ir::StaticControl {
        s.attributes.remove(ir::NumAttr::PromoteStatic);
        ir::StaticControl::Enable(ir::StaticEnable {
            // upgrading group to static group
            group: self.construct_static_group(
                builder,
                Rc::clone(&s.group),
                s.group
                    .borrow()
                    .get_attributes()
                    .unwrap()
                    .get(ir::NumAttr::PromoteStatic)
                    .unwrap(),
            ),
            attributes: std::mem::take(&mut s.attributes),
        })
    }

    // Converts dynamic invoke to static
    fn convert_invoke_to_static(
        &mut self,
        s: &mut ir::Invoke,
    ) -> ir::StaticControl {
        assert!(
            s.comb_group.is_none(),
            "Shouldn't Promote to Static if there is a Comb Group",
        );
        s.attributes.remove(ir::NumAttr::PromoteStatic);
        let latency = *self.static_info.static_component_latencies.get(
            &s.comp.borrow().type_name().unwrap_or_else(|| {
                unreachable!(
                    "Already checked that comp is component"
                )
            }),
        ).unwrap_or_else(|| unreachable!("Called convert_to_static for static invoke that does not have a static component"));
        // Self::check_latencies_match(*self.static_info.static_component_latencies.get(
        //     &s.comp.borrow().type_name().unwrap_or_else(|| {
        //         unreachable!(
        //             "Already checked that comp is component"
        //         )
        //     }),
        // ).unwrap_or_else(|| unreachable!("Called convert_to_static for static invoke that does not have a static component")), inferred_latency);
        let s_inv = ir::StaticInvoke {
            comp: Rc::clone(&s.comp),
            inputs: std::mem::take(&mut s.inputs),
            outputs: std::mem::take(&mut s.outputs),
            latency,
            attributes: std::mem::take(&mut s.attributes),
            ref_cells: std::mem::take(&mut s.ref_cells),
            comb_group: std::mem::take(&mut s.comb_group),
        };
        ir::StaticControl::Invoke(s_inv)
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
            ir::Control::Enable(s) => self.convert_enable_to_static(s, builder),
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
                Self::check_latencies_match(*self.static_info.static_component_latencies.get(
                    &comp.borrow().type_name().unwrap_or_else(|| {
                        unreachable!(
                            "Already checked that comp is component"
                        )
                    }),
                ).unwrap_or_else(|| unreachable!("Called convert_to_static for static invoke that does not have a static component")), inferred_latency);
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
            // Invokes are same size as enables.
            ir::Control::Invoke(_) => APPROX_ENABLE_SIZE,
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
                    .unwrap_or_else(|| unreachable!("Empty Par Block")),
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
                    // Need this attribute for a weird, in-between state.
                    // It has a known latency but also produces a done signal.
                    comp.attributes.insert(ir::BoolAttr::Promoted, 1);
                }
                // This makes the component static.
                comp.latency = Some(NonZeroU64::new(lat).unwrap());
            } else {
                // If we ended up not deciding to promote, we need to update static_info
                // and remove @static attribute from the signature.
                self.static_info.latency_data.remove(&comp.name);
                self.static_info
                    .static_component_latencies
                    .remove(&comp.name);
                let comp_sig = comp.signature.borrow();
                let go_ports = comp_sig.find_all_with_attr(ir::NumAttr::Go);
                for go_port in go_ports {
                    if go_port.borrow_mut().attributes.has(ir::NumAttr::Static)
                    {
                        go_port
                            .borrow_mut()
                            .attributes
                            .remove(ir::NumAttr::Static);
                        // Insert comp.name into updated component to signify the
                        // component now has an unknown latency.
                        self.updated_components.insert(comp.name, None);
                    }
                }
            }
        }
        // if comp.is_static() {
        //     self.static_component_latencies
        //         .insert(comp.name, comp.latency.unwrap());
        // }
        Ok(Action::Continue)
    }

    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Re-infer static timing based on the components we have updated in
        // this pass.
        self.static_info
            .fixup_timing(comp, &self.updated_components);
        Ok(Action::Continue)
    }

    fn enable(
        &mut self,
        s: &mut ir::Enable,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        if let Some(latency) = s.attributes.get(ir::NumAttr::PromoteStatic) {
            // Convert to static if within cycle limit and size is below threshold.
            if self.within_cycle_limit(latency)
                && (APPROX_ENABLE_SIZE > self.threshold)
            {
                return Ok(Action::change(ir::Control::Static(
                    self.convert_enable_to_static(s, &mut builder),
                )));
            }
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
        if let Some(latency) = s.attributes.get(ir::NumAttr::PromoteStatic) {
            // Convert to static if within cycle limit and size is below threshold.
            if self.within_cycle_limit(latency)
                && (APPROX_ENABLE_SIZE > self.threshold)
            {
                return Ok(Action::change(ir::Control::Static(
                    self.convert_invoke_to_static(s),
                )));
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
                return Ok(Action::Continue);
            }
        }
        // Entire seq is not static, so we're only (possibly) promoting the cur_vec
        new_stmts.extend(self.convert_vec_seq_if_sat(&mut builder, cur_vec));
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
            s.stmts.drain(..).partition(|c| Self::can_be_promoted(&c));
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
                return Ok(Action::Continue);
            }
        }
        // Otherwise just promote the par threads that we can into a static par
        new_stmts.extend(self.convert_vec_par_if_sat(&mut builder, s_stmts));
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
        if let Some(latency) = s.attributes.get(ir::NumAttr::PromoteStatic) {
            let approx_size_if = Self::approx_size(&s.tbranch)
                + Self::approx_size(&s.fbranch)
                + APPROX_IF_SIZE;
            let branch_diff = Self::get_inferred_latency(&s.tbranch)
                .abs_diff(Self::get_inferred_latency(&s.fbranch));
            if approx_size_if > self.threshold
                && self.within_cycle_limit(latency)
                && self.within_if_diff_limit(branch_diff)
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
        // First check that while loop is promotable
        if let Some(latency) = s.attributes.get(ir::NumAttr::PromoteStatic) {
            // Then check that body is static/promotable
            let approx_size =
                Self::approx_size(&s.body) + APPROX_WHILE_REPEAT_SIZE;
            // Then check that it reaches the threshold
            if approx_size > self.threshold && self.within_cycle_limit(latency)
            {
                // Turn repeat into static repeat
                let sc = self.convert_to_static(&mut s.body, &mut builder);
                let static_repeat = ir::StaticControl::repeat(
                    s.attributes.get(ir::NumAttr::Bound).unwrap_or_else(|| {
                        unreachable!(
                            "Unbounded loop has has @promotable attribute"
                        )
                    }),
                    latency,
                    Box::new(sc),
                );
                return Ok(Action::Change(Box::new(ir::Control::Static(
                    static_repeat,
                ))));
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
        if let Some(latency) = s.attributes.get(ir::NumAttr::PromoteStatic) {
            // Body can be promoted
            let approx_size =
                Self::approx_size(&s.body) + APPROX_WHILE_REPEAT_SIZE;
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
            }
        }
        Ok(Action::Continue)
    }
}
