use std::collections::HashMap;
use ir::{StaticTiming, build_assignments, structure, Attributes};

use crate::{traversal::{Named, Visitor}, analysis};
use petgraph::{
  algo,
  graph::NodeIndex,
};
use calyx_ir as ir;
use crate::traversal::Action;

#[derive(Default)]
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
      if s.attributes.has(ir::NumAttr::Compactable) {
        for stmt in s.stmts.iter() {
          match stmt {
              ir::StaticControl::Enable(_) => {}
              _ => { return Ok(Action::Continue);}
          }
        }

        let mut dependency: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
        let mut latency_map: HashMap<NodeIndex, u64> = HashMap::new();
        let mut schedule: HashMap<NodeIndex, u64> = HashMap::new();
        let mut builder = ir::Builder::new(comp, sigs);


        let mut total_order =
            analysis::ControlOrder::<false>::get_total_order_static_seq(s.stmts.drain(..), &mut dependency, &mut latency_map);

        if let Ok(order) = algo::toposort(&total_order, None) {

          let mut group_assignments: Vec<ir::Assignment<StaticTiming>> = Vec::new();
          let mut total_time:u64 = 0;
          
          for i in order {
            let mut start:u64 = 0;
            for node in dependency.get(&i).unwrap() {
              let allow_start = schedule[node] + latency_map[node];
              if allow_start > start {
                start = allow_start;
              }
            }
            schedule.insert(i, start);

            let control = total_order[i].take().unwrap();
            
            
            match control {
              ir::StaticControl::Enable(en) => {
                let timing_guard = ir::Guard::Info(ir::StaticTiming::new((start, start + latency_map[&i] - 1)));
                structure!(
                  builder;
                  let one = constant(1, 1);
                );
                let group = en.group;
                let assignments = build_assignments!( builder;
                  group["go"] = timing_guard ? one["out"];
                );
                if start + latency_map[&i] > total_time {
                  total_time = start + latency_map[&i];
                }
                group_assignments.extend(assignments);
              }
              _ => {}
            }
            
            
          }

          let group = builder.add_static_group("compact_seq", total_time);
          group.borrow_mut().assignments.extend(group_assignments);
          let s_enable = ir::StaticEnable{
            group,
            attributes: Attributes::default()
                    };
          return Ok(Action::static_change(ir::StaticControl::Enable(s_enable)));
        }
      }
      Ok(Action::Continue)
  }
}