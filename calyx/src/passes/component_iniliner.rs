use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use itertools::Itertools;

use crate::analysis;
use crate::errors::Error;
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

    /// Rewrite assignments using a [CellMap], [PortMap], and an optional new group.
    /// Attempts the following rewrites in order:
    /// 1. Using the [CellMap] to get the same port on a new [Cell].
    /// 2. Using the [PortMap] to a new [Port].
    /// 3. Using `new_group` to rewrite use of a group hole if the port is a hole.
    fn rewrite_assigns(
        assigns: &mut Vec<ir::Assignment>,
        port_rewrite: &ir::Rewriter,
        new_group: Option<&RRC<ir::Group>>,
    ) {
        assigns.iter_mut().for_each(|assign| {
            assign.for_each_port(|port| {
                port_rewrite.get(port).or_else(|| {
                    if let Some(grp) = new_group {
                        if port.borrow().is_hole() {
                            return Some(grp.borrow().get(&port.borrow().name));
                        }
                    }
                    None
                })
            });
        })
    }

    /// Inline a group definition from a component into the component associated
    /// with the `builder`.
    fn inline_group(
        builder: &mut ir::Builder,
        port_rewrite: &ir::Rewriter,
        gr: &RRC<ir::Group>,
    ) -> (ir::Id, RRC<ir::Group>) {
        let group = gr.borrow();
        let new_group = builder.add_group(group.clone_name());
        new_group.borrow_mut().attributes = group.attributes.clone();

        // Rewrite assignments
        let mut asgns = group.assignments.clone();
        Self::rewrite_assigns(&mut asgns, port_rewrite, Some(&new_group));
        new_group.borrow_mut().assignments = asgns;
        (group.clone_name(), new_group)
    }

    /// Inline a group definition from a component into the component associated
    /// with the `builder`.
    fn inline_comb_group(
        builder: &mut ir::Builder,
        port_rewrite: &ir::Rewriter,
        gr: &RRC<ir::CombGroup>,
    ) -> (ir::Id, RRC<ir::CombGroup>) {
        let group = gr.borrow();
        let new_group = builder.add_comb_group(group.clone_name());
        new_group.borrow_mut().attributes = group.attributes.clone();

        // Rewrite assignments
        let mut asgns = group.assignments.clone();
        Self::rewrite_assigns(&mut asgns, port_rewrite, None);
        new_group.borrow_mut().assignments = asgns;
        (group.clone_name(), new_group)
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
                let wire_ref =
                    builder.add_primitive(wire_name, "std_wire", &[port.width]);
                let wire = wire_ref.borrow();
                let pn = match port.direction {
                    ir::Direction::Input => "in",
                    ir::Direction::Output => "out",
                    ir::Direction::Inout => unreachable!(),
                };
                (port.canonical(), wire.get(pn))
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
        let interface_map = Self::inline_interface(builder, comp, name.clone());
        let rewrite = ir::Rewriter::new(&cell_map, &interface_map);

        // For each group, create a new group and rewrite all assignments within
        // it using the `rewrite_map`.
        let group_map: GroupMap = comp
            .groups
            .iter()
            .map(|gr| Self::inline_group(builder, &rewrite, gr))
            .collect();
        let comb_group_map: CombGroupMap = comp
            .comb_groups
            .iter()
            .map(|gr| Self::inline_comb_group(builder, &rewrite, gr))
            .collect();

        // Rewrite continuous assignments
        let mut cont_assigns = comp.continuous_assignments.clone();
        Self::rewrite_assigns(&mut cont_assigns, &rewrite, None);
        builder
            .component
            .continuous_assignments
            .extend(cont_assigns);

        // Generate a control program associated with this instance
        let mut con = ir::Control::clone(&comp.control.borrow());
        rewrite.rewrite_control(&mut con, &group_map, &comb_group_map);

        (
            con,
            interface_map
                .into_iter()
                .map(|((_, p), pr)| {
                    let port = pr.borrow();
                    let np = match port.name.id.as_str() {
                        "in" => "out",
                        "out" => "in",
                        _ => unreachable!(),
                    };
                    ((name.clone(), p), port.cell_parent().borrow().get(np))
                })
                .collect(),
        )
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
        // Calculate the control ports before the component is modified.
        let control_ports =
            analysis::ControlPorts::from(&*comp.control.borrow());

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

        // Track names of cells that were inlined.
        let mut inlined_cells = HashSet::new();
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
                inlined_cells.insert(cell.clone_name());
            }
        }

        // XXX: This is unneccessarily iterate over the newly inlined groups.
        // Rewrite all assignment in the component to use interface wires
        // from the inlined instances.
        builder.component.for_each_assignment(&|assign| {
            assign.for_each_port(|pr| {
                let port = &pr.borrow();
                interface_rewrites.get(&port.canonical()).cloned()
            });
        });

        // Use analysis to get all bindings for invokes and filter out bindings
        // for inlined cells.
        let invoke_bindings = control_ports
            .get_all_bindings()
            .into_iter()
            .filter(|(instance, _)| inlined_cells.contains(instance));

        // Ensure that all invokes use the same parameters and inline the parameter assignments.
        for (instance, mut bindings) in invoke_bindings {
            assert!(!bindings.is_empty(), "Instance binding cannot be empty");
            if bindings.len() > 1 {
                return Err(
                    Error::PassAssumption(
                        Self::name().to_string(),
                        format!(
                            "Instance `{}.{}` invoked with multiple parameters (currently unsupported)",
                            comp.name,
                            instance)));
            }
            let binding = bindings.pop().unwrap();
            let mut assigns = binding
                .into_iter()
                .map(|(name, param)| {
                    let port = Rc::clone(
                        &interface_rewrites[&(instance.clone(), name)],
                    );
                    let dir = port.borrow().direction.clone();
                    match dir {
                        ir::Direction::Input => builder.build_assignment(
                            port,
                            param,
                            ir::Guard::True,
                        ),
                        ir::Direction::Output => builder.build_assignment(
                            param,
                            port,
                            ir::Guard::True,
                        ),
                        ir::Direction::Inout => unreachable!(),
                    }
                })
                .collect_vec();
            builder
                .component
                .continuous_assignments
                .append(&mut assigns);
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
