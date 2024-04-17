use crate::analysis;
use crate::traversal::{
    Action, ConstructVisitor, Named, Order, ParseVal, PassOpt, VisResult,
    Visitor,
};
use calyx_ir::{self as ir, rewriter, GetAttributes, LibrarySignatures, RRC};
use calyx_utils::Error;
use ir::Nothing;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

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
pub struct ComponentInliner {
    /// Force inlining of all components. Parsed from the command line.
    always_inline: bool,
    /// Generate new_fsms for the componnent we generate. Helpful if you don't
    /// want the fsms to get too many states
    new_fsms: bool,
    /// Map from the name of an instance to its associated control program.
    control_map: HashMap<ir::Id, ir::Control>,
    /// Mapping for ports on cells that have been inlined.
    interface_rewrites: rewriter::PortRewriteMap,
    /// Cells that have been inlined. We retain these so that references within
    /// the control program of the parent are valid.
    inlined_cells: Vec<RRC<ir::Cell>>,
}

impl Named for ComponentInliner {
    fn name() -> &'static str {
        "inline"
    }

    fn description() -> &'static str {
        "inline all component instances marked with @inline attribute"
    }

    fn opts() -> Vec<PassOpt> {
        vec![
            PassOpt::new(
                "always",
                "Attempt to inline all components into the main component",
                ParseVal::Bool(false),
                PassOpt::parse_bool,
            ),
            PassOpt::new(
                "new-fsms",
                "Instantiate new FSM for each inlined component",
                ParseVal::Bool(false),
                PassOpt::parse_bool,
            ),
        ]
    }
}

impl ComponentInliner {
    /// Equivalent to a default method but not automatically derived because
    /// it conflicts with the autogeneration of `ConstructVisitor`.
    fn new(always_inline: bool, new_fsms: bool) -> Self {
        ComponentInliner {
            always_inline,
            new_fsms,
            control_map: HashMap::default(),
            interface_rewrites: HashMap::default(),
            inlined_cells: Vec::default(),
        }
    }
}

impl ConstructVisitor for ComponentInliner {
    fn from(ctx: &ir::Context) -> calyx_utils::CalyxResult<Self>
    where
        Self: Sized,
    {
        let opts = Self::get_opts(ctx);
        Ok(ComponentInliner::new(
            opts[&"always"].bool(),
            opts[&"new-fsms"].bool(),
        ))
    }

    fn clear_data(&mut self) {
        *self = ComponentInliner::new(self.always_inline, self.new_fsms);
    }
}

impl ComponentInliner {
    /// Inline a cell definition into the component associated with the `builder`.
    fn inline_cell(
        builder: &mut ir::Builder,
        cell_ref: &RRC<ir::Cell>,
    ) -> (ir::Id, RRC<ir::Cell>) {
        let cell = cell_ref.borrow();
        let cn = cell.name();
        let new_cell = match &cell.prototype {
            ir::CellType::Primitive {
                name,
                param_binding,
                ..
            } => builder.add_primitive(
                cn,
                *name,
                &param_binding.iter().map(|(_, v)| *v).collect_vec(),
            ),
            ir::CellType::Component { name } => {
                builder.add_component(cn, *name, cell.get_signature())
            }
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
        assigns: &mut [ir::Assignment<Nothing>],
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

    /// Rewrite assignments using a [CellMap], [PortMap], and an optional new group.
    /// Attempts the following rewrites in order:
    /// 1. Using the [CellMap] to get the same port on a new [Cell].
    /// 2. Using the [PortMap] to a new [Port].
    /// 3. Using `new_group` to rewrite use of a group hole if the port is a hole.
    fn rewrite_assigns_static(
        assigns: &mut [ir::Assignment<ir::StaticTiming>],
        port_rewrite: &ir::Rewriter,
        new_group: Option<&RRC<ir::StaticGroup>>,
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

    /// Rewrites vec based on self.interface_rewrites Used as a helper function
    /// for rewrite_invoke_ports
    fn rewrite_vec(&self, v: &mut [(ir::Id, RRC<ir::Port>)]) {
        v.iter_mut().for_each(|(_, port)| {
            let key = port.borrow().canonical();
            if let Some(rewrite) = self.interface_rewrites.get(&key) {
                *port = Rc::clone(rewrite);
            }
        })
    }

    /// Rewrites the input/output ports of the invoke based on self.interface_rewrites
    fn rewrite_invoke_ports(&self, invoke: &mut ir::Invoke) {
        self.rewrite_vec(&mut invoke.inputs);
        self.rewrite_vec(&mut invoke.outputs);
    }

    /// Inline a group definition from a component into the component associated
    /// with the `builder`.
    fn inline_group(
        builder: &mut ir::Builder,
        port_rewrite: &ir::Rewriter,
        gr: &RRC<ir::Group>,
    ) -> (ir::Id, RRC<ir::Group>) {
        let group = gr.borrow();
        let new_group = builder.add_group(group.name());
        new_group.borrow_mut().attributes = group.attributes.clone();

        // Rewrite assignments
        let mut asgns = group.assignments.clone();
        Self::rewrite_assigns(&mut asgns, port_rewrite, Some(&new_group));
        new_group.borrow_mut().assignments = asgns;
        (group.name(), new_group)
    }

    /// Inline a group definition from a component into the component associated
    /// with the `builder`.
    fn inline_static_group(
        builder: &mut ir::Builder,
        port_rewrite: &ir::Rewriter,
        gr: &RRC<ir::StaticGroup>,
    ) -> (ir::Id, RRC<ir::StaticGroup>) {
        let group = gr.borrow();
        let new_group =
            builder.add_static_group(group.name(), group.get_latency());
        new_group.borrow_mut().attributes = group.attributes.clone();

        // Rewrite assignments
        let mut asgns = group.assignments.clone();
        Self::rewrite_assigns_static(
            &mut asgns,
            port_rewrite,
            Some(&new_group),
        );
        new_group.borrow_mut().assignments = asgns;
        (group.name(), new_group)
    }

    /// Inline a group definition from a component into the component associated
    /// with the `builder`.
    fn inline_comb_group(
        builder: &mut ir::Builder,
        port_rewrite: &ir::Rewriter,
        gr: &RRC<ir::CombGroup>,
    ) -> (ir::Id, RRC<ir::CombGroup>) {
        let group = gr.borrow();
        let new_group = builder.add_comb_group(group.name());
        new_group.borrow_mut().attributes = group.attributes.clone();

        // Rewrite assignments
        let mut asgns = group.assignments.clone();
        Self::rewrite_assigns(&mut asgns, port_rewrite, None);
        new_group.borrow_mut().assignments = asgns;
        (group.name(), new_group)
    }

    /// Adds wires that can hold the values written to various output ports.
    fn inline_interface(
        builder: &mut ir::Builder,
        comp: &ir::Component,
        name: ir::Id,
    ) -> rewriter::PortRewriteMap {
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
    /// 2. A [PortMap] (in form of an iterator) to be used in the parent component to rewrite uses
    ///    of interface ports of the component being inlined.
    fn inline_component(
        builder: &mut ir::Builder,
        mut cell_map: rewriter::RewriteMap<ir::Cell>,
        comp: &ir::Component,
        name: ir::Id,
    ) -> (
        ir::Control,
        impl Iterator<Item = (ir::Canonical, RRC<ir::Port>)>,
    ) {
        // For each cell in the component, create a new cell in the parent
        // of the same type and build a rewrite map using it.
        cell_map.extend(comp.cells.iter().filter_map(|cell_ref| {
            if !cell_ref.borrow().is_reference() {
                Some(Self::inline_cell(builder, cell_ref))
            } else {
                None
            }
        }));

        // Rewrites to inline the interface.
        let interface_map = Self::inline_interface(builder, comp, name);
        let mut rewrite = ir::Rewriter {
            cell_map,
            port_map: interface_map,
            ..Default::default()
        };

        // For each group, create a new group and rewrite all assignments within
        // it using the `rewrite_map`.
        rewrite.group_map = comp
            .get_groups()
            .iter()
            .map(|gr| Self::inline_group(builder, &rewrite, gr))
            .collect();
        rewrite.static_group_map = comp
            .get_static_groups()
            .iter()
            .map(|gr| Self::inline_static_group(builder, &rewrite, gr))
            .collect();
        rewrite.comb_group_map = comp
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
        let mut con = ir::Cloner::control(&comp.control.borrow());
        rewrite.rewrite_control(&mut con);

        // Generate interface map for use in the parent cell.
        // Return as an iterator because it's immediately merged into the global rewrite map.
        let rev_interface_map =
            rewrite.port_map.into_iter().map(move |(cp, pr)| {
                let ir::Canonical { port: p, .. } = cp;
                let port = pr.borrow();
                let np = match port.name.id.as_str() {
                    "in" => "out",
                    "out" => "in",
                    _ => unreachable!(),
                };
                (
                    ir::Canonical::new(name, p),
                    port.cell_parent().borrow().get(np),
                )
            });

        (con, rev_interface_map)
    }
}

impl Visitor for ComponentInliner {
    // Inlining should proceed bottom-up
    fn iteration_order() -> Order {
        Order::Post
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
                // If forced inlining is enabled, attempt to inline every
                // component.
                if self.always_inline {
                    cell.is_component()
                } else {
                    cell.get_attribute(ir::BoolAttr::Inline).is_some()
                }
            });
        comp.cells.append(cells.into_iter());

        // Early exit if there is nothing to inline
        if inline_cells.is_empty() {
            return Ok(Action::Stop);
        }

        // Use analysis to get all bindings for invokes and filter out bindings
        // for inlined cells.
        let invoke_bindings: HashMap<ir::Id, _> =
            analysis::ControlPorts::<true>::from(&*comp.control.borrow())
                .get_all_bindings()
                .into_iter()
                .filter(|(instance, _)| {
                    inline_cells.iter().any(|c| c.borrow().name() == instance)
                })
                .collect();

        // If any invoke has more than one binding, error out:
        for (instance, bindings) in &invoke_bindings {
            if bindings.len() > 1 {
                let bindings_str = bindings
                    .iter()
                    .map(|(cells, ports)| {
                        format!(
                            "[{}]({})",
                            cells
                                .iter()
                                .map(|(c, cell)| format!(
                                    "{c}={}",
                                    cell.borrow().name()
                                ))
                                .join(", "),
                            ports
                                .iter()
                                .map(|(p, port)| format!(
                                    "{p}={}",
                                    port.borrow().canonical()
                                ))
                                .join(", ")
                        )
                    })
                    .join("\n");
                return Err(
                    Error::pass_assumption(
                        Self::name(),
                        format!(
                            "Instance `{}.{instance}` invoked with multiple parameters (currently unsupported):\n{bindings_str}",
                            comp.name,
                        )));
            }
        }

        // Mapping from component name to component definition
        let comp_map = comps
            .iter()
            .map(|comp| (&comp.name, comp))
            .collect::<HashMap<_, _>>();

        // Rewrites for the interface ports of inlined cells.
        let mut interface_rewrites: rewriter::PortRewriteMap = HashMap::new();
        // Track names of cells that were inlined.
        let mut inlined_cells = HashSet::new();
        let mut builder = ir::Builder::new(comp, sigs);
        for cell_ref in &inline_cells {
            let cell = cell_ref.borrow();
            // Error if the cell is not a component
            if !cell.is_component() {
                let msg = format!(
                    "Cannot inline `{}`. It is a instance of primitive: `{}`",
                    cell.name(),
                    cell.type_name()
                        .unwrap_or_else(|| ir::Id::from("constant"))
                );

                return Err(Error::pass_assumption(Self::name(), msg)
                    .with_pos(&cell.attributes));
            }

            let comp_name = cell.type_name().unwrap();
            let cell_map =
                if let Some(binding) = &invoke_bindings.get(&cell.name()) {
                    let (cell_binds, _) = &binding[0];
                    cell_binds.iter().map(|(k, v)| (*k, v.clone())).collect()
                } else {
                    log::info!(
                        "no binding for `{}` which means instance is unused",
                        cell.name()
                    );
                    HashMap::new()
                };
            let (control, rewrites) = Self::inline_component(
                &mut builder,
                cell_map,
                comp_map[&comp_name],
                cell.name(),
            );
            interface_rewrites.extend(rewrites);
            self.control_map.insert(cell.name(), control);
            inlined_cells.insert(cell.name());
        }

        // XXX: This is unneccessarily iterate over the newly inlined groups.
        // Rewrite all assignment in the component to use interface wires
        // from the inlined instances and check if the `go` or `done` ports
        // on any of the instances was used for structural invokes.
        builder.component.for_each_assignment(|assign| {
            assign.for_each_port(|pr| {
                let port = &pr.borrow();
                let np = interface_rewrites.get(&port.canonical());
                if np.is_some() && (port.name == "go" || port.name == "done") {
                    panic!(
                        "Cannot inline instance. It is structurally structurally invoked: `{}`",
                        port.cell_parent().borrow().name(),
                    );
                }
                np.cloned()
            });
        });

        builder.component.for_each_static_assignment(|assign| {
            assign.for_each_port(|pr| {
                let port = &pr.borrow();
                let np = interface_rewrites.get(&port.canonical());
                if np.is_some() && (port.name == "go" || port.name == "done") {
                    panic!(
                        "Cannot inline instance. It is structurally structurally invoked: `{}`",
                        port.cell_parent().borrow().name(),
                    );
                }
                np.cloned()
            });
        });

        // Ensure that all invokes use the same parameters and inline the parameter assignments.
        for (instance, mut bindings) in invoke_bindings {
            let Some((_, binding)) = bindings.pop() else {
                unreachable!("Instance binding is empty");
            };
            let mut assigns = binding
                .into_iter()
                .filter(|(_, pr)| {
                    let port = pr.borrow();
                    // Skip clk and reset ports
                    !port.attributes.has(ir::BoolAttr::Clk)
                        && !port.attributes.has(ir::BoolAttr::Reset)
                })
                .map(|(name, param)| {
                    let port = Rc::clone(
                        &interface_rewrites
                            [&ir::Canonical::new(instance, name)],
                    );
                    // The parameter can refer to port on a cell that has been
                    // inlined.
                    let name = param.borrow().canonical();
                    let new_param = interface_rewrites
                        .get(&name)
                        .map(Rc::clone)
                        .unwrap_or(param);
                    let dir = port.borrow().direction.clone();
                    match dir {
                        ir::Direction::Input => builder.build_assignment(
                            port,
                            new_param,
                            ir::Guard::True,
                        ),
                        ir::Direction::Output => builder.build_assignment(
                            new_param,
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

        self.interface_rewrites = interface_rewrites;
        // Save inlined cells so that references within the parent control
        // program remain valid.
        self.inlined_cells = inline_cells;

        Ok(Action::Continue)
    }

    fn start_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let name = &s.port.borrow().canonical();
        if let Some(new_port) = self.interface_rewrites.get(name) {
            s.port = Rc::clone(new_port);
        }
        Ok(Action::Continue)
    }

    fn start_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let name = &s.port.borrow().canonical();
        if let Some(new_port) = self.interface_rewrites.get(name) {
            s.port = Rc::clone(new_port);
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
        // Regardless of whether the associated instance has been inlined,
        // we still may need to rewrite the input/output ports
        self.rewrite_invoke_ports(s);

        // If the associated instance has been inlined, replace the invoke with
        // its control program.
        let cell = s.comp.borrow();
        if let Some(con) = self.control_map.get_mut(&cell.name()) {
            if self.new_fsms {
                con.get_mut_attributes().insert(ir::BoolAttr::NewFSM, 1);
            }
            Ok(Action::change(ir::Cloner::control(con)))
        } else {
            Ok(Action::Continue)
        }
    }
}
