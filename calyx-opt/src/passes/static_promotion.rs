use crate::analysis::{InferenceAnalysis, PromotionAnalysis};
use crate::traversal::{
    Action, ConstructVisitor, Named, Order, ParseVal, PassOpt, VisResult,
    Visitor,
};
use calyx_ir::{self as ir, LibrarySignatures};
use calyx_utils::CalyxResult;
use ir::GetAttributes;
use itertools::Itertools;
use std::num::NonZeroU64;
use std::rc::Rc;

const APPROX_ENABLE_SIZE: u64 = 1;
const APPROX_IF_SIZE: u64 = 3;
const APPROX_WHILE_REPEAT_SIZE: u64 = 3;

/// Promote control to static when (conservatively) possible, using @promote_static
/// annotations from `infer_static`.
///
/// Promotion occurs the following policies:
/// 1. ``Threshold'': How large the island must be. We have three const
/// defined as heuristics to measure approximately how big each control program
/// is. It must be larger than that threshold.
/// 2. ``Cycle limit": The maximum number of cycles the island can be when we
/// promote it.
/// 3. ``If Diff Limit": The maximum difference in latency between if statments
/// that we can tolerate to promote it.
///
pub struct StaticPromotion {
    /// An InferenceAnalysis object so that we can re-infer the latencies of
    /// certain components.
    inference_analysis: InferenceAnalysis,
    /// PromotionAnalysis object so that we can easily infer control, and keep
    /// track of which groups were promoted.
    promotion_analysis: PromotionAnalysis,
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
            inference_analysis: InferenceAnalysis::from_ctx(ctx),
            promotion_analysis: PromotionAnalysis::default(),
            threshold: opts["threshold"].pos_num().unwrap(),
            if_diff_limit: opts["if-diff-limit"].pos_num(),
            cycle_limit: opts["cycle-limit"].pos_num(),
        })
    }

    // This pass shared information between components
    fn clear_data(&mut self) {
        self.promotion_analysis = PromotionAnalysis::default();
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

    fn approx_size_static(sc: &ir::StaticControl, promoted: bool) -> u64 {
        if !(sc.get_attributes().has(ir::BoolAttr::Promoted) || promoted) {
            return APPROX_ENABLE_SIZE;
        }
        match sc {
            ir::StaticControl::Empty(_) => 0,
            ir::StaticControl::Enable(_) | ir::StaticControl::Invoke(_) => {
                APPROX_ENABLE_SIZE
            }
            ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
                Self::approx_size_static(body, true) + APPROX_WHILE_REPEAT_SIZE
            }
            ir::StaticControl::If(ir::StaticIf {
                tbranch, fbranch, ..
            }) => {
                Self::approx_size_static(tbranch, true)
                    + Self::approx_size_static(fbranch, true)
                    + APPROX_IF_SIZE
            }
            ir::StaticControl::Par(ir::StaticPar { stmts, .. })
            | ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => stmts
                .iter()
                .map(|stmt| Self::approx_size_static(stmt, true))
                .sum(),
        }
    }

    /// Calculates the approximate "size" of the control statements.
    /// Tries to approximate the number of dynamic FSM transitions that will occur
    fn approx_size(c: &ir::Control) -> u64 {
        match c {
            ir::Control::Empty(_) => 0,
            ir::Control::Enable(_) | ir::Control::Invoke(_) => {
                APPROX_ENABLE_SIZE
            }
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
            ir::Control::Static(sc) => Self::approx_size_static(sc, false),
        }
    }

    /// Uses `approx_size` function to sum the sizes of the control statements
    /// in the given vector
    fn approx_control_vec_size(v: &[ir::Control]) -> u64 {
        v.iter().map(Self::approx_size).sum()
    }

    /// Converts the control_vec (i..e, the stmts of the seq) using heuristics.
    fn promote_vec_seq_heuristic(
        &mut self,
        builder: &mut ir::Builder,
        mut control_vec: Vec<ir::Control>,
    ) -> Vec<ir::Control> {
        if control_vec.is_empty() {
            // Base case
            return vec![];
        } else if control_vec.len() == 1 {
            return vec![control_vec.pop().unwrap()];
        } else if Self::approx_control_vec_size(&control_vec) <= self.threshold
        {
            // Too small to be promoted, return as is
            return control_vec;
        } else if !self.within_cycle_limit(
            control_vec
                .iter()
                .map(PromotionAnalysis::get_inferred_latency)
                .sum(),
        ) {
            // Too large, try to break up
            let right = control_vec.split_off(control_vec.len() / 2);
            dbg!(control_vec.len());
            dbg!(right.len());
            let mut left_res =
                self.promote_vec_seq_heuristic(builder, control_vec);
            let right_res = self.promote_vec_seq_heuristic(builder, right);
            left_res.extend(right_res);
            return left_res;
        }
        // Correct size, convert the entire vec
        let s_seq_stmts = self
            .promotion_analysis
            .convert_vec_to_static(builder, control_vec);
        let latency = s_seq_stmts.iter().map(|sc| sc.get_latency()).sum();
        let sseq =
            ir::Control::Static(ir::StaticControl::seq(s_seq_stmts, latency));
        // sseq.get_mut_attributes()
        //     .insert(ir::NumAttr::Compactable, 1);
        vec![sseq]
    }

    /// First checks if the vec of control statements meets the self.threshold
    /// and is within self.cycle_limit
    /// If so, converts vec of control to a static par, and returns a vec containing
    /// the static par.
    /// Otherwise, just returns the vec without changing it.
    fn promote_vec_par_heuristic(
        &mut self,
        builder: &mut ir::Builder,
        mut control_vec: Vec<ir::Control>,
    ) -> Vec<ir::Control> {
        if control_vec.is_empty() {
            // Base case
            return vec![];
        } else if control_vec.len() == 1 {
            return vec![control_vec.pop().unwrap()];
        } else if Self::approx_control_vec_size(&control_vec) <= self.threshold
        {
            // Too small to be promoted, return as is
            return control_vec;
        } else if !self.within_cycle_limit(
            control_vec
                .iter()
                .map(PromotionAnalysis::get_inferred_latency)
                .max()
                .unwrap_or_else(|| unreachable!("Empty Par Block")),
        ) {
            // Too large to be promoted, take out largest thread and try to promote rest.
            // Can safely unwrap bc we already checked for an empty vector.
            let (index, _) = control_vec
                .iter()
                .enumerate()
                .max_by_key(|&(_, c)| Self::approx_size(c))
                .unwrap();
            // Pop the largest element from the vector
            let largest_thread = control_vec.remove(index);
            let mut left = self.promote_vec_par_heuristic(builder, control_vec);
            left.push(largest_thread);
            return left;
        }
        // Convert vec to static par
        let s_par_stmts = self
            .promotion_analysis
            .convert_vec_to_static(builder, control_vec);
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
        if comp.name != "main" {
            let comp_sig = comp.signature.borrow();
            let go_ports =
                comp_sig.find_all_with_attr(ir::NumAttr::Go).collect_vec();
            if go_ports.iter().any(|go_port| {
                go_port.borrow_mut().attributes.has(ir::NumAttr::Interval)
            }) {
                if comp.control.borrow().is_static() {
                    // We ended up promoting it
                    if !comp.is_static() {
                        // Need this attribute for a weird, in-between state.
                        // It has a known latency but also produces a done signal.
                        comp.attributes.insert(ir::BoolAttr::Promoted, 1);
                    }
                    // This makes the component appear as a static<n> component.
                    comp.latency = Some(
                        NonZeroU64::new(
                            comp.control.borrow().get_latency().unwrap(),
                        )
                        .unwrap(),
                    );
                } else {
                    // We decided not to promote, so we need to update data structures
                    // and remove @static attribute from the signature.

                    // Updating `static_info`.
                    self.inference_analysis.remove_component(comp.name);
                }
            };
            // Either we have upgraded component to static<n> or we have decided
            // not to promote component at all. Either way, we should remove the
            // @interval attribute.
            for go_port in go_ports {
                go_port
                    .borrow_mut()
                    .attributes
                    .remove(ir::NumAttr::Interval);
            }
        }
        // Remove @promotable (i.e., @promote_static) attribute from control.
        // Probably not necessary, since we'll ignore it anyways, but makes for
        // cleaner code.
        InferenceAnalysis::remove_promotable_attribute(
            &mut comp.control.borrow_mut(),
        );
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
        self.inference_analysis.fixup_timing(comp);
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
            // Convert to static if enable is
            // within cycle limit and size is above threshold.
            if self.within_cycle_limit(latency)
                && (APPROX_ENABLE_SIZE > self.threshold)
            {
                return Ok(Action::change(ir::Control::Static(
                    self.promotion_analysis
                        .convert_enable_to_static(s, &mut builder),
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
            // Convert to static if within cycle limit and size is above threshold.
            if self.within_cycle_limit(latency)
                && (APPROX_ENABLE_SIZE > self.threshold)
            {
                return Ok(Action::change(ir::Control::Static(
                    self.promotion_analysis.convert_invoke_to_static(s),
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
        // Checking if entire seq is promotable
        if let Some(latency) = s.attributes.get(ir::NumAttr::PromoteStatic) {
            // If seq is too small to promote, then continue without doing anything.
            if Self::approx_control_vec_size(&s.stmts) <= self.threshold {
                return Ok(Action::Continue);
            } else if self.within_cycle_limit(latency) {
                // We promote entire seq.
                let sseq = ir::Control::Static(ir::StaticControl::seq(
                    self.promotion_analysis.convert_vec_to_static(
                        &mut builder,
                        std::mem::take(&mut s.stmts),
                    ),
                    latency,
                ));
                // sseq.get_mut_attributes()
                //     .insert(ir::NumAttr::Compactable, 1);
                return Ok(Action::change(sseq));
            }
        }
        // The seq either a) takes too many cylces to promote entirely or
        // b) has dynamic stmts in it. Either way, the solution is to
        // break it up into smaller static seqs.
        // We know that this seq will *never* be promoted. Therefore, we can
        // safely replace it with a standard `seq` that does not have an `@promotable`
        // attribute. This temporarily messes up  its parents' `@promotable`
        // attribute, but this is fine since we know its parent will never try
        // to promote it.
        let old_stmts = std::mem::take(&mut s.stmts);
        let mut new_stmts: Vec<ir::Control> = Vec::new();
        let mut cur_vec: Vec<ir::Control> = Vec::new();
        for stmt in old_stmts {
            if PromotionAnalysis::can_be_promoted(&stmt) {
                cur_vec.push(stmt);
            } else {
                // Use heuristics to decide how to promote this cur_vec of promotable stmts.
                let possibly_promoted_stmts =
                    self.promote_vec_seq_heuristic(&mut builder, cur_vec);
                new_stmts.extend(possibly_promoted_stmts);
                // Add the current (non-promotable) stmt
                new_stmts.push(stmt);
                // New cur_vec
                cur_vec = Vec::new();
            }
        }
        new_stmts.extend(self.promote_vec_seq_heuristic(&mut builder, cur_vec));
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
        // Check if entire par is promotable
        if let Some(latency) = s.attributes.get(ir::NumAttr::PromoteStatic) {
            let approx_size: u64 = s.stmts.iter().map(Self::approx_size).sum();
            if approx_size <= self.threshold {
                // Par is too small to promote, continue.
                return Ok(Action::Continue);
            } else if self.within_cycle_limit(latency) {
                // Promote entire par
                let spar = ir::Control::Static(ir::StaticControl::par(
                    self.promotion_analysis.convert_vec_to_static(
                        &mut builder,
                        std::mem::take(&mut s.stmts),
                    ),
                    latency,
                ));
                return Ok(Action::change(spar));
            }
        }
        let mut new_stmts: Vec<ir::Control> = Vec::new();
        // The par either a) takes too many cylces to promote entirely or
        // b) has dynamic stmts in it. Either way, the solution is to
        // break it up.
        // Split the par into static and dynamic stmts, and use heuristics
        // to choose whether to promote the static ones. This replacement will
        // not have a `@promotable` attribute.
        // This temporarily messes up  its parents' `@promotable`
        // attribute, but this is fine since we know its parent will never try
        // to promote it.
        let (s_stmts, d_stmts): (Vec<ir::Control>, Vec<ir::Control>) = s
            .stmts
            .drain(..)
            .partition(PromotionAnalysis::can_be_promoted);
        new_stmts.extend(self.promote_vec_par_heuristic(&mut builder, s_stmts));
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
            let branch_diff = PromotionAnalysis::get_inferred_latency(
                &s.tbranch,
            )
            .abs_diff(PromotionAnalysis::get_inferred_latency(&s.fbranch));
            if approx_size_if > self.threshold
                && self.within_cycle_limit(latency)
                && self.within_if_diff_limit(branch_diff)
            {
                // Meets size threshold so promote to static
                let static_tbranch = self
                    .promotion_analysis
                    .convert_to_static(&mut s.tbranch, &mut builder);
                let static_fbranch = self
                    .promotion_analysis
                    .convert_to_static(&mut s.fbranch, &mut builder);
                return Ok(Action::change(ir::Control::Static(
                    ir::StaticControl::static_if(
                        Rc::clone(&s.port),
                        Box::new(static_tbranch),
                        Box::new(static_fbranch),
                        latency,
                    ),
                )));
            }
            // If this takes too many cycles, then we will
            // never promote this if statement, meaning we will never promote any
            // of its parents. We can therefore safely remove the `@promotable` attribute.
            // This isn't strictly necessary, but it is helpful for parent control
            // programs applying heuristics.
            if !(self.within_cycle_limit(latency)) {
                s.attributes.remove(ir::NumAttr::PromoteStatic);
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
            let approx_size =
                Self::approx_size(&s.body) + APPROX_WHILE_REPEAT_SIZE;
            // Then check that it fits the heuristics
            if approx_size > self.threshold && self.within_cycle_limit(latency)
            {
                // Turn repeat into static repeat
                let sc = self
                    .promotion_analysis
                    .convert_to_static(&mut s.body, &mut builder);
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
            // If this takes too many cycles, then we will
            // never promote this if statement, meaning we will never promote any
            // of its parents. We can therefore safely remove the `@promotable` attribute.
            // This isn't strictly necessary, but it is helpful for parent control
            // programs applying heuristics.
            if !(self.within_cycle_limit(latency)) {
                s.attributes.remove(ir::NumAttr::PromoteStatic);
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
            let approx_size =
                Self::approx_size(&s.body) + APPROX_WHILE_REPEAT_SIZE;
            if approx_size > self.threshold && self.within_cycle_limit(latency)
            {
                // Meets size threshold, so turn repeat into static repeat
                let sc = self
                    .promotion_analysis
                    .convert_to_static(&mut s.body, &mut builder);
                let static_repeat = ir::StaticControl::repeat(
                    s.num_repeats,
                    latency,
                    Box::new(sc),
                );
                return Ok(Action::Change(Box::new(ir::Control::Static(
                    static_repeat,
                ))));
            }
            // If this takes too many cycles, then we will
            // never promote this if statement, meaning we will never promote any
            // of its parents. We can therefore safely remove the `@promotable` attribute.
            // This isn't strictly necessary, but it is helpful for parent control
            // programs applying heuristics.
            if !(self.within_cycle_limit(latency)) {
                s.attributes.remove(ir::NumAttr::PromoteStatic);
            }
        }
        Ok(Action::Continue)
    }
}
