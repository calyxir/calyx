use std::{collections::HashMap, rc::Rc};

use itertools::Itertools;

use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, CloneName, LibrarySignatures, RRC};

/// Inlines all sub-components marked with the `@inline` attribute.
/// Cannot inline components when they:
///   1. Are primitives
///   2. Are invoked structurally
///   3. Invoked using `invoke`-`with` statements
///
/// For each component that needs to be inlined, we need to:
///   1. Inline all cells defined by that instance.
///   2. Inline all groups defined by that instance.
///   3. Inline the control program for every `invoke` statement referring to the
///      instance.
#[derive(Default)]
pub struct ComponentInliner {
    // Map from the name of an instance to its associated control program.
    control_map: HashMap<ir::Id, ir::Control>,
}

/// Map name of old cell to the new cell
type CellMap = HashMap<ir::Id, RRC<ir::Cell>>;
/// Map name of old group to new group
type GroupMap = HashMap<ir::Id, RRC<ir::Group>>;
/// Map name of old combination group to new combinational group
type CombGroupMap = HashMap<ir::Id, RRC<ir::CombGroup>>;
/// Map canonical name of old port to new port
type PortMap = HashMap<(ir::Id, ir::Id), RRC<ir::Port>>;

impl ComponentInliner {
    /// Inline a cell definition into the component associated with the `builder`.
    fn inline_cell(
        builder: &mut ir::Builder,
        cell_ref: &RRC<ir::Cell>,
    ) -> (ir::Id, RRC<ir::Cell>) {
        let cell = cell_ref.borrow();
        let cn = cell.clone_name();
        let new_cell = match &cell.prototype {
            ir::CellType::Primitive {
                name,
                param_binding,
                ..
            } => builder.add_primitive(
                cn.clone(),
                name,
                &param_binding.iter().map(|(_, v)| *v).collect_vec(),
            ),
            ir::CellType::Component { name } => builder.add_component(
                cn.clone(),
                name.clone(),
                cell.get_signature(),
            ),
            ir::CellType::Constant { val, width } => {
                builder.add_constant(*val, *width)
            }
            ir::CellType::ThisComponent => unreachable!(),
        };
        (cn, new_cell)
    }

    /// Inline a group definition from a component into the component associated
    /// with the `builder`.
    fn inline_group(
        builder: &mut ir::Builder,
        cell_map: &CellMap,
        gr: &RRC<ir::Group>,
    ) -> (ir::Id, RRC<ir::Group>) {
        let group = gr.borrow();
        let new_group = builder.add_group(group.clone_name());
        new_group.borrow_mut().attributes = group.attributes.clone();

        // Rewrite assignments
        let mut asgns = group.assignments.clone();
        ir::Builder::rename_cell_uses(cell_map, &mut asgns);
        new_group.borrow_mut().assignments = asgns;
        (group.clone_name(), new_group)
    }

    /// Inline a group definition from a component into the component associated
    /// with the `builder`.
    fn inline_comb_group(
        builder: &mut ir::Builder,
        cell_map: &CellMap,
        gr: &RRC<ir::CombGroup>,
    ) -> (ir::Id, RRC<ir::CombGroup>) {
        let group = gr.borrow();
        let new_group = builder.add_comb_group(group.clone_name());
        new_group.borrow_mut().attributes = group.attributes.clone();

        // Rewrite assignments
        let mut asgns = group.assignments.clone();
        ir::Builder::rename_cell_uses(cell_map, &mut asgns);
        new_group.borrow_mut().assignments = asgns;
        (group.clone_name(), new_group)
    }

    /// Given a control program, rewrite all uses of cells, groups, and comb groups using the given
    /// rewrite maps.
    fn rewrite_control(
        c: &mut ir::Control,
        cm: &CellMap,
        gm: &GroupMap,
        cgm: &CombGroupMap,
    ) {
        match c {
            ir::Control::Empty(_) => (),
            ir::Control::Enable(en) => {
                let g = &en.group.borrow().clone_name();
                let new_group = Rc::clone(&gm[g]);
                en.group = new_group;
            }
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => stmts
                .iter_mut()
                .for_each(|c| Self::rewrite_control(c, cm, gm, cgm)),
            ir::Control::If(ife) => {
                // Rewrite port use
                let new_port = {
                    let port = &ife.port.borrow();
                    let parent = port.cell_parent().borrow().clone_name();
                    let new_parent = &cm[&parent];
                    &new_parent.borrow().get(&port.name)
                };
                ife.port = Rc::clone(new_port);
                // Rewrite conditional comb group if defined
                if let Some(cg_ref) = &ife.cond {
                    let cg = cg_ref.borrow().clone_name();
                    let new_cg = Rc::clone(&cgm[&cg]);
                    ife.cond = Some(new_cg);
                }
                // rewrite branches
                Self::rewrite_control(&mut ife.tbranch, cm, gm, cgm);
                Self::rewrite_control(&mut ife.fbranch, cm, gm, cgm);
            }
            ir::Control::While(wh) => {
                // Rewrite port use
                let new_port = {
                    let port = &wh.port.borrow();
                    let parent = port.cell_parent().borrow().clone_name();
                    let new_parent = &cm[&parent];
                    &new_parent.borrow().get(&port.name)
                };
                wh.port = Rc::clone(new_port);
                // Rewrite conditional comb group if defined
                if let Some(cg_ref) = &wh.cond {
                    let cg = cg_ref.borrow().clone_name();
                    let new_cg = Rc::clone(&cgm[&cg]);
                    wh.cond = Some(new_cg);
                }
                // rewrite body
                Self::rewrite_control(&mut wh.body, cm, gm, cgm);
            }
            ir::Control::Invoke(inv) => {
                // Rewrite the name of the cell
                let name = inv.comp.borrow().clone_name();
                let new_cell = &cm[&name];
                inv.comp = Rc::clone(new_cell);
                // Rewrite the parameters
                let rewrite_port = |port_ref: &RRC<ir::Port>| -> RRC<ir::Port> {
                    let port = port_ref.borrow();
                    let name = port.cell_parent().borrow().clone_name();
                    let new_parent = &cm[&name];
                    new_parent.borrow().get(&port.name)
                };
                inv.inputs.iter_mut().for_each(|(_, port)| {
                    *port = rewrite_port(&*port);
                });
                inv.outputs.iter_mut().for_each(|(_, port)| {
                    *port = rewrite_port(&*port);
                });
                // Rewrite the combinational group
                if let Some(cg_ref) = &inv.comb_group {
                    let cg = cg_ref.borrow().clone_name();
                    let new_cg = Rc::clone(&cgm[&cg]);
                    inv.comb_group = Some(new_cg);
                }
            }
        }
    }

    /// Adds wires that can hold the values written to various output ports.
    fn inline_outputs(
        builder: &mut ir::Builder,
        comp: &ir::Component,
    ) -> HashMap<ir::Id, RRC<ir::Cell>> {
        // For each output port, generate a wire that will store its value
        comp.signature
            .borrow()
            .ports
            .iter()
            .map(|port_ref| {
                let port = port_ref.borrow();
                let wire_name = format!("{}_{}", comp.name, port.name);
                let wire =
                    builder.add_primitive(wire_name, "std_wire", &[port.width]);
                (port.name.clone(), wire)
            })
            .collect()
    }

    /// Inline component `comp` into the parent component attached to `builder`
    fn inline_component(
        builder: &mut ir::Builder,
        comp: &ir::Component,
    ) -> ir::Control {
        // For each cell in the component, create a new cell in the parent
        // of the same type and build a rewrite map using it.
        let cell_map: CellMap = comp
            .cells
            .iter()
            .map(|cell_ref| Self::inline_cell(builder, cell_ref))
            .collect();

        // For each group, create a new group and rewrite all assignments within
        // it using the `rewrite_map`.
        let group_map: GroupMap = comp
            .groups
            .iter()
            .map(|gr| Self::inline_group(builder, &cell_map, gr))
            .collect();
        let comb_group_map: CombGroupMap = comp
            .comb_groups
            .iter()
            .map(|gr| Self::inline_comb_group(builder, &cell_map, gr))
            .collect();

        // Generate a control program associated with this instance
        let mut con = ir::Control::clone(&comp.control.borrow());
        Self::rewrite_control(&mut con, &cell_map, &group_map, &comb_group_map);
        con
    }
}

impl Named for ComponentInliner {
    fn name() -> &'static str {
        "inline"
    }

    fn description() -> &'static str {
        "inline all component instances marked with @inline attribute"
    }
}

impl Visitor for ComponentInliner {
    // Inlining should proceed bottom-up
    fn require_postorder() -> bool {
        true
    }

    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        comps: &[ir::Component],
    ) -> VisResult {
        // Separate out cells that need to be inlined.
        let (inline_cells, cells): (Vec<_>, Vec<_>) =
            comp.cells.drain().partition(|cr| {
                let cell = cr.borrow();
                cell.get_attribute("inline").is_some()
            });
        comp.cells.append(cells.into_iter());

        // Mapping from component name to component definition
        let comp_map = comps
            .iter()
            .map(|comp| (comp.name.clone(), comp))
            .collect::<HashMap<_, _>>();

        let mut builder = ir::Builder::new(comp, sigs);
        for cell_ref in inline_cells {
            let cell = cell_ref.borrow();
            if cell.is_component() {
                let comp_name = cell.type_name().unwrap();
                let con =
                    Self::inline_component(&mut builder, comp_map[comp_name]);
                self.control_map.insert(cell.clone_name(), con);
            }
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
        // If the associated instance has been inlined, replace the invoke with
        // its control program.
        let cell = s.comp.borrow();
        if let Some(con) = self.control_map.get(cell.name()) {
            Ok(Action::Change(ir::Control::clone(con)))
        } else {
            Ok(Action::Continue)
        }
    }
}
