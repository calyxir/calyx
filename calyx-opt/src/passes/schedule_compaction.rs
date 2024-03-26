use crate::analysis::{
    AssignmentAnalysis, InferenceAnalysis, PromotionAnalysis,
};
use crate::traversal::{Action, ConstructVisitor};
use crate::{
    analysis,
    traversal::{Named, Visitor},
};
use calyx_ir as ir;
use calyx_utils::CalyxResult;
use ir::GetAttributes;
use itertools::Itertools;
use petgraph::{algo, graph::NodeIndex};
use std::collections::HashMap;

/// For static seqs that are statically promoted by the compiler.
/// Aggressively compacts the execution schedule so that the execution
/// order of control operators still respects data dependency
/// Example: see tests/passes/schedule-compaction/schedule-compaction.futil
pub struct ScheduleCompaction {
    inference_analysis: InferenceAnalysis,
    promotion_analysis: PromotionAnalysis,
}

// Override constructor to build latency_data information from the primitives
// library.
impl ConstructVisitor for ScheduleCompaction {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        Ok(ScheduleCompaction {
            inference_analysis: InferenceAnalysis::from_ctx(ctx),
            promotion_analysis: PromotionAnalysis::default(),
        })
    }

    // This pass shared information between components
    fn clear_data(&mut self) {
        self.promotion_analysis = PromotionAnalysis::default()
    }
}

impl Named for ScheduleCompaction {
    fn name() -> &'static str {
        "schedule-compaction"
    }

    fn description() -> &'static str {
        "compact execution scheduled for reschedulable static programs"
    }
}

impl ScheduleCompaction {
    // Compacts `cur_stmts`, and appends the result to `new_stmts`.
    fn append_and_compact(
        &mut self,
        (cont_reads, cont_writes): (
            &Vec<ir::RRC<ir::Cell>>,
            &Vec<ir::RRC<ir::Cell>>,
        ),
        builder: &mut ir::Builder,
        cur_stmts: Vec<ir::Control>,
        new_stmts: &mut Vec<ir::Control>,
    ) {
        if !cur_stmts.is_empty() {
            let og_latency = cur_stmts
                .iter()
                .map(PromotionAnalysis::get_inferred_latency)
                .sum();
            // Try to compact cur_stmts.
            let possibly_compacted_stmt = self.compact_control_vec(
                cur_stmts,
                (cont_reads, cont_writes),
                builder,
                og_latency,
                ir::Attributes::default(),
            );
            new_stmts.push(possibly_compacted_stmt);
        }
    }

    // Given a total_order and sorted schedule, builds a seq based on the original
    // schedule.
    // Note that this function assumes the `total_order`` and `sorted_schedule`
    // represent a completely sequential schedule.
    fn recover_seq(
        mut total_order: petgraph::graph::DiGraph<Option<ir::Control>, ()>,
        sorted_schedule: Vec<(NodeIndex, u64)>,
        attributes: ir::Attributes,
    ) -> ir::Control {
        let stmts = sorted_schedule
            .into_iter()
            .map(|(i, _)| total_order[i].take().unwrap())
            .collect_vec();
        ir::Control::Seq(ir::Seq { stmts, attributes })
    }

    // Takes a vec of ctrl stmts and turns it into a compacted schedule (a static par).
    // If it can't compact at all, then it just returns a `seq` in the `stmts`
    // original order.
    fn compact_control_vec(
        &mut self,
        stmts: Vec<ir::Control>,
        (cont_reads, cont_writes): (
            &Vec<ir::RRC<ir::Cell>>,
            &Vec<ir::RRC<ir::Cell>>,
        ),
        builder: &mut ir::Builder,
        og_latency: u64,
        attributes: ir::Attributes,
    ) -> ir::Control {
        // Records the corresponding node indices that each control program
        // has data dependency on.
        let mut dependency: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
        // Records the latency of corresponding control operator for each
        // node index.
        let mut latency_map: HashMap<NodeIndex, u64> = HashMap::new();
        // Records the scheduled start time of corresponding control operator
        // for each node index.
        let mut schedule: HashMap<NodeIndex, u64> = HashMap::new();

        let mut total_order =
            analysis::ControlOrder::<false>::get_dependency_graph_seq(
                stmts.into_iter(),
                (cont_reads, cont_writes),
                &mut dependency,
                &mut latency_map,
            );

        if let Ok(order) = algo::toposort(&total_order, None) {
            let mut total_time: u64 = 0;

            // First we build the schedule.
            for i in order {
                // Start time is when the latest dependency finishes
                let start = dependency
                    .get(&i)
                    .unwrap()
                    .iter()
                    .map(|node| schedule[node] + latency_map[node])
                    .max()
                    .unwrap_or(0);
                schedule.insert(i, start);
                total_time = std::cmp::max(start + latency_map[&i], total_time);
            }

            // We sort the schedule by start time.
            let mut sorted_schedule: Vec<(NodeIndex, u64)> =
                schedule.into_iter().collect();
            sorted_schedule
                .sort_by(|(k1, v1), (k2, v2)| (v1, k1).cmp(&(v2, k2)));

            if total_time == og_latency {
                // If we can't comapct at all, then just recover the and return
                // the original seq.
                return Self::recover_seq(
                    total_order,
                    sorted_schedule,
                    attributes,
                );
            }

            // Threads for the static par, where each entry is (thread, thread_latency)
            let mut par_threads: Vec<(Vec<ir::Control>, u64)> = Vec::new();

            // We encode the schedule while trying to minimize the number of
            // par threads.
            'outer: for (i, start) in sorted_schedule {
                let control = total_order[i].take().unwrap();
                for (thread, thread_latency) in par_threads.iter_mut() {
                    if *thread_latency <= start {
                        if *thread_latency < start {
                            // Need a no-op group so the schedule starts correctly
                            let no_op = builder.add_static_group(
                                "no-op",
                                start - *thread_latency,
                            );
                            thread.push(ir::Control::Static(
                                ir::StaticControl::Enable(ir::StaticEnable {
                                    group: no_op,
                                    attributes: ir::Attributes::default(),
                                }),
                            ));
                            *thread_latency = start;
                        }
                        thread.push(control);
                        *thread_latency += latency_map[&i];
                        continue 'outer;
                    }
                }
                // We must create a new par thread.
                if start > 0 {
                    // If start > 0, then we must add a delay to the start of the
                    // group.
                    let no_op = builder.add_static_group("no-op", start);
                    let no_op_enable = ir::Control::Static(
                        ir::StaticControl::Enable(ir::StaticEnable {
                            group: no_op,
                            attributes: ir::Attributes::default(),
                        }),
                    );
                    par_threads.push((
                        vec![no_op_enable, control],
                        start + latency_map[&i],
                    ));
                } else {
                    par_threads.push((vec![control], latency_map[&i]));
                }
            }
            // Turn Vec<ir::StaticControl> -> StaticSeq
            let mut par_control_threads: Vec<ir::StaticControl> = Vec::new();
            for (thread, thread_latency) in par_threads {
                let mut promoted_stmts = thread
                    .into_iter()
                    .map(|mut stmt| {
                        self.promotion_analysis
                            .convert_to_static(&mut stmt, builder)
                    })
                    .collect_vec();
                if promoted_stmts.len() == 1 {
                    // Don't wrap in static seq if we don't need to.
                    par_control_threads.push(promoted_stmts.pop().unwrap());
                } else {
                    par_control_threads.push(ir::StaticControl::Seq(
                        ir::StaticSeq {
                            stmts: promoted_stmts,
                            attributes: ir::Attributes::default(),
                            latency: thread_latency,
                        },
                    ));
                }
            }
            // Double checking that we have built the static par correctly.
            let max: Option<u64> =
                par_control_threads.iter().map(|c| c.get_latency()).max();
            assert!(max.unwrap() == total_time, "The schedule expects latency {}. The static par that was built has latency {}", total_time, max.unwrap());

            let mut s_par = ir::StaticControl::Par(ir::StaticPar {
                stmts: par_control_threads,
                attributes: ir::Attributes::default(),
                latency: total_time,
            });
            s_par.get_mut_attributes().insert(ir::BoolAttr::Promoted, 1);
            ir::Control::Static(s_par)
        } else {
            panic!(
                "Error when producing topo sort. Dependency graph has a cycle."
            );
        }
    }
}

impl Visitor for ScheduleCompaction {
    fn iteration_order() -> crate::traversal::Order
    where
        Self: Sized,
    {
        crate::traversal::Order::Post
    }

    fn finish_seq(
        &mut self,
        s: &mut calyx_ir::Seq,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        let (cont_reads, cont_writes) = comp
            .continuous_assignments
            .iter()
            .analysis()
            .reads_and_writes();
        let (cont_reads, cont_writes) =
            (cont_reads.cells().collect(), cont_writes.cells().collect());
        InferenceAnalysis::remove_promotable_from_seq(s);
        self.inference_analysis.fixup_seq(s);

        let mut builder = ir::Builder::new(comp, sigs);

        if let Some(latency) = s.attributes.get(ir::NumAttr::Promotable) {
            // If entire seq is promotable, then we can compact entire thing
            // and replace it with a static<n> construct.
            return Ok(Action::Change(Box::new(self.compact_control_vec(
                std::mem::take(&mut s.stmts),
                (&cont_reads, &cont_writes),
                &mut builder,
                latency,
                std::mem::take(&mut s.attributes),
            ))));
        }

        // We have to break up the seq into portions that we can compact.
        let old_stmts = std::mem::take(&mut s.stmts);
        let mut new_stmts: Vec<ir::Control> = Vec::new();
        let mut cur_stmts: Vec<ir::Control> = Vec::new();

        for stmt in old_stmts {
            if PromotionAnalysis::can_be_promoted(&stmt) {
                cur_stmts.push(stmt);
            } else {
                self.append_and_compact(
                    (&cont_reads, &cont_writes),
                    &mut builder,
                    cur_stmts,
                    &mut new_stmts,
                );
                // Appending the non-promotable statement.
                new_stmts.push(stmt);
                // New cur_vec
                cur_stmts = Vec::new();
            }
        }
        self.append_and_compact(
            (&cont_reads, &cont_writes),
            &mut builder,
            cur_stmts,
            &mut new_stmts,
        );
        Ok(Action::change(ir::Control::Seq(ir::Seq {
            stmts: new_stmts,
            attributes: ir::Attributes::default(),
        })))
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> crate::traversal::VisResult {
        // Re-infer component's latency.
        self.inference_analysis.fixup_timing(comp);
        if comp.name != "main" {
            // Fixup components to show the new latency.
            let comp_sig = comp.signature.borrow();
            let go_ports: Vec<_> =
                comp_sig.find_all_with_attr(ir::NumAttr::Go).collect_vec();
            // We only need to check for the @promotable attribute.
            // The @interval attribute means the component's control is entirely
            // static, meaning it's interval/latency is already locked in, so
            // we know we can't change its control, so no need to change its
            // signature.
            if go_ports.iter().any(|go_port| {
                go_port.borrow_mut().attributes.has(ir::NumAttr::Promotable)
            }) {
                // Getting current latency
                let cur_latency = go_ports
                    .iter()
                    .filter_map(|go_port| {
                        go_port
                            .borrow_mut()
                            .attributes
                            .get(ir::NumAttr::Promotable)
                    })
                    .next()
                    .unwrap();
                // Getting new latency. We know it will exist because compaction
                // does not remove latency information, it just alters it.
                let new_latency = InferenceAnalysis::get_possible_latency(
                    &comp.control.borrow(),
                )
                .unwrap();
                if cur_latency != new_latency {
                    // We adjust the signature of our component
                    self.inference_analysis
                        .adjust_component((comp.name, new_latency));
                    for go_port in go_ports {
                        go_port
                            .borrow_mut()
                            .attributes
                            .insert(ir::NumAttr::Promotable, new_latency);
                    }
                }
            };
        }
        Ok(Action::Continue)
    }

    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> crate::traversal::VisResult {
        self.inference_analysis.fixup_timing(comp);
        Ok(Action::Continue)
    }
}
