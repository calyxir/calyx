use std::collections::HashMap;
use std::rc::Rc;

use itertools::Itertools;

use crate::errors::{CalyxResult, Error};
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures, RRC,
};
use crate::ir::{CloneName, GetAttributes};
use crate::{analysis, guard, structure};

#[derive(Default)]
/// Transforms combinational groups, which have a constant done condition,
/// into proper groups by registering the values read from the ports of cells
/// used within the combinational group.
///
/// It also transforms *-with into semantically equivalent control programs that first enable a
/// group that calculates and registers the ports defined by the combinational group.
/// execute the respective cond group and then execute the control operator.
///
/// # Example
/// ```
/// group comb_cond<"static"=0> {
///     lt.right = 32'd10;
///     lt.left = 32'd1;
///     eq.right = r.out;
///     eq.left = x.out;
///     comb_cond[done] = 1'd1;
/// }
/// control {
///     invoke comp(left = lt.out, ..)(..) with comb_cond;
///     if lt.out with comb_cond {
///         ...
///     }
///     while eq.out with comb_cond {
///         ...
///     }
/// }
/// ```
/// into:
/// ```
/// group comb_cond<"static"=1> {
///     lt.right = 32'd10;
///     lt.left = 32'd1;
///     eq.right = r.out;
///     eq.left = x.out;
///     lt_reg.in = lt.out
///     lt_reg.write_en = 1'd1;
///     eq_reg.in = eq.out;
///     eq_reg.write_en = 1'd1;
///     comb_cond[done] = lt_reg.done & eq_reg.done ? 1'd1;
/// }
/// control {
///     seq {
///       comb_cond;
///       invoke comp(left = lt_reg.out, ..)(..);
///     }
///     seq {
///       comb_cond;
///       if lt_reg.out {
///           ...
///       }
///     }
///     seq {
///       comb_cond;
///       while eq_reg.out {
///           ...
///           comb_cond;
///       }
///     }
/// }
/// ```
pub struct RemoveCombGroups {
    // Mapping from (group_name, (cell_name, port_name)) -> (port, group).
    port_rewrite: HashMap<PortInGroup, (RRC<ir::Port>, RRC<ir::Group>)>,
}

/// Represents (group_name, (cell_name, port_name))
type PortInGroup = (ir::Id, (ir::Id, ir::Id));

impl Named for RemoveCombGroups {
    fn name() -> &'static str {
        "remove-comb-groups"
    }

    fn description() -> &'static str {
        "Transforms all groups with a constant done condition"
    }
}

impl Visitor for RemoveCombGroups {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut used_ports =
            analysis::ControlPorts::<false>::from(&*comp.control.borrow());

        // Early return if there are no combinational groups
        if comp.comb_groups.is_empty() {
            return Ok(Action::Stop);
        }

        let mut builder = ir::Builder::new(comp, sigs);

        // Groups generated by transforming combinational groups
        let groups = builder
            .component
            .comb_groups
            .drain()
            .map(|cg_ref| {
                let name = cg_ref.borrow().name().clone();
                // Register the ports read by the combinational group's usages.
                let used_ports = used_ports.remove(&name).ok_or_else(|| {
                    Error::malformed_structure(format!(
                        "values from combinational group `{}` never used",
                        name
                    ))
                })?;

                // Group generated to replace this comb group.
                let group_ref = builder.add_group(name.as_ref());
                let mut group = group_ref.borrow_mut();
                // Attach assignmens from comb group
                group.assignments =
                    cg_ref.borrow_mut().assignments.drain(..).collect();

                // Registers to save value for the group
                let mut save_regs = Vec::with_capacity(used_ports.len());
                for port in used_ports {
                    // Register to save port value
                    structure!(builder;
                        let comb_reg = prim std_reg(port.borrow().width);
                        let signal_on = constant(1, 1);
                    );
                    let write = builder.build_assignment(
                        comb_reg.borrow().get("in"),
                        Rc::clone(&port),
                        ir::Guard::True,
                    );
                    let en = builder.build_assignment(
                        comb_reg.borrow().get("write_en"),
                        signal_on.borrow().get("out"),
                        ir::Guard::True,
                    );
                    group.assignments.push(write);
                    group.assignments.push(en);

                    // Define mapping from this port to the register's output
                    // value.
                    self.port_rewrite.insert(
                        (name.clone(), port.borrow().canonical().clone()),
                        (
                            Rc::clone(&comb_reg.borrow().get("out")),
                            Rc::clone(&group_ref),
                        ),
                    );

                    save_regs.push(comb_reg);
                }

                structure!(builder;
                    let signal_on = constant(1, 1);
                );

                // Create a done condition
                let done_guard = save_regs
                    .drain(..)
                    .map(|reg| guard!(reg["done"]))
                    .fold(ir::Guard::True, ir::Guard::and);
                let done_assign = builder.build_assignment(
                    group.get("done"),
                    signal_on.borrow().get("out"),
                    done_guard,
                );
                group.assignments.push(done_assign);

                // Add a "static" attribute
                group.attributes.insert("static", 1);
                drop(group);

                Ok(group_ref)
            })
            .collect::<CalyxResult<Vec<_>>>()?;

        for group in groups {
            comp.groups.add(group)
        }

        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(c) = &s.comb_group {
            let mut new_group = None;
            // Calculate the new input arguments for the invoke statement.
            let new_inputs = s
                .inputs
                .drain(..)
                .map(|(arg, port)| {
                    let key = (c.clone_name(), port.borrow().canonical());
                    if let Some((new_port, gr)) = self.port_rewrite.get(&key) {
                        new_group = Some(gr);
                        (arg, Rc::clone(new_port))
                    } else {
                        // Don't rewrite if the port is not defined by the combinational group.
                        (arg, port)
                    }
                })
                .collect_vec();
            if new_group.is_none() {
                return Err(Error::malformed_control(format!(
                    "Ports from combinational group `{}` attached to invoke-with clause are not used.",
                    c.borrow().name()
                )));
            }
            // New invoke statement with rewritten inputs.
            let mut invoke = ir::Control::invoke(
                Rc::clone(&s.comp),
                new_inputs,
                s.outputs.drain(..).collect(),
            );
            if let Some(attrs) = invoke.get_mut_attributes() {
                *attrs = std::mem::take(&mut s.attributes);
            }
            // Seq to run the rewritten comb group first and then the invoke.
            let seq = ir::Control::seq(vec![
                ir::Control::enable(Rc::clone(new_group.unwrap())),
                invoke,
            ]);
            Ok(Action::Change(seq))
        } else {
            Ok(Action::Continue)
        }
    }

    fn finish_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if s.cond.is_none() {
            return Ok(Action::Continue);
        }

        // Construct a new `while` statement
        let key = (
            s.cond.as_ref().unwrap().borrow().name().clone(),
            s.port.borrow().canonical(),
        );
        let (port_ref, cond_ref) = self.port_rewrite.get(&key).unwrap();
        let cond_in_body = ir::Control::enable(Rc::clone(cond_ref));
        let body = std::mem::replace(s.body.as_mut(), ir::Control::empty());
        let new_body = ir::Control::seq(vec![body, cond_in_body]);
        let mut while_ =
            ir::Control::while_(Rc::clone(port_ref), None, Box::new(new_body));
        if let Some(attrs) = while_.get_mut_attributes() {
            *attrs = std::mem::take(&mut s.attributes);
        }
        let cond_before_body = ir::Control::enable(Rc::clone(cond_ref));
        Ok(Action::Change(ir::Control::seq(vec![
            cond_before_body,
            while_,
        ])))
    }

    /// Transforms a `if-with` into a `seq-if` which first runs the cond group
    /// and then the branch.
    fn finish_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if s.cond.is_none() {
            return Ok(Action::Continue);
        }

        // Construct a new `if` statement
        let key = (
            s.cond.as_ref().unwrap().borrow().name().clone(),
            s.port.borrow().canonical(),
        );
        let (port_ref, cond_ref) =
            self.port_rewrite.get(&key).unwrap_or_else(|| {
                panic!(
                    "{}: Port `{}.{}` in group `{}` doesn't have a rewrite",
                    Self::name(),
                    key.1 .0,
                    key.1 .1,
                    key.0
                )
            });
        let tbranch =
            std::mem::replace(s.tbranch.as_mut(), ir::Control::empty());
        let fbranch =
            std::mem::replace(s.fbranch.as_mut(), ir::Control::empty());
        let if_ = ir::Control::if_(
            Rc::clone(port_ref),
            None,
            Box::new(tbranch),
            Box::new(fbranch),
        );
        let cond = ir::Control::enable(Rc::clone(cond_ref));
        Ok(Action::Change(ir::Control::seq(vec![cond, if_])))
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if comp.attributes.get("static").is_some() {
            let msg =
                format!("Component {} has both a top-level \"static\" annotations and combinational groups which is not supported", comp.name);
            return Err(Error::pass_assumption(Self::name().to_string(), msg)
                .with_pos(&comp.attributes));
        }
        Ok(Action::Continue)
    }
}
