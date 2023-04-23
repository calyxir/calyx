use super::math_utilities::get_bit_width_from;
use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use calyx_ir::{guard, structure, GetAttributes};
use ir::{build_assignments, Nothing, StaticGroup, StaticTiming};
use itertools::Itertools;
use std::collections::HashMap;
use std::ops::Not;
use std::rc::Rc;

const NODE_ID: &str = "NODE_ID";

#[derive(Default)]
/// Compiles Static Islands
pub struct CompileStatic {
    /// maps static enable ids (u64) to a bool that indicates whether they are
    /// in a dynamic context: true if dynamic context, false if static context
    enable_context_map: HashMap<u64, bool>,
    /// maps original static group names to the corresponding group that has an FSM that reset early
    reset_early_map: HashMap<ir::Id, ir::Id>,
    /// maps group that has an FSM that resets early to its dynamic "wrapper" group name.
    wrapper_map: HashMap<ir::Id, ir::Id>,
    /// maps reset_early_group names to the fsm that they use
    fsm_map: HashMap<ir::Id, ir::Id>,
    /// rewrites `static_group[go]` to `dynamic_group[go]`
    group_rewrite: ir::rewriter::PortRewriteMap,
}

impl Named for CompileStatic {
    fn name() -> &'static str {
        "compile-static"
    }

    fn description() -> &'static str {
        "Compiles Static Islands"
    }
}

// Takes in a static guard `guard`, and returns equivalent dynamic guard
// The only thing that actually changes is the Guard::Info case
// We need to turn static_timing to dynamic guards using `fsm`.
// E.g.: %[2:3] gets turned into fsm.out >= 2 & fsm.out <= 3
fn make_guard_dyn(
    guard: ir::Guard<StaticTiming>,
    fsm: &ir::RRC<ir::Cell>,
    fsm_size: u64,
    builder: &mut ir::Builder,
) -> Box<ir::Guard<Nothing>> {
    match guard {
        ir::Guard::Or(l, r) => Box::new(ir::Guard::Or(
            make_guard_dyn(*l, fsm, fsm_size, builder),
            make_guard_dyn(*r, fsm, fsm_size, builder),
        )),
        ir::Guard::And(l, r) => Box::new(ir::Guard::And(
            make_guard_dyn(*l, fsm, fsm_size, builder),
            make_guard_dyn(*r, fsm, fsm_size, builder),
        )),
        ir::Guard::Not(g) => {
            Box::new(ir::Guard::Not(make_guard_dyn(*g, fsm, fsm_size, builder)))
        }
        ir::Guard::CompOp(op, l, r) => Box::new(ir::Guard::CompOp(op, l, r)),
        ir::Guard::Port(p) => Box::new(ir::Guard::Port(p)),
        ir::Guard::True => Box::new(ir::Guard::True),
        ir::Guard::Info(static_timing) => {
            let (beg, end) = static_timing.get_interval();
            if beg + 1 == end {
                // if beg + 1 == end then we only need to check if fsm == beg
                let interval_const = builder.add_constant(beg, fsm_size);
                let g = guard!(fsm["out"]).eq(guard!(interval_const["out"]));
                Box::new(g)
            } else if beg == 0 {
                // if beg == 0, then we only need to check if fsm < end
                let end_const = builder.add_constant(end, fsm_size);
                let lt: ir::Guard<Nothing> =
                    guard!(fsm["out"]).lt(guard!(end_const["out"]));
                Box::new(lt)
            } else {
                // otherwise, check if fsm >= beg & fsm < end
                let beg_const = builder.add_constant(beg, fsm_size);
                let end_const = builder.add_constant(end, fsm_size);
                let beg_guard: ir::Guard<Nothing> =
                    guard!(fsm["out"]).ge(guard!(beg_const["out"]));
                let end_guard: ir::Guard<Nothing> =
                    guard!(fsm["out"]).lt(guard!(end_const["out"]));
                Box::new(ir::Guard::And(
                    Box::new(beg_guard),
                    Box::new(end_guard),
                ))
            }
        }
    }
}

// Takes in static assignment `assign` and returns a dynamic assignments
// Mainly does two things:
// 1) if `assign` writes to a go/done hole, we should change that to the go/done
// hole of the new, dynamic group instead of the old static group
// 2) for each static Info guard (e.g. %[2:3]), we need to convert that to
// dynamic guards, using `fsm`.
// E.g.: %[2:3] gets turned into fsm.out >= 2 & fsm.out <= 3
fn make_assign_dyn(
    assign: ir::Assignment<StaticTiming>,
    dyn_group: &ir::RRC<ir::Group>,
    fsm: &ir::RRC<ir::Cell>,
    fsm_size: u64,
    builder: &mut ir::Builder,
) -> ir::Assignment<Nothing> {
    // let new_dst = if assign.dst.borrow().is_hole() {
    //     // holes should be either go/done
    //     if assign.dst.borrow().name == "go" {
    //         dyn_group.borrow().get("go")
    //     } else {
    //         panic!("hole port other than go port")
    //     }
    // } else {
    //     // if dst is not a hole, then we should keep it as is for the new assignment
    //     assign.dst
    // };
    let new_dst = assign.dst;
    ir::Assignment {
        src: assign.src,
        dst: new_dst,
        attributes: assign.attributes,
        guard: make_guard_dyn(*assign.guard, fsm, fsm_size, builder),
    }
}

fn add_enable_ids_static(
    scon: &mut ir::StaticControl,
    mut cur_state: u64,
) -> u64 {
    match scon {
        ir::StaticControl::Enable(se) => {
            se.attributes.insert(NODE_ID, cur_state);
            cur_state + 1
        }
        ir::StaticControl::Invoke(_) | ir::StaticControl::Empty(_) => cur_state,
        ir::StaticControl::Par(ir::StaticPar { stmts, .. })
        | ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => {
            for stmt in stmts {
                let new_state = add_enable_ids_static(stmt, cur_state);
                cur_state = new_state
            }
            cur_state
        }
        ir::StaticControl::If(ir::StaticIf {
            tbranch, fbranch, ..
        }) => {
            let mut new_state = add_enable_ids_static(tbranch, cur_state);
            cur_state = new_state;
            new_state = add_enable_ids_static(fbranch, cur_state);
            new_state
        }
        ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
            add_enable_ids_static(body, cur_state)
        }
    }
}

fn add_enable_ids(con: &mut ir::Control, mut cur_state: u64) -> u64 {
    match con {
        ir::Control::Enable(_)
        | ir::Control::Invoke(_)
        | ir::Control::Empty(_) => cur_state,
        ir::Control::Par(ir::Par { stmts, .. })
        | ir::Control::Seq(ir::Seq { stmts, .. }) => {
            for stmt in stmts {
                let new_state = add_enable_ids(stmt, cur_state);
                cur_state = new_state
            }
            cur_state
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            let mut new_state = add_enable_ids(tbranch, cur_state);
            cur_state = new_state;
            new_state = add_enable_ids(fbranch, cur_state);
            new_state
        }
        ir::Control::While(ir::While { body, .. }) => {
            add_enable_ids(body, cur_state)
        }
        ir::Control::Static(s) => add_enable_ids_static(s, cur_state),
    }
}

// Gets attribute s from c, panics otherwise. Should be used when you know
// that c has attribute s.
fn get_guaranteed_enable_id(se: &ir::StaticEnable) -> u64 {
    se.get_attribute(NODE_ID).unwrap_or_else(||unreachable!(
          "called get_guaranteed_enable_id, meaning we had to be sure it had a NODE_ID attribute"
      ))
}

impl CompileStatic {
    // makes self.enable_context_map
    // uses is_ctx_dynamic to determine whether the current `sc` is located in a
    // static or dynamic context
    // self.enable_context_map maps ids of static enables to a boolean indicating
    // wether the context is dynamic (true if dynamic, false if static)
    fn make_context_map_static(
        &mut self,
        sc: &ir::StaticControl,
        is_ctx_dynamic: bool,
    ) {
        match sc {
            ir::StaticControl::Enable(se) => {
                self.enable_context_map
                    .insert(get_guaranteed_enable_id(se), is_ctx_dynamic);
            }
            ir::StaticControl::Invoke(_) => {
                todo!("think abt how to handle static invoke")
            }
            ir::StaticControl::Empty(_) => (),
            ir::StaticControl::Seq(ir::StaticSeq { stmts, .. })
            | ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
                for stmt in stmts {
                    self.make_context_map_static(stmt, false);
                }
            }
            ir::StaticControl::If(ir::StaticIf {
                tbranch, fbranch, ..
            }) => {
                self.make_context_map_static(tbranch, false);
                self.make_context_map_static(fbranch, false);
            }
            ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
                self.make_context_map_static(body, false);
            }
        }
    }

    // make self.enable_context_map
    // self.enable_context_map maps ids of static enables to a boolean indicating
    // wether the context is dynamic (true if dynamic, false if static)
    fn make_context_map(&mut self, c: &ir::Control) {
        match c {
            ir::Control::Enable(_)
            | ir::Control::Invoke(_)
            | ir::Control::Empty(_) => (),
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                for stmt in stmts {
                    self.make_context_map(stmt);
                }
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                self.make_context_map(tbranch);
                self.make_context_map(fbranch);
            }
            ir::Control::While(ir::While { body, .. }) => {
                self.make_context_map(body);
            }
            ir::Control::Static(sc) => self.make_context_map_static(sc, true),
        }
    }

    fn make_early_reset_group(
        &mut self,
        sgroup_assigns: &mut Vec<ir::Assignment<ir::StaticTiming>>,
        sgroup_name: ir::Id,
        latency: u64,
        attributes: ir::Attributes,
        builder: &mut ir::Builder,
    ) -> ir::RRC<ir::Group> {
        let fsm_size =
            get_bit_width_from(latency + 1 /* represent 0..latency */);
        structure!( builder;
            let fsm = prim std_reg(fsm_size);
            //let ud = prim undef(1);
            let signal_on = constant(1,1);
            let adder = prim std_add(fsm_size);
            let const_one = constant(1, fsm_size);
            let first_state = constant(0, fsm_size);
            let penultimate_state = constant(latency-1, fsm_size);
            let last_state = constant(latency, fsm_size);
        );
        // create the dynamic group we will use to replace the static group
        let mut early_reset_name = sgroup_name.clone().to_string();
        early_reset_name.insert_str(0, "early_reset_");
        let g = builder.add_group(early_reset_name);
        // converting static assignments to dynamic assignments
        let mut assigns = sgroup_assigns
            .drain(..)
            .map(|assign| make_assign_dyn(assign, &g, &fsm, fsm_size, builder))
            .collect_vec();
        // assignments to increment the fsm
        let not_penultimate_state_guard: ir::Guard<ir::Nothing> =
            guard!(fsm["out"]).neq(guard!(penultimate_state["out"]));
        let penultimate_state_guard: ir::Guard<ir::Nothing> =
            guard!(fsm["out"]).eq(guard!(penultimate_state["out"]));
        let last_state_guard: ir::Guard<ir::Nothing> =
            guard!(fsm["out"]).eq(guard!(last_state["out"]));
        let fsm_incr_assigns = build_assignments!(
          builder;
          adder["left"] = ? fsm["out"];
          adder["right"] = ? const_one["out"];
          fsm["write_en"] = ? signal_on["out"];
          fsm["in"] = not_penultimate_state_guard ? adder["out"];
          fsm["in"] = penultimate_state_guard ? first_state["out"];
          // will never reach this guard since we are resetting when we get to
          // the penultimate state
          g["done"] = last_state_guard ? signal_on["out"];
        );
        assigns.extend(fsm_incr_assigns.to_vec());
        self.fsm_map.insert(g.borrow().name(), fsm.borrow().name());
        // adding the assignments to the new dynamic group and creating a
        // new (dynamic) enable
        g.borrow_mut().assignments = assigns;
        g.borrow_mut().attributes = attributes;
        g
    }

    fn build_wrapper_group(
        fsm_name: &ir::Id,
        group_name: &ir::Id,
        builder: &mut ir::Builder,
    ) -> ir::RRC<ir::Group> {
        // get the groups/cells necessary to build the wrapper group
        let early_reset_group = builder
            .component
            .get_groups()
            .find(*group_name)
            .unwrap_or_else(|| {
                panic!(
                    "called build_wrapper_group with {}, which is not a group",
                    group_name
                )
            });
        let early_reset_fsm =
            builder.component.find_cell(*fsm_name).unwrap_or_else(|| {
                panic!(
                    "called build_wrapper_group with {}, which is not an fsm",
                    fsm_name
                )
            });
        let fsm_width = early_reset_fsm
            .borrow()
            .ports()
            .iter()
            .find(|port| port.borrow().name == "in")
            .unwrap_or_else(|| panic!("called {} in build_wrapper_group as an fsm; should have `in` port", early_reset_fsm.borrow().name()))
            .borrow()
            .width;

        structure!( builder;
            let signal_reg = prim std_reg(1);
            let state_zero = constant(0, fsm_width);
            let signal_on = constant(1, 1);
            let signal_off = constant(0, 1);
        );
        // make guards
        // fsm.out == 0 ?
        let first_state: ir::Guard<ir::Nothing> =
            guard!(early_reset_fsm["out"]).eq(guard!(state_zero["out"]));
        // signal_reg.out ?
        let signal_reg_guard: ir::Guard<ir::Nothing> =
            guard!(signal_reg["out"]);
        // !signal_reg.out ?
        let not_signal_reg = signal_reg_guard.clone().not();
        // fsm.out == 0 & signal_reg.out ?
        let first_state_and_signal = first_state.clone().and(signal_reg_guard);
        // fsm.out == 0 & ! signal_reg.out ?
        let first_state_and_not_signal = first_state.and(not_signal_reg);
        // create the dynamic group we will use to replace the static group
        let mut wrapper_name = group_name.clone().to_string();
        wrapper_name.insert_str(0, "wrapper_");
        let g = builder.add_group(wrapper_name);
        let group_assigns = build_assignments!(
          builder;
          // early_reset_group[go] = 1'd1
          early_reset_group["go"] = ? signal_on["out"];
          // signal_reg.write_en = !signal_reg.out & fsm.out == 0 ? 1'd1
          signal_reg["write_en"] = first_state_and_not_signal ? signal_on["out"];
          // signal_reg.in= !signal_reg.out & fsm.out == 0 ? 1'd1
          signal_reg["in"] =  first_state_and_not_signal ? signal_on["out"];
          // group[done] = fsm.out == 0 & signal_reg.out ? 1'd1
          g["done"] = first_state_and_signal ? signal_on["out"];
        );
        // continuous assignments to reset signal_reg back to 0 when the wrapper is done
        let continuous_assigns = build_assignments!(
            builder;
            // signal_reg.write_en = signal_reg.out & fsm.out == 0 ? 1'd1
            signal_reg["write_en"] = first_state_and_signal ? signal_on["out"];
            // signal_reg.in= signal_reg.out & fsm.out == 0 ? 1'd0
            signal_reg["in"] =  first_state_and_signal ? signal_off["out"];
        );
        builder.add_continuous_assignments(continuous_assigns.to_vec());
        g.borrow_mut().assignments = group_assigns.to_vec();
        g.borrow_mut().attributes =
            early_reset_group.borrow().attributes.clone();
        g
    }
}

impl Visitor for CompileStatic {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // assign unique ids so we can use them in our map
        add_enable_ids(&mut comp.control.borrow_mut(), 0);
        // make the "context map" so we know which enables are in a static vs dynamic context
        self.make_context_map(&comp.control.borrow());
        let sgroups: Vec<ir::RRC<ir::StaticGroup>> =
            comp.get_static_groups_mut().drain().collect();
        let mut builder = ir::Builder::new(comp, sigs);
        // create "early reset" dynamic groups that still take the same number
        // of cycles, but never reach their done hole
        for sgroup in sgroups.iter() {
            let mut sgroup_ref = sgroup.borrow_mut();
            let sgroup_name = sgroup_ref.name();
            let sgroup_latency = sgroup_ref.get_latency();
            let sgroup_attributes = sgroup_ref.attributes.clone();
            let sgroup_assigns = &mut sgroup_ref.assignments;
            let g = self.make_early_reset_group(
                sgroup_assigns,
                sgroup_name,
                sgroup_latency,
                sgroup_attributes,
                &mut builder,
            );
            self.reset_early_map.insert(sgroup_name, g.borrow().name());
            self.group_rewrite.insert(
                ir::Canonical(sgroup_name, ir::Id::from("go")),
                g.borrow().find("go").unwrap_or_else(|| {
                    panic!("group {} has no go port", g.borrow().name())
                }),
            );
        }

        // rewrite static_group[go] to early_reset_group[go]
        comp.for_each_assignment(|assign| {
            assign.for_each_port(|port| {
                match self.group_rewrite.get(&port.borrow().canonical()) {
                    None => None,
                    Some(port_ref) => Some(Rc::clone(port_ref)),
                }
            })
        });

        comp.get_static_groups_mut().append(sgroups.into_iter());

        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::Static] node.
    fn start_static_control(
        &mut self,
        sc: &mut ir::StaticControl,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        match sc {
            ir::StaticControl::Enable(s) => {
                let sgroup = s.group.borrow_mut();
                let sgroup_name = sgroup.name();

                // get the "early reset group". If it doesn't exist then we make it.
                let early_reset_name =
                    self.reset_early_map.get(&sgroup_name).unwrap_or_else(|| {
                        panic!("group {} not in self.reset_early_map", sgroup_name)
                    });
                let early_reset_group = comp.find_group(*early_reset_name).unwrap();
                // create the builder/cells that we need to turn static group dynamic
                let mut builder = ir::Builder::new(comp, sigs);

                // pick the group (either early reset or wrapper) based on self.enable_context_map
                let group_choice =  match self.enable_context_map.get(&get_guaranteed_enable_id(s)) {
                    Some(true) => {
                        match self.wrapper_map.get(&early_reset_name){
                            None => {
                                let fsm_name = self.fsm_map.get(&early_reset_name).unwrap();
                                let wrapper = Self::build_wrapper_group(fsm_name, &early_reset_name, & mut builder);
                                self.wrapper_map.insert(*early_reset_name, wrapper.borrow().name());
                                wrapper
                            }
                            Some(name) => {
                                comp.find_group(*name).unwrap()
                            }
                        }
                    },
                    Some(false) => {
                        // in static context, so just have to use static group
                        early_reset_group
                    },
                    None => panic!("self.enable_context_map should have mapped every static enable")
                };

                let mut e = ir::Control::enable(group_choice);
                let attrs = std::mem::take(&mut s.attributes);
                *e.get_mut_attributes() = attrs;
                Ok(Action::Change(Box::new(e)))
            }
            _ => unreachable!("Non-Enable Static Control should have been compiled away. Run static-inliner to do this"),
        }
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // make sure static groups have no assignments, since
        // we should have already drained the assignments in static groups
        for g in comp.get_static_groups() {
            if !g.borrow().assignments.is_empty() {
                unreachable!("Should have converted all static groups to dynamic. {} still has assignments in it", g.borrow().name());
            }
        }
        // remove all static groups
        comp.get_static_groups_mut().retain(|_| false);
        Ok(Action::Continue)
    }
}
