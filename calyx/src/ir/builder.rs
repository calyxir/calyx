//! IR Builder. Provides convience methods to build various parts of the internal
//! representation.
use crate::ir::{self, LibrarySignatures, RRC, WRC};
use smallvec::smallvec;
use std::cell::RefCell;
use std::rc::Rc;

/// IR builder.
/// Uses internal references to the component to construct and validate
/// constructs when needed.
/// By default, assumes that the cells are being added by a pass and marks
/// them with the `@generated` attribute.
///
/// In order to disable this behavior, call [[ir::Builder::not_generated()]].
pub struct Builder<'a> {
    /// Component for which this builder is constructing.
    pub component: &'a mut ir::Component,
    /// Library signatures.
    lib: &'a LibrarySignatures,
    /// Enable validation of components.
    /// Useful for debugging malformed AST errors.
    validate: bool,
    /// Cells added are generated during a compiler pass.
    generated: bool,
}

impl<'a> Builder<'a> {
    /// Instantiate a new builder using for a component.
    pub fn new(
        component: &'a mut ir::Component,
        lib: &'a LibrarySignatures,
    ) -> Self {
        Self {
            component,
            lib,
            validate: false,
            // By default, assume that builder is called from a pass
            generated: true,
        }
    }

    /// Enable the validation flag on a builder.
    pub fn validate(mut self) -> Self {
        self.validate = true;
        self
    }

    /// Disable the generated flag on the builder
    pub fn not_generated(mut self) -> Self {
        self.generated = false;
        self
    }

    /// Construct a new group and add it to the Component.
    /// The group is guaranteed to start with `prefix`.
    /// Returns a reference to the group.
    pub fn add_group<S>(&mut self, prefix: S) -> RRC<ir::Group>
    where
        S: Into<ir::Id> + ToString + Clone,
    {
        let name = self.component.generate_name(prefix);

        // Check if there is a group with the same name.
        let group = Rc::new(RefCell::new(ir::Group {
            name,
            attributes: ir::Attributes::default(),
            holes: smallvec![],
            assignments: vec![],
        }));

        // Add default holes to the group.
        for (name, width) in &[("go", 1), ("done", 1)] {
            let hole = Rc::new(RefCell::new(ir::Port {
                name: ir::Id::from(*name),
                width: *width,
                direction: ir::Direction::Inout,
                parent: ir::PortParent::Group(WRC::from(&group)),
                attributes: ir::Attributes::default(),
            }));
            group.borrow_mut().holes.push(hole);
        }

        // Add the group to the component.
        self.component.groups.add(Rc::clone(&group));

        group
    }

    /// Construct a combinational group
    pub fn add_comb_group<S>(&mut self, prefix: S) -> RRC<ir::CombGroup>
    where
        S: Into<ir::Id> + ToString + Clone,
    {
        let name = self.component.generate_name(prefix);

        // Check if there is a group with the same name.
        let group = Rc::new(RefCell::new(ir::CombGroup {
            name,
            attributes: ir::Attributes::default(),
            assignments: vec![],
        }));

        // Add the group to the component.
        self.component.comb_groups.add(Rc::clone(&group));

        group
    }

    /// Return reference for a constant cell associated with the (val, width)
    /// pair, building and adding it to the component if needed..
    /// If the constant does not exist, it is added to the Context.
    pub fn add_constant(&mut self, val: u64, width: u64) -> RRC<ir::Cell> {
        let name = ir::Cell::constant_name(val, width);
        // If this constant has already been instantiated, return the relevant
        // cell.
        if let Some(cell) = self
            .component
            .cells
            .iter()
            .find(|&c| *c.borrow().name() == name)
        {
            return Rc::clone(cell);
        }

        // Construct this cell if it's not already present in the context.
        let cell = Self::cell_from_signature(
            name,
            ir::CellType::Constant { val, width },
            vec![(
                "out".into(),
                width,
                ir::Direction::Output,
                ir::Attributes::default(),
            )],
        );

        // Add constant to the Component.
        self.component.cells.add(Rc::clone(&cell));

        cell
    }

    /// Consturcts a primitive cell of type `primitive`.
    /// The name of the cell is guaranteed to start with `prefix`.
    /// Adds this cell to the underlying component and returns a reference
    /// to the Cell.
    ///
    /// For example:
    /// ```
    /// // Construct a std_reg.
    /// builder.add_primitive("fsm", "std_reg", vec![32]);
    /// ```
    pub fn add_primitive<Pre, Prim>(
        &mut self,
        prefix: Pre,
        primitive: Prim,
        param_values: &[u64],
    ) -> RRC<ir::Cell>
    where
        Pre: Into<ir::Id> + ToString + Clone,
        Prim: AsRef<str>,
    {
        let prim_id = ir::Id::from(primitive.as_ref());
        let prim = &self.lib.get_primitive(&prim_id);
        let (param_binding, ports) = prim
            .resolve(param_values)
            .expect("Failed to add primitive.");

        let name = self.component.generate_name(prefix);
        let cell = Self::cell_from_signature(
            name,
            ir::CellType::Primitive {
                name: prim_id,
                param_binding,
                is_comb: prim.is_comb,
            },
            ports,
        );
        if self.generated {
            cell.borrow_mut().add_attribute("generated", 1);
        }
        self.component.cells.add(Rc::clone(&cell));
        cell
    }

    /// Construct an assignment.
    pub fn build_assignment(
        &self,
        dst: RRC<ir::Port>,
        src: RRC<ir::Port>,
        guard: ir::Guard,
    ) -> ir::Assignment {
        // Valid the ports if required.
        if self.validate {
            self.is_port_well_formed(&dst.borrow());
            self.is_port_well_formed(&src.borrow());
            guard
                .all_ports()
                .into_iter()
                .for_each(|p| self.is_port_well_formed(&p.borrow()));
        }
        // If the ports have different widths, error out.
        debug_assert!(
            src.borrow().width == dst.borrow().width,
            "Invalid assignment. `{}.{}' and `{}.{}' have different widths",
            src.borrow().get_parent_name(),
            src.borrow().name,
            dst.borrow().get_parent_name(),
            dst.borrow().name,
        );
        // If ports have the wrong directions, error out.
        debug_assert!(
            // Allow for both Input and Inout ports.
            src.borrow().direction != ir::Direction::Input,
            "Not an ouput port: {}.{}",
            src.borrow().get_parent_name(),
            src.borrow().name
        );
        debug_assert!(
            // Allow for both Input and Inout ports.
            dst.borrow().direction != ir::Direction::Output,
            "Not an input port: {}.{}",
            dst.borrow().get_parent_name(),
            dst.borrow().name
        );

        ir::Assignment {
            dst,
            src,
            guard: Box::new(guard),
        }
    }

    /// Rewrite all reads and writes from `cell` in the given assingments to
    /// the same ports on `new_cell`.
    ///
    /// For example, given with `cell = a` and `new_cell = b`
    /// ```
    /// a.in = a.done ? a.out;
    /// ```
    /// is rewritten to
    /// ```
    /// b.in = b.done ? b.out;
    /// ```
    pub fn rename_port_uses(
        &self,
        rewrites: &[(RRC<ir::Cell>, RRC<ir::Cell>)],
        assigns: &mut Vec<ir::Assignment>,
    ) {
        // Returns true if the port's parent in the given cell.
        let parent_matches =
            |port: &RRC<ir::Port>, cell: &RRC<ir::Cell>| -> bool {
                if let ir::PortParent::Cell(cell_wref) = &port.borrow().parent {
                    Rc::ptr_eq(&cell_wref.upgrade(), cell)
                } else {
                    false
                }
            };

        // Returns a reference to the port with the same name in cell.
        let get_port =
            |port: &RRC<ir::Port>, cell: &RRC<ir::Cell>| -> RRC<ir::Port> {
                Rc::clone(&cell.borrow().get(&port.borrow().name))
            };

        let rewrite_port = |port: &RRC<ir::Port>| -> Option<RRC<ir::Port>> {
            rewrites
                .iter()
                .find(|(cell, _)| parent_matches(port, cell))
                .map(|(_, new_cell)| get_port(port, new_cell))
        };

        for assign in assigns {
            if let Some(new_port) = rewrite_port(&assign.src) {
                assign.src = new_port;
            }
            if let Some(new_port) = rewrite_port(&assign.dst) {
                assign.dst = new_port;
            }
            assign
                .guard
                .for_each(&|port| rewrite_port(&port).map(ir::Guard::port));
        }
    }

    ///////////////////// Internal functions/////////////////////////////////
    /// VALIDATE: Check if the component contains the cell/group associated
    /// with the port exists in the Component.
    /// Validate methods panic! in order to generate a stacktrace to the
    /// offending code.
    fn is_port_well_formed(&self, port: &ir::Port) {
        match &port.parent {
            ir::PortParent::Cell(cell_wref) => {
                let cell_ref = cell_wref.internal.upgrade().expect("Weak reference to port's parent cell points to nothing. This usually means that the Component did not retain a pointer to the Cell.");

                let cell_name = &cell_ref.borrow().name;
                self.component.find_cell(cell_name).expect("Port's parent cell not present in the component. Add the cell to the component before using the Port.");
            }
            ir::PortParent::Group(group_wref) => {
                let group_ref = group_wref.internal.upgrade().expect("Weak reference to hole's parent group points to nothing. This usually means that the Component did not retain a pointer to the Group.");

                let group_name = &group_ref.borrow().name;
                self.component.find_group(group_name).expect("Hole's parent cell not present in the component. Add the group to the component before using the Hole.");
            }
        };
    }
    /// Construct a cell from input/output signature.
    /// Input and output port definition in the form (name, width).
    pub(super) fn cell_from_signature(
        name: ir::Id,
        typ: ir::CellType,
        ports: Vec<(ir::Id, u64, ir::Direction, ir::Attributes)>,
    ) -> RRC<ir::Cell> {
        let cell = Rc::new(RefCell::new(ir::Cell {
            name,
            ports: smallvec![],
            prototype: typ,
            // with_capacity(0) does not allocate space.
            // Same as HashMap::with_capacity
            attributes: ir::Attributes::default(),
        }));
        ports
            .into_iter()
            .for_each(|(name, width, direction, attributes)| {
                let port = Rc::new(RefCell::new(ir::Port {
                    name,
                    width,
                    direction,
                    parent: ir::PortParent::Cell(WRC::from(&cell)),
                    attributes,
                }));
                cell.borrow_mut().ports.push(port);
            });
        cell
    }
}
