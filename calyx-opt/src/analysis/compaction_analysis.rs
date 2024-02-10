use crate::analysis::{ControlOrder, PromotionAnalysis};
use calyx_ir::{self as ir};
use ir::GetAttributes;
use itertools::Itertools;
use petgraph::{algo, graph::NodeIndex};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct CompactionAnalysis;

impl CompactionAnalysis {
    // Compacts `cur_stmts`, and appends the result to `new_stmts`.
    pub fn append_and_compact(
        &mut self,
        (cont_reads, cont_writes): (
            &Vec<ir::RRC<ir::Cell>>,
            &Vec<ir::RRC<ir::Cell>>,
        ),
        promotion_analysis: &mut PromotionAnalysis,
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
                promotion_analysis,
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
    pub fn compact_control_vec(
        &mut self,
        stmts: Vec<ir::Control>,
        (cont_reads, cont_writes): (
            &Vec<ir::RRC<ir::Cell>>,
            &Vec<ir::RRC<ir::Cell>>,
        ),
        promotion_analysis: &mut PromotionAnalysis,
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

        let mut total_order = ControlOrder::<false>::get_dependency_graph_seq(
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
                        promotion_analysis.convert_to_static(&mut stmt, builder)
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
