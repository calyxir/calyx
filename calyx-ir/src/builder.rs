//! IR Builder. Provides convience methods to build various parts of the internal
//! representation.
use crate::{self as ir, LibrarySignatures, Nothing, RRC, WRC};
use calyx_frontend::BoolAttr;
use calyx_utils::CalyxResult;
use std::rc::Rc;

use super::{CellType, PortDef};

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

    pub fn add_continuous_assignments(
        &mut self,
        assigns: Vec<ir::Assignment<Nothing>>,
    ) {
        self.component.continuous_assignments.extend(assigns);
    }

    /// Construct a new group and add it to the Component.
    /// The group is guaranteed to start with `prefix`.
    /// Returns a reference to the group.
    pub fn add_group<S>(&mut self, prefix: S) -> RRC<ir::Group>
    where
        S: Into<ir::Id>,
    {
        let prefix: ir::Id = prefix.into();
        assert!(
            prefix != "",
            "Cannot construct group with empty name prefix"
        );
        let name = self.component.generate_name(prefix);

        // Check if there is a group with the same name.
        let group = ir::rrc(ir::Group::new(name));

        // Add default holes to the group.
        for (name, width) in &[("go", 1), ("done", 1)] {
            let hole = ir::rrc(ir::Port {
                name: ir::Id::from(*name),
                width: *width,
                direction: ir::Direction::Inout,
                parent: ir::PortParent::Group(WRC::from(&group)),
                attributes: ir::Attributes::default(),
            });
            group.borrow_mut().holes.push(hole);
        }

        // Add the group to the component.
        self.component.get_groups_mut().add(Rc::clone(&group));

        group
    }

    /// Construct a new static group and add it to the Component.
    /// The group is guaranteed to start with `prefix`.
    /// Returns a reference to the group.
    pub fn add_static_group<S>(
        &mut self,
        prefix: S,
        latency: u64,
    ) -> RRC<ir::StaticGroup>
    where
        S: Into<ir::Id>,
    {
        let prefix: ir::Id = prefix.into();
        assert!(
            prefix != "",
            "Cannot construct group with empty name prefix"
        );
        let name = self.component.generate_name(prefix);

        // Check if there is a group with the same name.
        let group = ir::rrc(ir::StaticGroup::new(name, latency));

        // Add default holes to the group.
        // Static Groups don't need a done hole.
        // May be beneficial to have a go hole, though (although maybe not)
        let (name, width) = ("go", 1);
        let hole = ir::rrc(ir::Port {
            name: ir::Id::from(name),
            width,
            direction: ir::Direction::Inout,
            parent: ir::PortParent::StaticGroup(WRC::from(&group)),
            attributes: ir::Attributes::default(),
        });
        group.borrow_mut().holes.push(hole);

        // Add the group to the component.
        self.component
            .get_static_groups_mut()
            .add(Rc::clone(&group));

        group
    }

    /// Construct a combinational group
    pub fn add_comb_group<S>(&mut self, prefix: S) -> RRC<ir::CombGroup>
    where
        S: Into<ir::Id> + ToString + Clone,
    {
        let name = self.component.generate_name(prefix);

        // Check if there is a group with the same name.
        let group = ir::rrc(ir::CombGroup {
            name,
            attributes: ir::Attributes::default(),
            assignments: vec![],
        });

        // Add the group to the component.
        self.component.comb_groups.add(Rc::clone(&group));

        group
    }

    /// Return reference for a constant cell associated with the (val, width)
    /// pair, building and adding it to the component if needed..
    /// If the constant does not exist, it is added to the Context.
    pub fn add_constant(&mut self, val: u64, width: u64) -> RRC<ir::Cell> {
        // Ensure that the value can fit within the width
        assert!(
            // This calculates the position of the most significant 1 bit which
            // tells us the minimum number of bits required to represent the
            // constant. Note that this will not work for constants that require
            // more than 64 bits as those currently cannot be parsed
            (64 - val.leading_zeros()) as u64 <= width,
            "Constant value {} cannot fit in {} bits",
            val,
            width
        );
        let name = ir::Cell::constant_name(val, width);
        // If this constant has already been instantiated, return the relevant
        // cell.
        if let Some(cell) = self.component.cells.find(name) {
            return Rc::clone(&cell);
        }

        // Construct this cell if it's not already present in the context.
        let cell = Self::cell_from_signature(
            name,
            ir::CellType::Constant { val, width },
            vec![ir::PortDef::new(
                ir::Id::from("out"),
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
        Prim: Into<ir::Id>,
    {
        self.try_add_primitive(prefix, primitive, param_values)
            .expect("failed to add primitive:")
    }

    /// Result variant of [[ir::Builder::add_primitive()]].
    pub fn try_add_primitive<Pre, Prim>(
        &mut self,
        prefix: Pre,
        primitive: Prim,
        param_values: &[u64],
    ) -> CalyxResult<RRC<ir::Cell>>
    where
        Pre: Into<ir::Id> + ToString + Clone,
        Prim: Into<ir::Id>,
    {
        let prim_id = primitive.into();
        let prim = &self.lib.get_primitive(prim_id);
        let (param_binding, ports) = prim.resolve(param_values)?;

        let name = self.component.generate_name(prefix);
        let cell = Self::cell_from_signature(
            name,
            ir::CellType::Primitive {
                name: prim_id,
                param_binding: Box::new(param_binding),
                is_comb: prim.is_comb,
                latency: prim.latency,
            },
            ports,
        );
        if self.generated {
            cell.borrow_mut().add_attribute(BoolAttr::Generated, 1);
        }
        self.component.cells.add(Rc::clone(&cell));
        Ok(cell)
    }

    /// Add a component instance to this component using its name and port
    /// signature.
    pub fn add_component<Pre>(
        &mut self,
        prefix: Pre,
        component: Pre,
        sig: Vec<PortDef<u64>>,
    ) -> RRC<ir::Cell>
    where
        Pre: Into<ir::Id> + ToString + Clone,
    {
        let name = self.component.generate_name(prefix);
        let cell = Self::cell_from_signature(
            name,
            CellType::Component {
                name: component.into(),
            },
            sig,
        );
        if self.generated {
            cell.borrow_mut().add_attribute(BoolAttr::Generated, 1);
        }
        self.component.cells.add(Rc::clone(&cell));
        cell
    }

    /// Construct an assignment.
    pub fn build_assignment<T>(
        &self,
        dst: RRC<ir::Port>,
        src: RRC<ir::Port>,
        guard: ir::Guard<T>,
    ) -> ir::Assignment<T> {
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
            attributes: ir::Attributes::default(),
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

                let cell = &cell_ref.borrow();
                self.component.find_cell(cell.name()).expect("Port's parent cell not present in the component. Add the cell to the component before using the Port.");
            }
            ir::PortParent::Group(group_wref) => {
                let group_ref = group_wref.internal.upgrade().expect("Weak reference to hole's parent group points to nothing. This usually means that the Component did not retain a pointer to the Group.");

                let group = &group_ref.borrow();
                self.component.find_group(group.name()).expect("Hole's parent cell not present in the component. Add the group to the component before using the Hole.");
            }
            ir::PortParent::StaticGroup(group_wref) => {
                let group_ref = group_wref.internal.upgrade().expect("Weak reference to hole's parent group points to nothing. This usually means that the Component did not retain a pointer to the Group.");

                let group = &group_ref.borrow();
                self.component.find_static_group(group.name()).expect("Hole's parent cell not present in the component. Add the static group to the component before using the Hole.");
            }
        };
    }
    /// Construct a cell from input/output signature.
    /// Input and output port definition in the form (name, width).
    pub(super) fn cell_from_signature(
        name: ir::Id,
        typ: ir::CellType,
        ports: Vec<ir::PortDef<u64>>,
    ) -> RRC<ir::Cell> {
        let cell = ir::rrc(ir::Cell::new(name, typ));
        ports.into_iter().for_each(|pd| {
            let port = ir::rrc(ir::Port {
                name: pd.name(),
                width: pd.width,
                direction: pd.direction,
                parent: ir::PortParent::Cell(WRC::from(&cell)),
                attributes: pd.attributes,
            });
            cell.borrow_mut().ports.push(port);
        });
        cell
    }
}
