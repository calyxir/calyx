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
    fn finish_static_seq(
        &mut self,
        s: &mut calyx_ir::StaticSeq,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        // currently this pass only works for cases where all control operators
        // are static enables
        if s.attributes.has(ir::NumAttr::Compactable) {
            for stmt in s.stmts.iter() {
                match stmt {
                    ir::StaticControl::Enable(_) => {}
                    _ => {
                        return Ok(Action::Continue);
                    }
                }
            }

            // records the corresponding node indices that each control program
            // has data dependency on
            let mut dependency: HashMap<NodeIndex, Vec<NodeIndex>> =
                HashMap::new();
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
                let mut stmts: Vec<ir::StaticControl> = Vec::new();

                for i in order {
                    let mut start: u64 = 0;
                    for node in dependency.get(&i).unwrap() {
                        let allow_start = schedule[node] + latency_map[node];
                        if allow_start > start {
                            start = allow_start;
                        }
                    }
                    schedule.insert(i, start);

                    let control = total_order[i].take().unwrap();
                    let mut st_seq_stmts: Vec<ir::StaticControl> = Vec::new();
                    if start > 0 {
                        let no_op = builder.add_static_group("no-op", start);

                        st_seq_stmts.push(ir::StaticControl::Enable(
                            ir::StaticEnable {
                                group: no_op,
                                attributes: ir::Attributes::default(),
                            },
                        ));
                    }
                    if start + latency_map[&i] > total_time {
                        total_time = start + latency_map[&i];
                    }

                    st_seq_stmts.push(control);
                    stmts.push(ir::StaticControl::Seq(ir::StaticSeq {
                        stmts: st_seq_stmts,
                        attributes: ir::Attributes::default(),
                        latency: start + latency_map[&i],
                    }));
                }

                let s_par = ir::StaticControl::Par(ir::StaticPar {
                    stmts,
                    attributes: ir::Attributes::default(),
                    latency: total_time,
                });
                return Ok(Action::static_change(s_par));
            } else {
                println!("Error when producing topo sort. Dependency graph has a cycle.");
            }
        }
        Ok(Action::Continue)
    }
}
