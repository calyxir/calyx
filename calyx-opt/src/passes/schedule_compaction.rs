use crate::traversal::Action;
use crate::{
    analysis,
    traversal::{Named, Visitor},
};
use calyx_ir as ir;
use petgraph::{algo, graph::NodeIndex};
use std::collections::HashMap;

#[derive(Default)]
/// for static seqs that are statically promoted by the compiler,
/// aggressively compacts the execution schedule so that the execution
/// order of control operators still respects data dependency
/// Example: see tests/passes/schedule-compaction/schedule-compaction.rs
pub struct ScheduleCompaction;

impl Named for ScheduleCompaction {
    fn name() -> &'static str {
        "schedule-compaction"
    }

    fn description() -> &'static str {
        "Aggressively compact schedule for static seqs which were promoted from generic seqs"
    }
}

impl Visitor for ScheduleCompaction {
    fn iteration_order() -> crate::traversal::Order
    where
        Self: Sized,
    {
        crate::traversal::Order::Post
    }

    fn finish_static_seq(
        &mut self,
        s: &mut calyx_ir::StaticSeq,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        // records the corresponding node indices that each control program
        // has data dependency on
        let mut dependency: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
        // records the latency of corresponding control operator for each node index
        let mut latency_map: HashMap<NodeIndex, u64> = HashMap::new();
        // records the scheduled start time of corresponding control operator for each node index
        let mut schedule: HashMap<NodeIndex, u64> = HashMap::new();

        let mut builder = ir::Builder::new(comp, sigs);

        let mut total_order =
            analysis::ControlOrder::<false>::get_dependency_graph_static_seq(
                s.stmts.drain(..),
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
            // Threads for the static par, where each entry is (thread, thread_latency)
            let mut par_threads: Vec<(Vec<ir::StaticControl>, u64)> =
                Vec::new();

            // We encode the schedule attempting to minimize the number of
            // par threads.
            'outer: for (i, start) in sorted_schedule {
                let control = total_order[i].take().unwrap();
                for (thread, thread_latency) in par_threads.iter_mut() {
                    if *thread_latency <= start {
                        if *thread_latency < start {
                            // Might need a no-op group so the schedule starts correctly
                            let no_op = builder.add_static_group(
                                "no-op",
                                start - *thread_latency,
                            );
                            thread.push(ir::StaticControl::Enable(
                                ir::StaticEnable {
                                    group: no_op,
                                    attributes: ir::Attributes::default(),
                                },
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
                    let no_op_enable =
                        ir::StaticControl::Enable(ir::StaticEnable {
                            group: no_op,
                            attributes: ir::Attributes::default(),
                        });
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
                par_control_threads.push(ir::StaticControl::Seq(
                    ir::StaticSeq {
                        stmts: thread,
                        attributes: ir::Attributes::default(),
                        latency: thread_latency,
                    },
                ));
            }
            // Double checking that we have built the static par correctly.
            let max = par_control_threads.iter().map(|c| c.get_latency()).max();
            assert!(max.unwrap() == total_time, "The schedule expects latency {}. The static par that was built has latency {}", total_time, max.unwrap());

            if par_control_threads.len() == 1 {
                let c = Vec::pop(&mut par_control_threads).unwrap();
                Ok(Action::static_change(c))
            } else {
                let s_par = ir::StaticControl::Par(ir::StaticPar {
                    stmts: par_control_threads,
                    attributes: ir::Attributes::default(),
                    latency: total_time,
                });
                Ok(Action::static_change(s_par))
            }
        } else {
            panic!(
                "Error when producing topo sort. Dependency graph has a cycle."
            );
        }
    }

    fn finish_static_repeat(
        &mut self,
        s: &mut ir::StaticRepeat,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> crate::traversal::VisResult {
        s.latency = s.body.get_latency() * s.num_repeats;
        Ok(Action::Continue)
    }

    fn finish_static_par(
        &mut self,
        s: &mut ir::StaticPar,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> crate::traversal::VisResult {
        let mut latency: u64 = 0;
        for stmt in s.stmts.iter() {
            latency = std::cmp::max(latency, stmt.get_latency());
        }
        s.latency = latency;
        Ok(Action::Continue)
    }

    fn finish_static_if(
        &mut self,
        s: &mut ir::StaticIf,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> crate::traversal::VisResult {
        s.latency =
            std::cmp::max(s.tbranch.get_latency(), s.fbranch.get_latency());
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> crate::traversal::VisResult {
        if comp.is_static() {
            comp.latency = Some(
                std::num::NonZeroU64::new(
                    comp.control.borrow().get_latency().unwrap(),
                )
                .unwrap(),
            );
        }
        Ok(Action::Continue)
    }

    fn static_invoke(
        &mut self,
        s: &mut ir::StaticInvoke,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        comps: &[ir::Component],
    ) -> crate::traversal::VisResult {
        for comp in comps {
            if comp.name.eq(&s.comp.borrow().type_name().unwrap()) {
                s.latency = u64::from(comp.latency.unwrap());
            }
        }
        Ok(Action::Continue)
    }
}
