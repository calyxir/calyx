use crate::analysis::GraphColoring;
use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::structure;
use calyx_ir::LibrarySignatures;
use calyx_ir::{self as ir, StaticTiming};
use ir::build_assignments;
use itertools::Itertools;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Default)]
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
    // updates single assignment in the same way `update_assignments_timing` does
    // adds offset to each timing guard in `assigns`
    // e.g., %[2,3] with offset = 2 -> %[4,5]
    // all guards also must update so that guard -> guard & %[offset, offset+latency] since that
    // is when the group will be active in the control, i.e., dst = guard ? src
    // becomes dst = guard & %[offset, offset+latency] ? src
    fn update_assignment_timing(
        assign: &mut ir::Assignment<ir::StaticTiming>,
        offset: u64,
        latency: u64,
    ) {
        // adding the offset to each timing interval
        assign.for_each_interval(|timing_interval| {
            let (beg, end) = timing_interval.get_interval();
            Some(ir::Guard::Info(ir::StaticTiming::new((
                beg + offset,
                end + offset,
            ))))
        });
        // adding the interval %[offset, offset + latency]
        assign
            .guard
            .add_interval(ir::StaticTiming::new((offset, offset + latency)));
    }

    // calls update_assignment_timing on each assignment in assigns, which does the following:
    // adds offset to each timing guard in `assigns`
    // e.g., %[2,3] with offset = 2 -> %[4,5]
    // all guards also must update so that guard -> guard & %[offset, offset+latency] since that
    // is when the group will be active in the control, i.e., dst = guard ? src
    // becomes dst =  guard & %[offset, offset+latency] ? src
    // total_latency is the latency of the entire control block being inlined.
    fn update_assignments_timing(
        assigns: &mut Vec<ir::Assignment<ir::StaticTiming>>,
        offset: u64,
        latency: u64,
        total_latency: u64,
    ) {
        if offset == 0 && latency == total_latency {
            // In this special case, we do nothing, since the timing guards
            // would be redundant.
            return;
        }
        for assign in assigns {
            Self::update_assignment_timing(assign, offset, latency);
        }
    }

    // Makes assignments such that if branches can start executing on the first
    // possible cycle.
    // essentially, on the first cycle, we write port's value into a `cond` = a register.
    // this is because the tru/false branch might alter port's value when it executes
    // cond_wire reads from port on the first cycle, and then cond for the other cycles.
    // this means that all of the tru branch assigns can get the cond_wire ? in front of them,
    // and all false branch assigns can get !cond_wire ? in front of them
    // makes the following assignments:
    // read more here: https://github.com/calyxir/calyx/issues/1344 (specifically
    // the section "Conditionl")
    fn make_cond_assigns(
        cond: ir::RRC<ir::Cell>,
        cond_wire: ir::RRC<ir::Cell>,
        port: ir::RRC<ir::Port>,
        latency: u64,
        builder: &mut ir::Builder,
    ) -> Vec<ir::Assignment<ir::StaticTiming>> {
        structure!( builder;
            let signal_on = constant(1,1);
        );
        let mut cond_assigns = vec![];
        let cycle_0_guard = ir::Guard::Info(ir::StaticTiming::new((0, 1)));
        // = %[1:latency] ?
        let other_cycles_guard =
            ir::Guard::Info(ir::StaticTiming::new((1, latency)));
        // cond.in = port
        let cond_gets_port = builder.build_assignment(
            cond.borrow().get("in"),
            Rc::clone(&port),
            ir::Guard::True,
        );
        // cond_wire.in = %0 ? port
        let cond_wire_gets_port = builder.build_assignment(
            cond_wire.borrow().get("in"),
            port,
            cycle_0_guard.clone(),
        );
        cond_assigns.push(cond_gets_port);
        cond_assigns.push(cond_wire_gets_port);
        let asgns = build_assignments!(builder;
            // cond.write_en = %0 ? 1'd1 (since we also have cond.in = %0 ? port)
            // cond_wire.in = %[1:latency] ? cond.out (since we also have cond_wire.in = %0 ? port)
            cond["write_en"] = cycle_0_guard ? signal_on["out"];
            cond_wire["in"] = other_cycles_guard ? cond["out"];
        );
        cond_assigns.extend(asgns.to_vec());
        cond_assigns
    }

    fn get_offload_latencies(
        sc: &ir::StaticControl,
        cur_latency: u64,
    ) -> Vec<(u64, u64)> {
        match sc {
            ir::StaticControl::Enable(_) => vec![],
            ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => {
                let mut lat = cur_latency;
                let mut res = vec![];
                for stmt in stmts {
                    res.extend(Self::get_offload_latencies(stmt, lat));
                    lat += stmt.get_latency();
                }
                res
            }
            ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
                let mut res = vec![];
                if Self::have_overlapping_offloads(sc) {
                    res.push((cur_latency, cur_latency + sc.get_latency()))
                } else {
                    for stmt in stmts {
                        res.extend(Self::get_offload_latencies(
                            stmt,
                            cur_latency,
                        ));
                    }
                }
                res
            }
            ir::StaticControl::If(ir::StaticIf {
                tbranch, fbranch, ..
            }) => {
                let mut res =
                    Self::get_offload_latencies(&tbranch, cur_latency);
                res.extend(Self::get_offload_latencies(&fbranch, cur_latency));
                res
            }
            ir::StaticControl::Repeat(ir::StaticRepeat {
                num_repeats,
                body,
                ..
            }) => {
                let res = vec![(
                    cur_latency,
                    cur_latency + num_repeats * body.get_latency(),
                )];
                // Ths commented code is unnecessary since we only care about
                // intersections, and the above interval covers all intersections.
                // let mut lat = cur_latency;
                // for _ in 0..*num_repeats {
                //     res.extend(Self::get_repeat_latencies(&body, lat));
                //     lat += cur_latency;
                // }
                res
            }
            ir::StaticControl::Empty(_) => unreachable!(
                "should not call inline_static_control on empty stmt"
            ),
            ir::StaticControl::Invoke(inv) => {
                dbg!(inv.comp.borrow().name());
                todo!("implement static inlining for invokes")
            }
        }
    }

    fn have_overlapping_offloads(sc: &ir::StaticControl) -> bool {
        match sc {
            ir::StaticControl::Enable(_) => false,
            ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => stmts
                .iter()
                .any(|stmt| Self::have_overlapping_offloads(stmt)),
            ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
                let intervals: Vec<_> = stmts
                    .iter()
                    .map(|stmt| Self::get_offload_latencies(stmt, 0))
                    .collect();
                for (intervals1, intervals2) in
                    intervals.iter().tuple_combinations()
                {
                    for &(start1, end1) in intervals1.iter() {
                        for &(start2, end2) in intervals2.iter() {
                            if (start2 <= end1 && end1 <= end2)
                                || (start1 <= end2 && end2 <= end1)
                            {
                                return true;
                            }
                        }
                    }
                }
                false
                // again we don't have to check because we will check this later
                // on in inline_static_control
                // stmts
                //     .iter()
                //     .any(|stmt| Self::have_overlapping_repeats(stmt))
            }

            ir::StaticControl::If(ir::StaticIf {
                tbranch, fbranch, ..
            }) => {
                Self::have_overlapping_offloads(&tbranch)
                    || Self::have_overlapping_offloads(&fbranch)
            }
            ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
                Self::have_overlapping_offloads(&body)
            }
            ir::StaticControl::Empty(_) => unreachable!(
                "should not call inline_static_control on empty stmt"
            ),
            ir::StaticControl::Invoke(inv) => {
                dbg!(inv.comp.borrow().name());
                todo!("implement static inlining for invokes")
            }
        }
    }

    // inlines the static control `sc` and returns an equivalent single static group
    fn inline_static_control(
        sc: &ir::StaticControl,
        builder: &mut ir::Builder,
    ) -> ir::RRC<ir::StaticGroup> {
        match sc {
            ir::StaticControl::Enable(ir::StaticEnable { group, .. }) => {
                Rc::clone(group)
            }
            ir::StaticControl::Seq(ir::StaticSeq {
                stmts,
                latency,
                attributes,
            }) => {
                let seq_group =
                    builder.add_static_group("static_seq", *latency);
                let mut seq_group_assigns: Vec<
                    ir::Assignment<ir::StaticTiming>,
                > = vec![];
                let mut cur_offset = 0;
                for stmt in stmts {
                    let stmt_latency = stmt.get_latency();
                    // first recursively call each stmt in seq, and turn each stmt
                    // into static group g.
                    let g = StaticInliner::inline_static_control(stmt, builder);
                    assert!(
                        g.borrow().get_latency() == stmt_latency,
                        "static group latency doesn't match static stmt latency"
                    );
                    // get the assignments from g
                    // currently we clone, since we might need these assignments elsewhere
                    // We could probably do some sort of analysis to see when we need to
                    // clone vs. can drain
                    let mut g_assigns: Vec<ir::Assignment<ir::StaticTiming>> =
                        g.borrow_mut().assignments.clone();
                    // add cur_offset to each static guard in g_assigns
                    // and add %[offset, offset + latency] to each assignment in
                    // g_assigns
                    StaticInliner::update_assignments_timing(
                        &mut g_assigns,
                        cur_offset,
                        stmt_latency,
                        *latency,
                    );
                    // add g_assigns to seq_group_assigns
                    seq_group_assigns.extend(g_assigns.into_iter());
                    // updates cur_offset so that next stmt gets its static timing
                    // offset appropriately
                    cur_offset += stmt_latency;
                }
                assert!(
                    *latency == cur_offset,
                    "static group latency doesn't match static seq latency"
                );
                seq_group.borrow_mut().assignments = seq_group_assigns;
                seq_group.borrow_mut().attributes = attributes.clone();
                seq_group
            }
            ir::StaticControl::Par(ir::StaticPar {
                stmts,
                latency,
                attributes,
            }) => {
                let mut conflict_graph: GraphColoring<usize> =
                    GraphColoring::from(0..stmts.len());
                let offload_interval_info = stmts
                    .iter()
                    .map(|stmt| Self::get_offload_latencies(stmt, 0))
                    .collect_vec();
                for (i, j) in (0..stmts.len()).tuple_combinations() {
                    let intervals1 = &offload_interval_info[i];
                    let intervals2 = &offload_interval_info[j];
                    for &(start1, end1) in intervals1.iter() {
                        for &(start2, end2) in intervals2.iter() {
                            if (start2 <= end1 && end1 <= end2)
                                || (start1 <= end2 && end2 <= end1)
                            {
                                conflict_graph.insert_conflict(&i, &j);
                            }
                        }
                    }
                }
                let threads_to_colors = conflict_graph.color_greedy(None, true);
                let mut colors_to_threads: BTreeMap<usize, Vec<_>> =
                    BTreeMap::new();
                for (thread, color) in &threads_to_colors {
                    colors_to_threads.entry(*color).or_default().push(thread);
                }
                let colors_to_latencies: BTreeMap<usize, u64> =
                    colors_to_threads
                        .into_iter()
                        .map(|(color, threads)| {
                            (
                                color,
                                threads
                                    .iter()
                                    .map(|thread| {
                                        stmts
                                            .get(**thread)
                                            .unwrap()
                                            .get_latency()
                                    })
                                    .max()
                                    .unwrap(),
                            )
                        })
                        .collect();

                let mut threads: BTreeMap<
                    usize,
                    Vec<ir::Assignment<ir::StaticTiming>>,
                > = BTreeMap::new();
                for (index, stmt) in stmts.iter().enumerate() {
                    let stmt_latency = stmt.get_latency();
                    let color_latency = *colors_to_latencies
                        .get(&threads_to_colors[&index])
                        .expect("coloring has gone wrong somehow");
                    // recursively turn each stmt in the par block into a group g
                    let stmt_group =
                        StaticInliner::inline_static_control(stmt, builder);
                    assert!(
                        stmt_group.borrow().get_latency() == stmt_latency,
                        "static group latency doesn't match static stmt latency"
                    );
                    let mut group_assigns =
                        stmt_group.borrow().assignments.clone();
                    if stmt_latency < color_latency {
                        group_assigns.iter_mut().for_each(|assign| {
                            assign.guard.add_interval(StaticTiming::new((
                                0,
                                stmt_latency,
                            )))
                        })
                    }
                    threads
                        .entry(*threads_to_colors.get(&index).unwrap())
                        .or_default()
                        .extend(group_assigns);
                }

                let mut groups = threads
                    .into_iter()
                    .map(|(index, assigns)| {
                        let thread_group = builder.add_static_group(
                            "static_par_thread",
                            *colors_to_latencies.get(&index).expect(
                                "something has gone wrong merging par threads",
                            ),
                        );
                        thread_group.borrow_mut().assignments = assigns;
                        thread_group
                    })
                    .collect_vec();

                if groups.len() == 1 {
                    let par_group = groups.pop().unwrap();
                    par_group.borrow_mut().attributes = attributes.clone();
                    par_group
                } else {
                    let par_group =
                        builder.add_static_group("static_par", *latency);
                    let mut par_group_assigns: Vec<
                        ir::Assignment<ir::StaticTiming>,
                    > = vec![];
                    for group in groups {
                        // Assign par_group[go] = %[0:par_latency] ? 1'd1;
                        structure!( builder;
                            let signal_on = constant(1,1);
                        );
                        let stmt_guard =
                            ir::Guard::Info(ir::StaticTiming::new((
                                0,
                                group.borrow().get_latency(),
                            )));
                        let trigger_body = build_assignments!(builder;
                            group["go"] = stmt_guard ? signal_on["out"];
                        );
                        par_group_assigns.extend(trigger_body);
                    }

                    par_group.borrow_mut().assignments = par_group_assigns;
                    par_group.borrow_mut().attributes = attributes.clone();
                    par_group
                        .borrow_mut()
                        .attributes
                        .insert(ir::BoolAttr::ParCtrl, 1);
                    let par_wrapper = builder
                        .add_static_group("static_par_wrapper", *latency);
                    structure!( builder;
                        let signal_on = constant(1,1);
                    );
                    let trigger_body = build_assignments!(builder;
                        par_group["go"] = ? signal_on["out"];
                    );
                    // par_wrapper triggers par_group[go]
                    par_wrapper.borrow_mut().assignments.extend(trigger_body);
                    par_wrapper
                }
            }
            ir::StaticControl::If(ir::StaticIf {
                port,
                tbranch,
                fbranch,
                latency,
                attributes,
            }) => {
                // Making sure max of the two branches latency is the latency
                // of the if statement
                let tbranch_latency = tbranch.get_latency();
                let fbranch_latency = fbranch.get_latency();
                let max_latency =
                    std::cmp::max(tbranch_latency, fbranch_latency);
                assert_eq!(max_latency, *latency, "if group latency and max of the if branch latencies do not match");

                // Inline assignments in tbranch and fbranch, and get resulting
                // tgroup_assigns and fgroup_assigns
                let tgroup =
                    StaticInliner::inline_static_control(tbranch, builder);
                let mut tgroup_assigns: Vec<ir::Assignment<ir::StaticTiming>> =
                    tgroup.borrow_mut().assignments.clone();
                assert_eq!(
                    tbranch_latency,
                    tgroup.borrow().get_latency(),
                    "tru branch and tru branch group latency do not match"
                );
                // turn fgroup (if it exists) into group and put assigns into fgroup_assigns
                let mut fgroup_assigns: Vec<ir::Assignment<ir::StaticTiming>> =
                    match **fbranch {
                        ir::StaticControl::Empty(_) => vec![],
                        _ => {
                            let fgroup = StaticInliner::inline_static_control(
                                fbranch, builder,
                            );
                            assert_eq!(fbranch_latency, fgroup.borrow().get_latency(), "false branch and false branch group latency do not match");
                            let fgroup_assigns: Vec<
                                ir::Assignment<ir::StaticTiming>,
                            > = fgroup.borrow_mut().assignments.clone();
                            fgroup_assigns
                        }
                    };

                // if_group = the eventual group we inline all the assignments
                // into.
                let if_group = builder.add_static_group("static_if", *latency);
                let mut if_group_assigns: Vec<
                    ir::Assignment<ir::StaticTiming>,
                > = vec![];
                if *latency == 1 {
                    // Special case: if latency = 1, we don't need a register
                    // to hold the value of the cond port.
                    let cond_port_guard = ir::Guard::Port(Rc::clone(port));
                    let not_cond_port_guard =
                        ir::Guard::Not(Box::new(cond_port_guard.clone()));
                    tgroup_assigns.iter_mut().for_each(|assign| {
                        // adds the cond_port ? guard
                        assign
                            .guard
                            .update(|guard| guard.and(cond_port_guard.clone()))
                    });
                    fgroup_assigns.iter_mut().for_each(|assign| {
                        // adds the !cond_port ? guard
                        assign.guard.update(|guard| {
                            guard.and(not_cond_port_guard.clone())
                        })
                    });
                } else {
                    // If latency != 1, we do need a register to hold the
                    // value of the cond port.
                    structure!( builder;
                        let cond = prim std_reg(port.borrow().width);
                        let cond_wire = prim std_wire(port.borrow().width);
                    );
                    // build_cond_assigns makes assigns such that
                    // cond_wire.in can guard all of the tru branch assigns,
                    // and !cond_wire.in can guard all fo the false branch assigns
                    let cond_assigns = StaticInliner::make_cond_assigns(
                        Rc::clone(&cond),
                        Rc::clone(&cond_wire),
                        Rc::clone(port),
                        *latency,
                        builder,
                    );
                    if_group_assigns.extend(cond_assigns.to_vec());

                    // need to do two things:
                    // add cond_wire.out ? in front of each tgroup assignment
                    // (and ! cond_wire.out for fgroup assignemnts)
                    // add %[0:tbranch_latency] in front of each tgroup assignment
                    // (and %[0: fbranch_latency]) in front of each fgroup assignment
                    let cond_wire_guard =
                        ir::Guard::Port(cond_wire.borrow().get("out"));
                    let not_cond_wire_guard =
                        ir::Guard::Not(Box::new(cond_wire_guard.clone()));
                    tgroup_assigns.iter_mut().for_each(|assign| {
                        // adds the %[0:tbranch_latency] ? guard
                        Self::update_assignment_timing(
                            assign,
                            0,
                            tbranch_latency,
                        );
                        // adds the cond_wire ? guard
                        assign
                            .guard
                            .update(|guard| guard.and(cond_wire_guard.clone()))
                    });
                    fgroup_assigns.iter_mut().for_each(|assign| {
                        // adds the %[0:fbranch_latency] ? guard
                        Self::update_assignment_timing(
                            assign,
                            0,
                            fbranch_latency,
                        );
                        // adds the !cond_wire ? guard
                        assign.guard.update(|guard| {
                            guard.and(not_cond_wire_guard.clone())
                        })
                    });
                }
                if_group_assigns.extend(tgroup_assigns);
                if_group_assigns.extend(fgroup_assigns);
                if_group.borrow_mut().assignments = if_group_assigns;
                if_group.borrow_mut().attributes = attributes.clone();
                if_group
            }
            ir::StaticControl::Repeat(ir::StaticRepeat {
                latency,
                num_repeats,
                body,
                attributes,
            }) => {
                let repeat_group =
                    builder.add_static_group("static_repeat", *latency);
                // turn body into a group body_group by recursively calling inline_static_control
                let body_group =
                    StaticInliner::inline_static_control(body, builder);
                assert_eq!(*latency, (num_repeats * body_group.borrow().get_latency()), "latency of static repeat is not equal to num_repeats * latency of body");
                // the assignments in the repeat group should simply trigger the
                // body group. So the static group will literally look like:
                // static group static_repeat <num_repeats * body_latency> {body[go] = 1'd1;}
                structure!( builder;
                    let signal_on = constant(1,1);
                );
                let trigger_body = build_assignments!(builder;
                    body_group["go"] = ? signal_on["out"];
                );
                repeat_group.borrow_mut().assignments = trigger_body.to_vec();
                repeat_group.borrow_mut().attributes = attributes.clone();
                repeat_group
            }
            ir::StaticControl::Empty(_) => unreachable!(
                "should not call inline_static_control on empty stmt"
            ),
            ir::StaticControl::Invoke(inv) => {
                dbg!(inv.comp.borrow().name());
                todo!("implement static inlining for invokes")
            }
        }
    }
}

impl Visitor for StaticInliner {
    /// Executed after visiting the children of a [ir::Static] node.
    fn start_static_control(
        &mut self,
        s: &mut ir::StaticControl,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        let replacement_group =
            StaticInliner::inline_static_control(s, &mut builder);
        Ok(Action::Change(Box::new(ir::Control::from(
            ir::StaticControl::from(replacement_group),
        ))))
    }
}
