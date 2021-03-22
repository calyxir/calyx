use super::sharing_components::ShareComponents;
use crate::analysis;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures, RRC,
};
use itertools::Itertools;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
#[allow(clippy::type_complexity)]
/// Rewrites groups to share cells marked with the "share" attribute
/// when the groups are guaranteed to never run in parallel.
pub struct ResourceSharing {
    // /// Mapping from the name of a group the cells that have been used by it.
    // used_cells: HashMap<ir::Id, Vec<RRC<ir::Cell>>>,
    shareable_components: HashMap<ir::Id, Vec<ir::Id>>,

    // /// Mapping from the name of a group to the (old_cell, new_cell) pairs.
    // /// This is used to rewrite all uses of `old_cell` with `new_cell` in the
    // /// group.
    // rewrites: HashMap<ir::Id, Vec<(RRC<ir::Cell>, RRC<ir::Cell>)>>,
    rewrites: Vec<(RRC<ir::Cell>, RRC<ir::Cell>)>,
}

impl Named for ResourceSharing {
    fn name() -> &'static str {
        "resource-sharing"
    }

    fn description() -> &'static str {
        "shares resources between groups that don't execute in parallel"
    }
}

impl ShareComponents for ResourceSharing {
    fn initialize(
        &mut self,
        component: &ir::Component,
        sigs: &ir::LibrarySignatures,
    ) {
        self.shareable_components = component
            .groups
            .iter()
            .map(|group| {
                (
                    group.borrow().name.clone(),
                    analysis::ReadWriteSet::uses(&group.borrow().assignments)
                        .into_iter()
                        .filter(|cell| self.cell_filter(&cell.borrow(), sigs))
                        .map(|cell| cell.borrow().name.clone())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
    }

    fn lookup_group_conflicts(&self, group_name: &ir::Id) -> Vec<ir::Id> {
        self.shareable_components[group_name].clone()
    }

    fn cell_filter(
        &self,
        cell: &ir::Cell,
        sigs: &ir::LibrarySignatures,
    ) -> bool {
        if let ir::CellType::Primitive {
            name: prim_type, ..
        } = &cell.prototype
        {
            sigs.get_primitive(&prim_type).attributes.get("share") == Some(&1)
        } else {
            false
        }
    }

    fn custom_conflicts(
        &self,
        _comp: &ir::Component,
        graph: &mut analysis::GraphColoring<ir::Id>,
    ) {
        for (_group, confs) in &self.shareable_components {
            graph.insert_conflicts(confs.iter());
        }
    }

    fn set_rewrites(&mut self, rewrites: Vec<(RRC<ir::Cell>, RRC<ir::Cell>)>) {
        self.rewrites = rewrites;
    }
    fn get_rewrites<'a>(&'a self) -> &'a [(RRC<ir::Cell>, RRC<ir::Cell>)] {
        &self.rewrites
    }
}

// /// Returns the name of the primitive used to construct this cell if the
// /// primitive's "share" attribute is set to 1.
// fn is_shareable(cell: &RRC<ir::Cell>, sigs: &LibrarySignatures) -> bool {
//     if let ir::CellType::Primitive {
//         name: prim_type, ..
//     } = &cell.borrow().prototype
//     {
//         sigs.get_primitive(&prim_type).attributes.get("share") == Some(&1)
//     } else {
//         false
//     }
// }

// impl Visitor for ResourceSharing {
//     fn start(
//         &mut self,
//         comp: &mut ir::Component,
//         sigs: &LibrarySignatures,
//     ) -> VisResult {
//         // construct map from group names to all the shareable
//         // components read or written by the group
//         // let shareable_component_map: HashMap<ir::Id, Vec<ir::Id>> = comp
//         //     .groups
//         //     .iter()
//         //     .map(|group| {
//         //         (
//         //             group.borrow().name.clone(),
//         //             analysis::ReadWriteSet::uses(&group.borrow().assignments)
//         //                 .into_iter()
//         //                 .filter(|cell| is_shareable(cell, &sigs))
//         //                 .map(|cell| cell.borrow().name.clone())
//         //                 .collect::<Vec<_>>(),
//         //         )
//         //     })
//         //     .collect();

//         // eprintln!("{:#?}", shareable_component_map);

//         // // cells that are shareable
//         // let shareable_components = comp
//         //     .cells
//         //     .iter()
//         //     .filter(|cell| is_shareable(cell, &sigs))
//         //     .map(|r| Rc::clone(r));

//         // // construct initial graph
//         // let mut graph: analysis::GraphColoring<ir::Id> =
//         //     SharingComponent::initial_conflict_graph(
//         //         shareable_components.clone(),
//         //         &*comp.control.borrow(),
//         //         |group_id| {
//         //             shareable_component_map
//         //                 .get(group_id)
//         //                 .expect("Group should exist")
//         //                 .clone()
//         //         },
//         //         // cells are equivalent when their cell prototypes are the same
//         //         |cell0, cell1| cell0.prototype == cell1.prototype,
//         //     );

//         // // add conflicts for components in use in the same group
//         // for (_group, confs) in shareable_component_map {
//         //     graph.insert_conflicts(confs.iter());
//         // }

//         // // do coloring, using a sorted list of shareable cell names as the ordering
//         // let ordering = shareable_components
//         //     .map(|cell| cell.borrow().name.clone())
//         //     .sorted();
//         // let coloring: Vec<_> = graph
//         //     .color_greedy_with(ordering)
//         //     .into_iter()
//         //     .filter(|(a, b)| a != b)
//         //     .map(|(a, b)| {
//         //         (comp.find_cell(&a).unwrap(), comp.find_cell(&b).unwrap())
//         //     })
//         //     .collect();

//         // eprintln!(
//         //     "{:#?}",
//         //     coloring
//         //         .iter()
//         //         .map(|(c0, c1)| {
//         //             format!(
//         //                 "{} -> {}",
//         //                 c0.borrow().name.as_ref(),
//         //                 c1.borrow().name.as_ref()
//         //             )
//         //         })
//         //         .collect::<Vec<_>>()
//         // );

//         // // Mapping from the name of the primitive to all cells that use it.
//         // let mut cell_map: HashMap<(ir::Id, ir::Binding), Vec<RRC<ir::Cell>>> =
//         //     HashMap::new();
//         // for cell in &comp.cells {
//         //     if let Some(name_and_binding) = shareable_primitive_name(cell, sigs)
//         //     {
//         //         cell_map
//         //             .entry(name_and_binding.clone())
//         //             .or_default()
//         //             .push(Rc::clone(cell))
//         //     }
//         // }

//         // // Create conflict graph for each cell type
//         // // let coloring_map: HashMap<
//         // //     (ir::Id, ir::Binding),
//         // //     analysis::GraphColoring<ir::Id>,
//         // // > = cell_map
//         // //     .into_iter()
//         // //     .map(|(id, cells)| {
//         // //         (
//         // //             id,
//         // //             analysis::GraphColoring::from(
//         // //                 cells.iter().map(|c| c.borrow().name.clone()),
//         // //             ),
//         // //         )
//         // //     })
//         // //     .collect();

//         // let conflicts =
//         //     analysis::ScheduleConflicts::from(&*comp.control.borrow());

//         // // Sort groups in descending order of number of conflicts.
//         // let sorted = comp.groups.iter().sorted_by(|g1, g2| {
//         //     // XXX(rachit): Potential performance pitfall since
//         //     // all_conflicts iterates over the conflicts graph.
//         //     conflicts
//         //         .conflicts_with(&g2.borrow().name)
//         //         .len()
//         //         .cmp(&conflicts.conflicts_with(&g1.borrow().name).len())
//         // });

//         // for group in sorted {
//         //     // Find all the primitives already used by neighbours.
//         //     let all_conflicts = conflicts
//         //         .conflicts_with(&group.borrow().name)
//         //         .into_iter()
//         //         .flat_map(|g| self.used_cells.get(&g))
//         //         .cloned()
//         //         .flatten()
//         //         .collect::<Vec<RRC<_>>>();

//         //     // Cells used by the generated assignment for this group.
//         //     let mut used_cells: Vec<RRC<ir::Cell>> = Vec::new();
//         //     // Rewrites generated for this group.
//         //     let mut rewrites: Vec<(RRC<ir::Cell>, RRC<ir::Cell>)> = Vec::new();

//         //     for old_cell in
//         //         analysis::ReadWriteSet::uses(&group.borrow().assignments)
//         //     {
//         //         // If this is a primitive cell
//         //         if let Some(name_and_binding) =
//         //             shareable_primitive_name(&old_cell, sigs)
//         //         {
//         //             // Find a cell of this primitive type that hasn't been used
//         //             // by the neighbours.
//         //             let cell = cell_map[&name_and_binding]
//         //                 .iter()
//         //                 .find(|c| {
//         //                     !all_conflicts
//         //                         .iter()
//         //                         .chain(used_cells.iter())
//         //                         .any(|uc| Rc::ptr_eq(uc, c))
//         //                 })
//         //                 .expect("Failed to find a non-conflicting cell.");

//         //             // Rewrite current cell to the new cell.
//         //             rewrites.push((Rc::clone(&old_cell), Rc::clone(cell)));

//         //             // Save the performed assignment to `cell_assigns`.
//         //             used_cells.push(Rc::clone(cell));
//         //         }
//         //     }

//         //     // Add the used cells and the rewrites to the workspace.
//         //     self.used_cells
//         //         .insert(group.borrow().name.clone(), used_cells);
//         //     self.rewrites.insert(group.borrow().name.clone(), rewrites);
//         // }

//         // let builder = ir::Builder::from(comp, sigs, false);

//         // // Apply the generated rewrites
//         // for group_ref in &builder.component.groups {
//         //     let mut group = group_ref.borrow_mut();
//         //     let mut assigns = group.assignments.drain(..).collect::<Vec<_>>();
//         //     builder.rename_port_uses(&self.rewrites[&group.name], &mut assigns);
//         //     group.assignments = assigns;
//         // }

//         Ok(Action::Continue)
//     }

//     // Rewrite the name of the cond port if this group was re-written.
//     fn start_if(
//         &mut self,
//         s: &mut ir::If,
//         _comp: &mut ir::Component,
//         _sigs: &LibrarySignatures,
//     ) -> VisResult {
//         let cond_port = &s.port;
//         let group_name = &s.cond.borrow().name;
//         // Check if the cell associated with the port was rewritten for the cond
//         // group.
//         let rewrite = self.rewrites[group_name].iter().find(|(c, _)| {
//             if let ir::PortParent::Cell(cell_wref) = &cond_port.borrow().parent
//             {
//                 return Rc::ptr_eq(c, &cell_wref.upgrade());
//             }
//             false
//         });

//         if let Some((_, new_cell)) = rewrite {
//             let new_port = new_cell.borrow().get(&cond_port.borrow().name);
//             s.port = new_port;
//         }

//         Ok(Action::Continue)
//     }

//     // Rewrite the name of the cond port if this group was re-written.
//     fn start_while(
//         &mut self,
//         s: &mut ir::While,
//         _comp: &mut ir::Component,
//         _sigs: &LibrarySignatures,
//     ) -> VisResult {
//         let cond_port = &s.port;
//         let group_name = &s.cond.borrow().name;
//         // Check if the cell associated with the port was rewritten for the cond
//         // group.
//         let rewrite = self.rewrites[group_name].iter().find(|(c, _)| {
//             if let ir::PortParent::Cell(cell_wref) = &cond_port.borrow().parent
//             {
//                 return Rc::ptr_eq(c, &cell_wref.upgrade());
//             }
//             false
//         });

//         if let Some((_, new_cell)) = rewrite {
//             let new_port = new_cell.borrow().get(&cond_port.borrow().name);
//             s.port = new_port;
//         }
//         Ok(Action::Continue)
//     }
// }
