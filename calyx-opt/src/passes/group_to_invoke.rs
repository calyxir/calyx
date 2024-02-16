use crate::analysis::AssignmentAnalysis;
use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir};
use calyx_ir::{GetAttributes, RRC};
use calyx_utils::CalyxResult;
use ir::Nothing;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Transform groups that are structurally invoking components into equivalent
/// [ir::Invoke] statements.
///
/// For a group to meet the requirements of this pass, it must
/// 1. Only write to one non-combinational component (all other writes must be
/// to combinational primitives)
/// 2. That component is *not* a ref cell, nor does it have the external attribute,
/// nor is it This Component
/// 3. Assign component.go = 1'd1
/// 4. Assign group[done] = component.done
pub struct GroupToInvoke {
    /// Primitives that have multiple @go-@done signals
    blacklist: HashSet<ir::Id>,
    /// Maps names of group to the invokes that will replace them
    group_invoke_map: HashMap<ir::Id, ir::Control>,
}

impl ConstructVisitor for GroupToInvoke {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        // Construct list of primitives that have multiple go-done signals
        let blacklist = ctx
            .lib
            .signatures()
            .filter(|p| p.find_all_with_attr(ir::NumAttr::Go).count() > 1)
            .map(|p| p.name)
            .collect();

        Ok(Self {
            blacklist,
            group_invoke_map: HashMap::new(),
        })
    }

    fn clear_data(&mut self) {
        self.group_invoke_map = HashMap::new();
    }
}

impl Named for GroupToInvoke {
    fn name() -> &'static str {
        "group2invoke"
    }

    fn description() -> &'static str {
        "covert groups that structurally invoke one component into invoke statements"
    }
}

/// Construct an [ir::Invoke] from an [ir::Group] that has been validated by this pass.
fn construct_invoke(
    assigns: &[ir::Assignment<Nothing>],
    comp: RRC<ir::Cell>,
    builder: &mut ir::Builder,
) -> ir::Control {
    // Check if port's parent is a combinational primitive
    let parent_is_comb = |port: &ir::Port| -> bool {
        if !port.is_hole() {
            if let ir::CellType::Primitive { is_comb, .. } =
                port.cell_parent().borrow().prototype
            {
                return is_comb;
            }
        }
        false
    };

    // Check if port's parent is equal to comp
    let parent_is_cell = |port: &ir::Port| -> bool {
        match &port.parent {
            ir::PortParent::Cell(cell_wref) => {
                Rc::ptr_eq(&cell_wref.upgrade(), &comp)
            }
            _ => false,
        }
    };

    let mut inputs = Vec::new();
    let mut comb_assigns = Vec::new();
    let mut wire_map: HashMap<ir::Id, ir::RRC<ir::Port>> = HashMap::new();

    for assign in assigns {
        // We know that all assignments in this group should write to either a)
        // a combinational component or b) comp or c) the group's done port-- we
        // should have checked for this condition before calling this function

        // If a combinational component's port is being used as a dest, add
        // it to comb_assigns
        if parent_is_comb(&assign.dst.borrow()) {
            comb_assigns.push(assign.clone());
        }
        // If the cell's port is being used as a dest, add the source to
        // inputs. we can ignore the cell.go assignment, since that is not
        // going to be part of the `invoke`.
        else if parent_is_cell(&assign.dst.borrow())
            && assign.dst
                != comp.borrow().get_unique_with_attr(ir::NumAttr::Go).unwrap()
        {
            let name = assign.dst.borrow().name;
            if assign.guard.is_true() {
                inputs.push((name, Rc::clone(&assign.src)));
            } else {
                // assign has a guard condition,so need a wire
                // We first check whether we have already built a wire
                // for this port or not.
                let wire_in = match wire_map.get(&assign.dst.borrow().name) {
                    Some(w) => {
                        // Already built a wire, so just need to return the
                        // wire's input port (which wire_map stores)
                        Rc::clone(w)
                    }
                    None => {
                        // Need to create a new wire
                        let width = assign.dst.borrow().width;
                        let wire = builder.add_primitive(
                            format!("{}_guarded_wire", name),
                            "std_wire",
                            &[width],
                        );
                        // Insert the wire's input port into wire_map
                        let wire_in = wire.borrow().get("in");
                        wire_map.insert(
                            assign.dst.borrow().name,
                            Rc::clone(&wire_in),
                        );
                        // add the wire's output port to the inputs of the
                        // invoke statement we are building
                        inputs.push((name, wire.borrow().get("out")));
                        // return wire_in
                        wire_in
                    }
                };
                // Use wire_in to add another assignment to combinational group
                let asmt = builder.build_assignment(
                    wire_in,
                    Rc::clone(&assign.src),
                    *assign.guard.clone(),
                );
                comb_assigns.push(asmt);
            }
        }
    }

    let comb_group = if comb_assigns.is_empty() {
        None
    } else {
        let comb_group_ref = builder.add_comb_group("comb_invoke");
        comb_group_ref
            .borrow_mut()
            .assignments
            .append(&mut comb_assigns);
        Some(comb_group_ref)
    };

    ir::Control::Invoke(ir::Invoke {
        comp,
        inputs,
        outputs: Vec::new(),
        comb_group,
        attributes: ir::Attributes::default(),
        ref_cells: Vec::new(),
    })
}

impl Visitor for GroupToInvoke {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let groups = comp.get_groups_mut().drain().collect_vec();
        let static_groups = comp.get_static_groups_mut().drain().collect_vec();
        let mut builder = ir::Builder::new(comp, sigs);
        for g in &groups {
            self.analyze_group(
                &mut builder,
                g.borrow().name(),
                &g.borrow().assignments,
                &g.borrow().get("done"),
            )
        }
        // Not transforming static groups rn
        /*for g in &static_groups {
            self.analyze_group(
                &mut builder,
                g.borrow().name(),
                &g.borrow().assignments,
                &g.borrow().get(ir::NumAttr::Done),
            )
        }*/

        comp.get_groups_mut().append(groups.into_iter());
        comp.get_static_groups_mut()
            .append(static_groups.into_iter());

        Ok(Action::Continue)
    }

    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        match self.group_invoke_map.get(&s.group.borrow().name()) {
            None => Ok(Action::Continue),
            Some(invoke) => {
                let mut inv = ir::Cloner::control(invoke);
                let attrs = std::mem::take(&mut s.attributes);
                *inv.get_mut_attributes() = attrs;
                Ok(Action::Change(Box::new(inv)))
            }
        }
    }
}

impl GroupToInvoke {
    // if g is able to be turned into invoke, then add to self.group_invoke_map
    fn analyze_group(
        &mut self,
        builder: &mut ir::Builder,
        group_name: ir::Id,
        assigns: &[ir::Assignment<Nothing>],
        group_done_port: &ir::RRC<ir::Port>,
    ) {
        let mut writes = assigns
            .iter()
            .analysis()
            .cell_writes()
            .filter(|cell| match cell.borrow().prototype {
                ir::CellType::Primitive { is_comb, .. } => !is_comb,
                _ => true,
            })
            .collect_vec();
        // Excluding writes to combinational components, should write to exactly
        // one cell
        if writes.len() != 1 {
            return;
        }

        // If component is ThisComponent, Reference, or External, don't turn into invoke
        let cr = writes.pop().unwrap();
        let cell = cr.borrow();
        match &cell.prototype {
            ir::CellType::Primitive { name, .. }
                if self.blacklist.contains(name) =>
            {
                return;
            }
            ir::CellType::ThisComponent => return,
            _ => {}
        }
        if cell.is_reference() || cell.attributes.has(ir::BoolAttr::External) {
            return;
        }

        // Component must define exactly one @go/@done interface
        let Ok(Some(go_port)) = cell.find_unique_with_attr(ir::NumAttr::Go)
        else {
            return;
        };
        let Ok(Some(done_port)) = cell.find_unique_with_attr(ir::NumAttr::Done)
        else {
            return;
        };

        let mut go_wr_cnt = 0;
        let mut done_wr_cnt = 0;

        'assigns: for assign in assigns {
            // @go port should have exactly one write and the src should be 1.
            if assign.dst == go_port {
                if go_wr_cnt > 0 {
                    log::info!(
                        "Cannot transform `{}` due to multiple writes to @go port",
                        group_name,
                    );
                    return;
                } else if !assign.guard.is_true() {
                    log::info!(
                            "Cannot transform `{}` due to guarded write to @go port: {}",
                            group_name,
                            ir::Printer::assignment_to_str(assign)
                        );
                    return;
                } else if assign.src.borrow().is_constant(1, 1) {
                    go_wr_cnt += 1;
                } else {
                    // if go port's guard is not true, src is not (1,1), then
                    // Continue
                    continue 'assigns;
                }
            }
            // @done port should have exactly one read and the dst should be
            // group's done signal.
            if assign.src == done_port {
                if done_wr_cnt > 0 {
                    log::info!(
                            "Cannot transform `{}` due to multiple writes to @done port",
                            group_name,
                        );
                    return;
                } else if !assign.guard.is_true() {
                    log::info!(
                            "Cannot transform `{}` due to guarded write to @done port: {}",
                            group_name,
                            ir::Printer::assignment_to_str(assign)
                        );
                    return;
                } else if assign.dst == *group_done_port {
                    done_wr_cnt += 1;
                } else {
                    // If done port's guard is not true and does not write to group's done
                    // then Continue
                    continue 'assigns;
                }
            }
        }
        drop(cell);

        if go_wr_cnt != 1 {
            log::info!(
                "Cannot transform `{}` because there are no writes to @go port",
                group_name
            );
            return;
        } else if done_wr_cnt != 1 {
            log::info!("Cannot transform `{}` because there are no writes to @done port", group_name);
            return;
        }

        self.group_invoke_map
            .insert(group_name, construct_invoke(assigns, cr, builder));
    }
}
