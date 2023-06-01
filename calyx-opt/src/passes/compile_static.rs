use super::math_utilities::get_bit_width_from;
use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use calyx_ir::{guard, structure, GetAttributes};
use calyx_utils::Error;
use ir::{build_assignments, Nothing, StaticTiming, RRC};
use itertools::Itertools;
use std::collections::HashMap;
use std::ops::Not;
use std::rc::Rc;

#[derive(Default)]
/// Compiles Static Islands
pub struct CompileStatic {
    /// maps original static group names to the corresponding group that has an FSM that reset early
    reset_early_map: HashMap<ir::Id, ir::Id>,
    /// maps group that has an FSM that resets early to its dynamic "wrapper" group name.
    wrapper_map: HashMap<ir::Id, ir::Id>,
    /// maps reset_early_group names to (fsm name, fsm_width)
    fsm_info_map: HashMap<ir::Id, (ir::Id, u64)>,
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
// E.g.: %[2:3] gets turned into fsm.out >= 2 & fsm.out < 3
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
// Mainly transforms the guards such that fsm.out >= 2 & fsm.out <= 3
fn make_assign_dyn(
    assign: ir::Assignment<StaticTiming>,
    fsm: &ir::RRC<ir::Cell>,
    fsm_size: u64,
    builder: &mut ir::Builder,
) -> ir::Assignment<Nothing> {
    ir::Assignment {
        src: assign.src,
        dst: assign.dst,
        attributes: assign.attributes,
        guard: make_guard_dyn(*assign.guard, fsm, fsm_size, builder),
    }
}

impl CompileStatic {
    // returns an "early reset" group based on the information given
    // in the arguments.
    // sgroup_assigns are the static assignments of the group (they need to be
    // changed to dynamic by instantiating an fsm, i.e., %[0,2] -> fsm.out < 2)
    // name of early reset group has prefix "early_reset_{sgroup_name}"
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
            // done hole will be undefined bc of early reset
            let ud = prim undef(1);
            let signal_on = constant(1,1);
            let adder = prim std_add(fsm_size);
            let const_one = constant(1, fsm_size);
            let first_state = constant(0, fsm_size);
            let penultimate_state = constant(latency-1, fsm_size);
        );
        // create the dynamic group we will use to replace the static group
        let mut early_reset_name = sgroup_name.to_string();
        early_reset_name.insert_str(0, "early_reset_");
        let g = builder.add_group(early_reset_name);
        // converting static assignments to dynamic assignments
        let mut assigns = sgroup_assigns
            .drain(..)
            .map(|assign| make_assign_dyn(assign, &fsm, fsm_size, builder))
            .collect_vec();
        // assignments to increment the fsm
        let not_penultimate_state_guard: ir::Guard<ir::Nothing> =
            guard!(fsm["out"]).neq(guard!(penultimate_state["out"]));
        let penultimate_state_guard: ir::Guard<ir::Nothing> =
            guard!(fsm["out"]).eq(guard!(penultimate_state["out"]));
        let fsm_incr_assigns = build_assignments!(
          builder;
          // increments the fsm
          adder["left"] = ? fsm["out"];
          adder["right"] = ? const_one["out"];
          fsm["write_en"] = ? signal_on["out"];
          fsm["in"] = not_penultimate_state_guard ? adder["out"];
           // resets the fsm early
          fsm["in"] = penultimate_state_guard ? first_state["out"];
          // will never reach this guard since we are resetting when we get to
          // the penultimate state
          g["done"] = ? ud["out"];
        );
        assigns.extend(fsm_incr_assigns.to_vec());
        // maps the "early reset" group name to the "fsm name" that it borrows.
        // this is helpful when we build the "wrapper group"
        self.fsm_info_map
            .insert(g.borrow().name(), (fsm.borrow().name(), fsm_size));
        // adding the assignments to the new dynamic group and creating a
        // new (dynamic) enable
        g.borrow_mut().assignments = assigns;
        g.borrow_mut().attributes = attributes;
        g
    }

    fn build_wrapper_group(
        fsm_name: &ir::Id,
        fsm_width: u64,
        group_name: &ir::Id,
        builder: &mut ir::Builder,
    ) -> ir::RRC<ir::Group> {
        // get the groups/fsm necessary to build the wrapper group
        let early_reset_group = builder
            .component
            .get_groups()
            .find(*group_name)
            .unwrap_or_else(|| {
                unreachable!(
                    "called build_wrapper_group with {}, which is not a group",
                    group_name
                )
            });
        let early_reset_fsm =
            builder.component.find_cell(*fsm_name).unwrap_or_else(|| {
                unreachable!(
                    "called build_wrapper_group with {}, which is not an fsm",
                    fsm_name
                )
            });

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
        // create the wrapper group for early_reset_group
        let mut wrapper_name = group_name.clone().to_string();
        wrapper_name.insert_str(0, "wrapper_");
        let g = builder.add_group(wrapper_name);
        let group_assigns = build_assignments!(
          builder;
          // early_reset_group[go] = 1'd1
          early_reset_group["go"] = ? signal_on["out"];
          // when fsm == 0, and !signal_reg, then set signal_reg to high
          signal_reg["write_en"] = first_state_and_not_signal ? signal_on["out"];
          signal_reg["in"] =  first_state_and_not_signal ? signal_on["out"];
          // group[done] = fsm.out == 0 & signal_reg.out ? 1'd1
          g["done"] = first_state_and_signal ? signal_on["out"];
        );
        // continuous assignments to reset signal_reg back to 0 when the wrapper is done
        let continuous_assigns = build_assignments!(
            builder;
            // when (fsm == 0 & signal_reg is high), which is the done condition of the wrapper,
            // reset the signal_reg back to low
            signal_reg["write_en"] = first_state_and_signal ? signal_on["out"];
            signal_reg["in"] =  first_state_and_signal ? signal_off["out"];
        );
        builder.add_continuous_assignments(continuous_assigns.to_vec());
        g.borrow_mut().assignments = group_assigns.to_vec();
        g.borrow_mut().attributes =
            early_reset_group.borrow().attributes.clone();
        g
    }

    fn get_reset_group_name(&self, sc: &mut ir::StaticControl) -> &ir::Id {
        // assume that there are only static enables left.
        // if there are any other type of static control, then error out.
        let ir::StaticControl::Enable(s) = sc else {
            unreachable!("Non-Enable Static Control should have been compiled away. Run {} to do this", crate::passes::StaticInliner::name());
        };

        let sgroup = s.group.borrow_mut();
        let sgroup_name = sgroup.name();
        // get the "early reset group". It should exist, since we made an
        // early_reset group for every static group in the component
        let early_reset_name =
            self.reset_early_map.get(&sgroup_name).unwrap_or_else(|| {
                unreachable!(
                    "group {} not in self.reset_early_map",
                    sgroup_name
                )
            });

        early_reset_name
    }

    /// compile `while` whose body is `static` control such that at the end of each
    /// iteration, the checking of condition does not incur an extra cycle of
    /// latency.
    /// We do this by wrapping the early reset group of the body with
    /// another wrapper group, which sets the go signal of the early reset group
    /// high, and is done when at the 0th cycle of each iteration, the condtion
    /// port is done.
    /// Note: this only works if the port for the while condition is `@stable`.
    fn build_wrapper_group_while(
        &self,
        fsm_name: &ir::Id,
        fsm_width: u64,
        group_name: &ir::Id,
        port: RRC<ir::Port>,
        builder: &mut ir::Builder,
    ) -> RRC<ir::Group> {
        let reset_early_group = builder
            .component
            .find_group(*group_name)
            .unwrap_or_else(|| {
                unreachable!(
                    "called build_wrapper_group with {}, which is not a group",
                    group_name
                )
            });
        let early_reset_fsm =
            builder.component.find_cell(*fsm_name).unwrap_or_else(|| {
                unreachable!(
                    "called build_wrapper_group with {}, which is not an fsm",
                    fsm_name
                )
            });

        let wrapper_group =
            builder.add_group(format!("while_wrapper_{}", group_name));

        structure!(
            builder;
            let one = constant(1, 1);
            let time_0 = constant(0, fsm_width);
        );

        let port_parent = port.borrow().cell_parent();
        let port_name = port.borrow().name;
        let done_guard = (!guard!(port_parent[port_name]))
            & guard!(early_reset_fsm["out"]).eq(guard!(time_0["out"]));

        let assignments = build_assignments!(
            builder;
            // reset_early_group[go] = 1'd1;
            // wrapper_group[done] = !port ? 1'd1;
            reset_early_group["go"] = ? one["out"];
            wrapper_group["done"] = done_guard ? one["out"];
        );

        wrapper_group.borrow_mut().assignments.extend(assignments);
        wrapper_group
    }
}

impl Visitor for CompileStatic {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let sgroups: Vec<ir::RRC<ir::StaticGroup>> =
            comp.get_static_groups_mut().drain().collect();
        let mut builder = ir::Builder::new(comp, sigs);
        // create "early reset" dynamic groups that never reach set their done hole
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
            // map the static group name -> early reset group name
            // helpful for rewriting control
            self.reset_early_map.insert(sgroup_name, g.borrow().name());
            // group_rewrite_map helps write static_group[go] to early_reset_group[go]
            // technically could do this w/ early_reset_map but is easier w/
            // group_rewrite, which is explicitly of type `PortRewriterMap`
            self.group_rewrite.insert(
                ir::Canonical(sgroup_name, ir::Id::from("go")),
                g.borrow().find("go").unwrap_or_else(|| {
                    unreachable!("group {} has no go port", g.borrow().name())
                }),
            );
        }

        // rewrite static_group[go] to early_reset_group[go]
        // don't have to worrry about writing static_group[done] b/c static
        // groups don't have done holes.
        comp.for_each_assignment(|assign| {
            assign.for_each_port(|port| {
                self.group_rewrite
                    .get(&port.borrow().canonical())
                    .map(Rc::clone)
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
        // assume that there are only static enables left.
        // if there are any other type of static control, then error out.
        let ir::StaticControl::Enable(s) = sc else {
            return Err(Error::malformed_control(format!("Non-Enable Static Control should have been compiled away. Run {} to do this", crate::passes::StaticInliner::name())));
        };

        let sgroup = s.group.borrow_mut();
        let sgroup_name = sgroup.name();
        // get the "early reset group". It should exist, since we made an
        // early_reset group for every static group in the component
        let early_reset_name =
            self.reset_early_map.get(&sgroup_name).unwrap_or_else(|| {
                unreachable!(
                    "group {} early reset wrapper has not been created",
                    sgroup_name
                )
            });
        // check if we've already built the wrapper group for early_reset_group
        // if so, we can just use that, otherwise, we must build the wrapper group
        let group_choice = match self.wrapper_map.get(early_reset_name) {
            None => {
                // create the builder/cells that we need to create wrapper group
                let mut builder = ir::Builder::new(comp, sigs);
                let (fsm_name, fsm_width )= self.fsm_info_map.get(early_reset_name).unwrap_or_else(|| unreachable!("group {} has no correspondoing fsm in self.fsm_map", early_reset_name));
                let wrapper = Self::build_wrapper_group(
                    fsm_name,
                    *fsm_width,
                    early_reset_name,
                    &mut builder,
                );
                self.wrapper_map
                    .insert(*early_reset_name, wrapper.borrow().name());
                wrapper
            }
            Some(name) => comp.find_group(*name).unwrap(),
        };

        let mut e = ir::Control::enable(group_choice);
        let attrs = std::mem::take(&mut s.attributes);
        *e.get_mut_attributes() = attrs;
        Ok(Action::Change(Box::new(e)))
    }

    /// if while body is static, then we want to make sure that the while
    /// body does not take the extra cycle incurred by the done condition
    /// So we replace the while loop with `enable` of a wrapper group
    /// that sets the go signal of the static group in the while loop body high
    /// (all static control should be compiled into static groups by
    /// `static_inliner` now). The done signal of the wrapper group should be
    /// the condition that the fsm of the while body is %0 and the port signal
    /// is 1'd0.
    /// For example, we replace
    /// ```
    /// wires {
    /// static group A<1> {
    ///     ...
    ///   }
    ///    ...
    /// }

    /// control {
    ///   while l.out {
    ///     A;
    ///   }
    /// }
    /// ```
    /// with
    /// ```
    /// wires {
    ///  group early_reset_A {
    ///     ...
    ///        }
    ///
    /// group while_wrapper_early_reset_A {
    ///       early_reset_A[go] = 1'd1;
    ///       while_wrapper_early_reset_A[done] = !l.out & fsm.out == 1'd0 ? 1'd1;
    ///     }
    ///   }
    ///   control {
    ///     while_wrapper_early_reset_A;
    ///   }
    /// ```
    fn start_while(
        &mut self,
        s: &mut ir::While,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if s.cond.is_none() {
            if let ir::Control::Static(sc) = &mut *(s.body) {
                let mut builder = ir::Builder::new(comp, sigs);
                let reset_group_name = self.get_reset_group_name(sc);

                // get fsm for reset_group
                let (fsm, fsm_width) = self.fsm_info_map.get(reset_group_name).unwrap_or_else(|| unreachable!("group {} has no correspondoing fsm in self.fsm_map", reset_group_name));
                let wrapper_group = self.build_wrapper_group_while(
                    fsm,
                    *fsm_width,
                    reset_group_name,
                    Rc::clone(&s.port),
                    &mut builder,
                );
                let c = ir::Control::enable(wrapper_group);
                return Ok(Action::change(c));
            }
        }

        Ok(Action::Continue)
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
