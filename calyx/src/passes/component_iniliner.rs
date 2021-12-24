use std::collections::HashMap;
use std::rc::Rc;

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
pub struct ComponentInliner;

impl ComponentInliner {
    /// Inline a cell definition into the component associated with the `builder`.
    fn inline_cell(
        builder: &mut ir::Builder,
        cell_ref: &RRC<ir::Cell>,
    ) -> (RRC<ir::Cell>, RRC<ir::Cell>) {
        let cell = cell_ref.borrow();
        let cn = cell.clone_name();
        let new_cell = match &cell.prototype {
            ir::CellType::Primitive {
                name,
                param_binding,
                ..
            } => builder.add_primitive(
                cn,
                name,
                &param_binding.iter().map(|(_, v)| *v).collect_vec(),
            ),
            ir::CellType::Component { name } => {
                builder.add_component(cn, name.clone(), cell.get_signature())
            }
            ir::CellType::Constant { val, width } => {
                builder.add_constant(*val, *width)
            }
            ir::CellType::ThisComponent => unreachable!(),
        };
        (Rc::clone(cell_ref), new_cell)
    }

    /// Inline a group definition from a component into the component associated
    /// with the `builder`.
    fn inline_group(
        builder: &mut ir::Builder,
        cell_map: &Vec<(RRC<ir::Cell>, RRC<ir::Cell>)>,
        gr: &RRC<ir::Group>,
    ) -> (RRC<ir::Group>, RRC<ir::Group>) {
        let group = gr.borrow();
        let new_group = builder.add_group(group.clone_name());
        new_group.borrow_mut().attributes = group.attributes.clone();

        // Rewrite assignments
        let mut asgns = group.assignments.clone();
        ir::Builder::rename_port_uses(&cell_map, &mut asgns);
        new_group.borrow_mut().assignments = asgns;
        (Rc::clone(gr), new_group)
    }

    /// Inline component `comp` into the parent component attached to `builder`
    fn inline_component(builder: &mut ir::Builder, comp: &ir::Component) {
        // For each cell in the component, create a new cell in the parent
        // of the same type and build a rewrite map using it.
        let cell_map = comp
            .cells
            .iter()
            .map(|cell_ref| Self::inline_cell(builder, cell_ref))
            .collect::<Vec<_>>();

        // For each group, create a new group and rewrite all assignments within
        // it using the `rewrite_map`.
        let group_map = comp
            .groups
            .iter()
            .map(|gr| Self::inline_group(builder, &cell_map, gr))
            .collect::<Vec<_>>();
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
        let cells = comp.cells.drain().collect_vec();
        let mut builder = ir::Builder::new(comp, sigs);

        // Mapping from component name to component definition
        let comp_map = comps
            .iter()
            .map(|comp| (comp.name.clone(), comp))
            .collect::<HashMap<_, _>>();

        for cell_ref in &cells {
            let cell = cell_ref.borrow();
            if cell.is_component() && cell.get_attribute("inline").is_some() {
                let comp_name = cell.type_name().unwrap();
                Self::inline_component(&mut builder, comp_map[comp_name]);
            }
        }

        // Add back all the cells in original order.
        let mut new_cells = ir::IdList::from(cells);
        new_cells.append(comp.cells.drain());
        comp.cells = new_cells;

        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        _s: &mut ir::Invoke,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }
}
