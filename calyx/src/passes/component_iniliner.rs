use std::collections::HashMap;

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
type PortMap = HashMap<(ir::Id, ir::Id), RRC<ir::Cell>>;

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

    /// Rewrite assignments using the given [CellMap] and [PortMap].
    fn rewrite_assigns(
        assigns: &mut Vec<ir::Assignment>,
        cell_map: &CellMap,
        interface_map: &PortMap,
    ) {
        for assign in assigns.iter_mut() {
            ir::Rewriter::rename_cell_use(cell_map, assign);
            Self::rewrite_interface_use(interface_map, assign)
        }
    }

    /// Inline a group definition from a component into the component associated
    /// with the `builder`.
    fn inline_group(
        builder: &mut ir::Builder,
        cell_map: &CellMap,
        interface_map: &PortMap,
        gr: &RRC<ir::Group>,
    ) -> (ir::Id, RRC<ir::Group>) {
        let group = gr.borrow();
        let new_group = builder.add_group(group.clone_name());
        new_group.borrow_mut().attributes = group.attributes.clone();

        // Rewrite assignments
        let mut asgns = group.assignments.clone();
        Self::rewrite_assigns(&mut asgns, cell_map, interface_map);
        new_group.borrow_mut().assignments = asgns;
        (group.clone_name(), new_group)
    }

    /// Inline a group definition from a component into the component associated
    /// with the `builder`.
    fn inline_comb_group(
        builder: &mut ir::Builder,
        cell_map: &CellMap,
        interface_map: &PortMap,
        gr: &RRC<ir::CombGroup>,
    ) -> (ir::Id, RRC<ir::CombGroup>) {
        let group = gr.borrow();
        let new_group = builder.add_comb_group(group.clone_name());
        new_group.borrow_mut().attributes = group.attributes.clone();

        // Rewrite assignments
        let mut asgns = group.assignments.clone();
        Self::rewrite_assigns(&mut asgns, cell_map, interface_map);
        new_group.borrow_mut().assignments = asgns;
        (group.clone_name(), new_group)
    }

    /// Rewrite a use of an interface port.
    fn rewrite_interface_use(port_map: &PortMap, assign: &mut ir::Assignment) {
        fn this_parent(port: &RRC<ir::Port>) -> bool {
            let parent = &port.borrow().parent;
            if let ir::PortParent::Cell(cell_wref) = parent {
                let cell_ref = cell_wref.upgrade();
                let cell = cell_ref.borrow();
                return matches!(cell.prototype, ir::CellType::ThisComponent);
            }
            false
        }

        if this_parent(&assign.src) {
            let idx = assign.src.borrow().canonical();
            assign.src = port_map[&idx].borrow().get("out");
        }
        if this_parent(&assign.dst) {
            let idx = assign.dst.borrow().canonical();
            assign.dst = port_map[&idx].borrow().get("in");
        }
        assign.guard.for_each(&|port| {
            if this_parent(&port) {
                let idx = port.borrow().canonical();
                Some(ir::Guard::port(port_map[&idx].borrow().get("out")))
            } else {
                None
            }
        });
    }

    /// Adds wires that can hold the values written to various output ports.
    fn inline_interface(
        builder: &mut ir::Builder,
        comp: &ir::Component,
        name: ir::Id,
    ) -> PortMap {
        // For each output port, generate a wire that will store its value
        comp.signature
            .borrow()
            .ports
            .iter()
            .map(|port_ref| {
                let port = port_ref.borrow();
                let wire_name = format!("{}_{}", name, port.name);
                let wire =
                    builder.add_primitive(wire_name, "std_wire", &[port.width]);
                (port.canonical(), wire)
            })
            .collect()
    }

    /// Inline component `comp` into the parent component attached to `builder`.
    /// Returns:
    /// 1. The control program associated with the component being inlined,
    ///    rewritten for the specific instance.
    /// 2. A [PortMap] to be used in the parent component to rewrite uses of
    ///    interface ports of the component being inlined.
    fn inline_component(
        builder: &mut ir::Builder,
        comp: &ir::Component,
        name: ir::Id,
    ) -> (ir::Control, PortMap) {
        // For each cell in the component, create a new cell in the parent
        // of the same type and build a rewrite map using it.
        let cell_map: CellMap = comp
            .cells
            .iter()
            .map(|cell_ref| Self::inline_cell(builder, cell_ref))
            .collect();

        // Rewrites to inline the interface.
        let interface_map = Self::inline_interface(builder, comp, name);

        // For each group, create a new group and rewrite all assignments within
        // it using the `rewrite_map`.
        let group_map: GroupMap = comp
            .groups
            .iter()
            .map(|gr| {
                Self::inline_group(builder, &cell_map, &interface_map, gr)
            })
            .collect();
        let comb_group_map: CombGroupMap = comp
            .comb_groups
            .iter()
            .map(|gr| {
                Self::inline_comb_group(builder, &cell_map, &interface_map, gr)
            })
            .collect();

        // Rewrite continuous assignments
        let mut cont_assigns = comp.continuous_assignments.clone();
        Self::rewrite_assigns(&mut cont_assigns, &cell_map, &interface_map);
        builder
            .component
            .continuous_assignments
            .extend(cont_assigns);

        // Generate a control program associated with this instance
        let mut con = ir::Control::clone(&comp.control.borrow());
        ir::Rewriter::rewrite_control(
            &mut con,
            &cell_map,
            &group_map,
            &comb_group_map,
        );

        (con, interface_map)
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

        // Rewrites for the interface ports of the component being used in the
        // parent.
        let mut interface_rewrites: PortMap = HashMap::new();

        let mut builder = ir::Builder::new(comp, sigs);
        for cell_ref in inline_cells {
            let cell = cell_ref.borrow();
            if cell.is_component() {
                let comp_name = cell.type_name().unwrap();
                let (control, rewrites) = Self::inline_component(
                    &mut builder,
                    comp_map[comp_name],
                    cell.clone_name(),
                );
                interface_rewrites.extend(&mut rewrites.into_iter());
                self.control_map.insert(cell.clone_name(), control);
            }
        }

        // Rewrite all assignment in the component to use interface wires
        // from the inlined instances.
        comp.for_each_assignment(&|assign| {
            assign.for_each_port(|pr| {
                let port = &pr.borrow();
                interface_rewrites.get(&port.canonical()).map(|cell| {
                    let pn = match port.direction {
                        ir::Direction::Input => "in",
                        ir::Direction::Output => "out",
                        ir::Direction::Inout => unreachable!(),
                    };
                    cell.borrow().get(pn)
                })
            });
            Self::rewrite_interface_use(&interface_rewrites, assign)
        });

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
