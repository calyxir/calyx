use crate::analysis::ControlId;
use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use calyx_ir::structure;
use calyx_ir::LibrarySignatures;
use ir::build_assignments;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
pub struct StaticInliner {
    /// maps ids of static control stmts to groups
    map: HashMap<u64, ir::RRC<ir::StaticGroup>>,
}

impl Named for StaticInliner {
    fn name() -> &'static str {
        "static-inline"
    }

    fn description() -> &'static str {
        "Compiles Static Control into a single Static Enable"
    }
}

impl StaticInliner {
    // Updates the assignments so that they have appropriate timing in the
    // inlined static seq group
    // adds offset to each timing guard in `assigns`
    // e.g., %[2,3] with offset = 2 -> %[4,5]
    // all guards also must get a guard = guard & %[offset, offset+latency] since that
    // is when the group will be active in the control
    fn update_assignment_timing(
        assigns: &mut Vec<ir::Assignment<ir::StaticTiming>>,
        offset: u64,
        latency: u64,
    ) {
        for assign in assigns {
            assign.for_each_interval(|timing_interval| {
                let (beg, end) = timing_interval.get_interval();
                Some(ir::Guard::Info(ir::StaticTiming::new((
                    beg + offset,
                    end + offset,
                ))))
            });
            assign.guard.add_interval(ir::StaticTiming::new((
                offset,
                offset + latency,
            )));
        }
    }

    // makes the following assignments:
    // cond.in = port; cond.write_en = %0 ? 1'd1; cond_wire.in = %0 ? port
    // cond_wire.in = %[1:latency] ? cond.out
    fn make_cond_assigns(
        cond: &ir::RRC<ir::Cell>,
        cond_wire: &ir::RRC<ir::Cell>,
        port: &ir::RRC<ir::Port>,
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
            Rc::clone(port),
            ir::Guard::True,
        );
        // cond_wire.in = %0 ? port
        let cond_wire_gets_port = builder.build_assignment(
            cond_wire.borrow().get("in"),
            Rc::clone(port),
            cycle_0_guard.clone(),
        );
        cond_assigns.push(cond_gets_port);
        cond_assigns.push(cond_wire_gets_port);
        let asgns = build_assignments!(builder;
            cond["write_en"] = cycle_0_guard ? signal_on["out"];
            cond_wire["in"] = other_cycles_guard ? cond["out"];
        );
        cond_assigns.extend(asgns.to_vec());
        cond_assigns
    }

    fn inline_static_control(
        sc: &ir::StaticControl,
        builder: &mut ir::Builder,
    ) -> ir::RRC<ir::StaticGroup> {
        match sc {
            ir::StaticControl::Enable(ir::StaticEnable { group, .. }) => {
                Rc::clone(&group)
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
                    let g = StaticInliner::inline_static_control(stmt, builder);
                    assert!(
                        g.borrow().get_latency() == stmt_latency,
                        "static group latency doesn't match static stmt latency"
                    );
                    // get the assignments from g_assigns
                    // currently we clone, since we might need these assignments elsewhere
                    // *BUT* there are only specific cases where we clone: speciifcally,
                    // if g was actually defined in the design at the beginning of the pass
                    let mut g_assigns: Vec<ir::Assignment<ir::StaticTiming>> =
                        g.borrow_mut().assignments.clone();
                    // add cur_offset to each static guard in g_assigns
                    StaticInliner::update_assignment_timing(
                        &mut g_assigns,
                        cur_offset,
                        stmt_latency,
                    );
                    // add g_assigns to seq_group_assigns
                    seq_group_assigns.extend(g_assigns.into_iter());
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
                    let g = StaticInliner::inline_static_control(stmt, builder);
                    assert!(
                        g.borrow().get_latency() == stmt_latency,
                        "static group latency doesn't match static stmt latency"
                    );
                    // get the assignments from g_assigns
                    // see note on the StaticControl::Seq(..) case abt why we need to clone
                    let mut g_assigns: Vec<ir::Assignment<ir::StaticTiming>> =
                        g.borrow_mut().assignments.clone();
                    // offset = 0 (all start at beginning of par),
                    // but still should add %[0:par_latency?] to beginning of group
                    StaticInliner::update_assignment_timing(
                        &mut g_assigns,
                        0,
                        stmt_latency,
                    );
                    // add g_assigns to seq_group_assigns
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
                let if_group = builder.add_static_group("static_if", *latency);
                let mut if_group_assigns: Vec<
                    ir::Assignment<ir::StaticTiming>,
                > = vec![];
                structure!( builder;
                    let cond = prim std_reg(port.borrow().width);
                    let cond_wire = prim std_wire(port.borrow().width);
                );
                let cond_assigns = StaticInliner::make_cond_assigns(
                    &cond, &cond_wire, port, *latency, builder,
                );
                if_group_assigns.extend(cond_assigns.to_vec());
                let tbranch_latency = tbranch.get_latency();
                let fbranch_latency = fbranch.get_latency();
                // turn tbranch into group and put assigns into tgroup_assigns
                let tgroup =
                    StaticInliner::inline_static_control(tbranch, builder);
                assert_eq!(
                    tbranch_latency,
                    tgroup.borrow().get_latency(),
                    "tru branch and tru branch group latency do not match"
                );
                let mut tgroup_assigns: Vec<ir::Assignment<ir::StaticTiming>> =
                    tgroup.borrow_mut().assignments.clone();
                // turn fgroup into group and put assigns into fgroup_assigns
                let mut fgroup_assigns: Vec<ir::Assignment<ir::StaticTiming>> =
                    match **fbranch {
                        ir::StaticControl::Empty(_) => vec![],
                        _ => {
                            let fgroup = StaticInliner::inline_static_control(
                                fbranch, builder,
                            );
                            assert_eq!(fbranch_latency, fgroup.borrow().get_latency(), "tru branch and tru branch group latency do not match");
                            let fgroup_assigns: Vec<
                                ir::Assignment<ir::StaticTiming>,
                            > = fgroup.borrow_mut().assignments.clone();
                            fgroup_assigns
                        }
                    };

                // update trgoup_assigns to have guard %[0:tbranch_latency] in front of
                // each assignment, and do the same (except w/ fbranch_latency) for fbranch
                StaticInliner::update_assignment_timing(
                    &mut tgroup_assigns,
                    0,
                    tbranch_latency,
                );
                StaticInliner::update_assignment_timing(
                    &mut fgroup_assigns,
                    0,
                    fbranch_latency,
                );
                // add cond_wire.out ? in front of each tgroup assignment
                // add !cond_wire.out ? in front of each fgroup assignment
                let cond_wire_guard =
                    ir::Guard::Port(cond_wire.borrow().get("out"));
                let not_cond_wire_guard =
                    ir::Guard::Not(Box::new(cond_wire_guard.clone()));
                tgroup_assigns.iter_mut().for_each(|assign| {
                    assign
                        .guard
                        .update(|guard| guard.and(cond_wire_guard.clone()))
                });
                if_group_assigns.extend(tgroup_assigns);
                fgroup_assigns.iter_mut().for_each(|assign| {
                    assign
                        .guard
                        .update(|guard| guard.and(not_cond_wire_guard.clone()))
                });
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
                let body_group =
                    StaticInliner::inline_static_control(body, builder);
                assert_eq!(*latency, (num_repeats * body_group.borrow().get_latency()), "latency of static repeat is not equal to num_repeats * latency of body");
                // the assignments in the repeat group should simply trigger the
                // body group
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
            ir::StaticControl::Invoke(_) => {
                todo!("implement static inlining for invokes")
            }
        }
    }

    fn build_static_map(
        &mut self,
        con: &ir::Control,
        builder: &mut ir::Builder,
    ) {
        match con {
            ir::Control::Enable(_)
            | ir::Control::Empty(_)
            | ir::Control::Invoke(_) => (),
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                for stmt in stmts {
                    self.build_static_map(stmt, builder);
                }
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                self.build_static_map(tbranch, builder);
                self.build_static_map(fbranch, builder);
            }
            ir::Control::While(ir::While { body, .. }) => {
                self.build_static_map(body, builder);
            }
            ir::Control::Static(sc) => {
                let id = ControlId::get_guaranteed_id_static(sc);
                let sgroup = Self::inline_static_control(sc, builder);
                self.map.insert(id, sgroup);
            }
        }
    }
}

impl Visitor for StaticInliner {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // assign unique ids so we can use them in our map
        ControlId::compute_unique_ids(&mut comp.control.borrow_mut(), 0, false);
        let control_ref = Rc::clone(&comp.control);
        let mut builder = ir::Builder::new(comp, sigs);
        self.build_static_map(&control_ref.borrow(), &mut builder);
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::Static] node.
    fn start_static_control(
        &mut self,
        s: &mut ir::StaticControl,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let id = ControlId::get_guaranteed_id_static(s);
        match self.map.remove(&id) {
            None => unreachable!("self.map has no entry for id {}", id),
            Some(sgroup) => Ok(Action::Change(Box::new(
                ir::Control::static_control(ir::StaticControl::enable(sgroup)),
            ))),
        }
    }

    // fn finish(
    //     &mut self,
    //     comp: &mut ir::Component,
    //     _sigs: &LibrarySignatures,
    //     _comps: &[ir::Component],
    // ) -> VisResult {
    //     // remove empty static groups
    //     comp.get_static_groups_mut()
    //         .retain(|sg| !sg.borrow().assignments.is_empty());
    //     Ok(Action::Continue)
    // }
}
