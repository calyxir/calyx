//! IR Builder. Provides convience methods to build various parts of the internal
//! representation.
use crate::frontend::library::ast::LibrarySignatures;
use crate::ir::{self, RRC};
use crate::utils;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// An IR builder.
/// Uses internal references to the component to construct and validate
/// constructs when needed.
pub struct Builder<'a> {
    /// Component for which this builder is constructing.
    pub component: &'a mut ir::Component,
    /// Library signatures.
    pub lib_sigs: &'a LibrarySignatures,
    /// Enable validation of components.
    /// Useful for debugging malformed AST errors.
    pub validate: bool,
    /// Internal name generator.
    namegen: utils::NameGenerator,
}

impl<'a> Builder<'a> {
    /// Instantiate a new builder using for a component.
    pub fn from(
        component: &'a mut ir::Component,
        lib_sigs: &'a LibrarySignatures,
        validate: bool,
    ) -> Self {
        Self {
            component,
            lib_sigs,
            validate,
            namegen: utils::NameGenerator::default(),
        }
    }

    /// Construct a new group and add it to the Component.
    /// The group is guaranteed to start with `prefix`.
    /// Returns a reference to the group.
    pub fn add_group<S: std::fmt::Display + Clone + AsRef<str>>(
        &mut self,
        prefix: S,
        attributes: HashMap<String, u64>,
    ) -> RRC<ir::Group> {
        let name = self.namegen.gen_name(prefix.as_ref());

        // Check if there is a group with the same name.
        let group = Rc::new(RefCell::new(ir::Group {
            name: ir::Id::from(name.as_ref()),
            attributes,
            holes: vec![],
            assignments: vec![],
        }));

        // Add default holes to the group.
        for (name, width) in vec![("go", 1), ("done", 1)] {
            let hole = Rc::new(RefCell::new(ir::Port {
                name: name.into(),
                width,
                direction: ir::Direction::Inout,
                parent: ir::PortParent::Group(Rc::downgrade(&group)),
            }));
            group.borrow_mut().holes.push(hole);
        }

        // Add the group to the component.
        self.component.groups.push(Rc::clone(&group));

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
            .find(|&c| c.borrow().name == name)
        {
            return Rc::clone(cell);
        }

        // Construct this cell if it's not already present in the context.
        let cell = Self::cell_from_signature(
            name.clone(),
            ir::CellType::Constant { val, width },
            vec![],
            vec![("out".into(), width)],
        );

        // Add constant to the Component.
        self.component.cells.push(Rc::clone(&cell));

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
    pub fn add_primitive<S, P>(
        &mut self,
        prefix: S,
        primitive: P,
        param_values: &[u64],
    ) -> RRC<ir::Cell>
    where
        S: std::fmt::Display + Clone + AsRef<str>,
        P: std::fmt::Display + Clone + AsRef<str>,
    {
        let prim_id = ir::Id::from(primitive.as_ref());
        let prim = &self.lib_sigs[&prim_id];
        let (param_binding, inputs, outputs) = prim
            .resolve(param_values)
            .expect("Failed to add primitive.");

        let name = self.namegen.gen_name(prefix.as_ref()).into();
        let cell = Self::cell_from_signature(
            name,
            ir::CellType::Primitive {
                name: prim_id,
                param_binding,
            },
            inputs,
            outputs,
        );
        self.component.cells.push(Rc::clone(&cell));
        cell
    }

    /// Construct an assignment.
    pub fn build_assignment(
        &self,
        dst: RRC<ir::Port>,
        src: RRC<ir::Port>,
        guard: Option<ir::Guard>,
    ) -> ir::Assignment {
        // Valid the ports if required.
        if self.validate {
            self.is_port_well_formed(&dst.borrow());
            self.is_port_well_formed(&src.borrow());
            if let Some(g) = &guard {
                g.all_ports()
                    .into_iter()
                    .for_each(|p| self.is_port_well_formed(&p.borrow()))
            }
        }
        // Validate: Check to see if the cell/group associated with the
        // port is in the component.
        ir::Assignment { dst, src, guard }
    }

    ///////////////////// Internal function ////////////////////
    /// VALIDATE: Check if the component contains the cell/group associated
    /// with the port exists in the Component.
    /// Validate methods panic! in order to generate a stacktrace to the
    /// offending code.
    fn is_port_well_formed(&self, port: &ir::Port) -> () {
        match &port.parent {
            ir::PortParent::Cell(cell_wref) => {
                let cell_ref = cell_wref.upgrade().expect("Weak reference to port's parent cell points to nothing. This usually means that the Component did not retain a pointer to the Cell.");

                let cell_name = &cell_ref.borrow().name;
                self.component.find_cell(cell_name).expect("Port's parent cell not present in the component. Add the cell to the component before using the Port.");
            }
            ir::PortParent::Group(group_wref) => {
                let group_ref = group_wref.upgrade().expect("Weak reference to hole's parent group points to nothing. This usually means that the Component did not retain a pointer to the Group.");

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
        inputs: Vec<(ir::Id, u64)>,
        outputs: Vec<(ir::Id, u64)>,
    ) -> RRC<ir::Cell> {
        let cell = Rc::new(RefCell::new(ir::Cell {
            name,
            ports: vec![],
            prototype: typ,
        }));
        // Construct ports
        for (name, width) in inputs {
            let port = Rc::new(RefCell::new(ir::Port {
                name,
                width,
                direction: ir::Direction::Input,
                parent: ir::PortParent::Cell(Rc::downgrade(&cell)),
            }));
            cell.borrow_mut().ports.push(port);
        }
        for (name, width) in outputs {
            let port = Rc::new(RefCell::new(ir::Port {
                name,
                width,
                direction: ir::Direction::Output,
                parent: ir::PortParent::Cell(Rc::downgrade(&cell)),
            }));
            cell.borrow_mut().ports.push(port);
        }
        cell
    }
}
