use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use calyx_ir::LibrarySignatures;

pub struct StaticInliner;

impl Named for StaticInliner {
    fn name() -> &'static str {
        "static-inline"
    }

    fn description() -> &'static str {
        "Compiles Static Control into a single Static Enable"
    }
}

impl StaticInliner {
    fn offset_assignments_timing(
        assigns: &mut Vec<ir::Assignment<ir::StaticTiming>>,
        offset: u64,
    ) {
        for mut assign in assigns {
            assign.for_each_interval(|timing_interval| {
                let (beg, end) = timing_interval.get_interval();
                Some(ir::Guard::Info(ir::StaticTiming::new((
                    beg + offset,
                    end + offset,
                ))))
            })
        }
    }

    fn inline_static_control(
        sc: ir::StaticControl,
        builder: &mut ir::Builder,
    ) -> ir::RRC<ir::StaticGroup> {
        match sc {
            ir::StaticControl::Seq(ir::StaticSeq {
                stmts,
                latency,
                attributes,
            }) => {
                let seq_group = builder.add_static_group("static_seq", latency);
                let mut seq_group_assigns: Vec<
                    ir::Assignment<ir::StaticTiming>,
                > = vec![];
                let mut cur_offset = 0;
                for stmt in stmts {
                    let latency = stmt.get_latency();
                    let mut g =
                        StaticInliner::inline_static_control(stmt, builder);
                    assert!(g.borrow().get_latency() == latency, "hello");
                    let mut g_assigns: Vec<ir::Assignment<ir::StaticTiming>> =
                        g.borrow_mut().assignments.drain(..).collect();
                    StaticInliner::offset_assignments_timing(
                        &mut g_assigns,
                        cur_offset,
                    );
                    seq_group_assigns.extend(g_assigns.into_iter());
                    cur_offset += latency;
                }
                seq_group.borrow_mut().assignments = seq_group_assigns;
                seq_group
            }
            _ => todo!(""),
        }
    }
}

impl Visitor for StaticInliner {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }
}
