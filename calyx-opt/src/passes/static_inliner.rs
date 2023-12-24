use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use calyx_ir::structure;
use calyx_ir::LibrarySignatures;
use ir::build_assignments;
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
                let par_group =
                    builder.add_static_group("static_par", *latency);
                let mut par_group_assigns: Vec<
                    ir::Assignment<ir::StaticTiming>,
                > = vec![];
                for stmt in stmts {
                    let stmt_latency = stmt.get_latency();
                    // recursively turn each stmt in the par block into a group g
                    let g = StaticInliner::inline_static_control(stmt, builder);
                    assert!(
                        g.borrow().get_latency() == stmt_latency,
                        "static group latency doesn't match static stmt latency"
                    );
                    // get the assignments from g
                    // see note on the StaticControl::Seq(..) case abt why we need to clone
                    let mut g_assigns: Vec<ir::Assignment<ir::StaticTiming>> =
                        g.borrow_mut().assignments.clone();
                    // offset = 0 (all start at beginning of par),
                    // but still should add %[0:stmt_latency] to beginning of group
                    StaticInliner::update_assignments_timing(
                        &mut g_assigns,
                        0,
                        stmt_latency,
                        *latency,
                    );
                    // add g_assigns to par_group_assigns
                    par_group_assigns.extend(g_assigns.into_iter());
                }
                par_group.borrow_mut().assignments = par_group_assigns;
                par_group.borrow_mut().attributes = attributes.clone();
                par_group
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
