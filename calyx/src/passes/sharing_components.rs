use crate::{
    analysis::{GraphColoring, ScheduleConflicts},
    ir,
};
use ir::{
    traversal::{Action, VisResult, Visitor},
    RRC,
};
use itertools::Itertools;
use std::rc::Rc;

pub trait ShareComponents {
    fn initialize(
        &mut self,
        component: &ir::Component,
        library_signatures: &ir::LibrarySignatures,
    ) {
    }
    fn lookup_group_conflicts(&self, group_name: &ir::Id) -> Vec<ir::Id>;
    fn cell_filter(
        &self,
        cell: &ir::Cell,
        sigs: &ir::LibrarySignatures,
    ) -> bool;
    fn cell_equality(&self, cell0: &ir::Cell, cell1: &ir::Cell) -> bool {
        cell0.prototype == cell1.prototype
    }
    fn custom_conflicts(
        &self,
        _comp: &ir::Component,
        _graph: &mut GraphColoring<ir::Id>,
    ) {
        // don't add any conflicts
    }
    fn set_rewrites(&mut self, rewrites: Vec<(RRC<ir::Cell>, RRC<ir::Cell>)>);
    fn get_rewrites<'a>(&'a self) -> &'a [(RRC<ir::Cell>, RRC<ir::Cell>)];
    fn ordering<I>(&self, cells: I) -> Box<dyn Iterator<Item = ir::Id>>
    where
        I: Iterator<Item = RRC<ir::Cell>>,
    {
        Box::new(
            cells
                .map(|cell_ref| cell_ref.borrow().name.clone())
                .sorted(),
        )
    }
}

impl<T: ShareComponents> Visitor for T {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
    ) -> VisResult {
        // call initialize and set self to result
        self.initialize(&comp, &sigs);

        let cells = comp
            .cells
            .iter()
            .filter(|c| self.cell_filter(&c.borrow(), sigs))
            .map(Rc::clone);

        let mut graph: GraphColoring<ir::Id> = GraphColoring::from(
            cells.clone().map(|cell_ref| cell_ref.borrow().name.clone()),
        );
        let par_conflicts = ScheduleConflicts::from(&*comp.control.borrow());

        // Conflict edges between all groups that are enabled in parallel.
        par_conflicts
            .all_conflicts()
            .into_grouping_map_by(|(g1, _)| g1.clone())
            .fold(vec![], |mut acc, _, (_, conflicted_group)| {
                acc.extend(self.lookup_group_conflicts(&conflicted_group));
                acc
            })
            .into_iter()
            .for_each(|(group, confs)| {
                let conflicts = self.lookup_group_conflicts(&group);
                confs
                    .into_iter()
                    // This unique call saves a lot of time!
                    .unique()
                    .for_each(|par_conflict| {
                        for conflict_here in &conflicts {
                            if conflict_here != &par_conflict {
                                graph.insert_conflict(
                                    &conflict_here,
                                    &par_conflict,
                                );
                            }
                        }
                    })
            });

        // add conflicts between things of different types
        cells
            .clone()
            .tuple_combinations()
            .for_each(|(cell0, cell1)| {
                if !self.cell_equality(&*cell0.borrow(), &*cell1.borrow()) {
                    graph.insert_conflict(
                        &cell0.borrow().name,
                        &cell1.borrow().name,
                    );
                }
            });

        // custom conflicts
        self.custom_conflicts(&comp, &mut graph);

        // used a sorted ordering to perform coloring
        let coloring: Vec<_> = graph
            .color_greedy_with(self.ordering(cells))
            .into_iter()
            .filter(|(a, b)| a != b)
            .map(|(a, b)| {
                (comp.find_cell(&a).unwrap(), comp.find_cell(&b).unwrap())
            })
            .collect();

        // apply the coloring as a renaming of registers for both groups
        // and continuous assignments
        let builder = ir::Builder::from(comp, sigs, false);
        for group_ref in &builder.component.groups {
            let mut group = group_ref.borrow_mut();
            let mut assigns: Vec<_> = group.assignments.drain(..).collect();
            builder.rename_port_uses(&coloring, &mut assigns);
            group.assignments = assigns;
        }

        let mut assigns: Vec<_> =
            builder.component.continuous_assignments.drain(..).collect();
        builder.rename_port_uses(&coloring, &mut assigns);
        builder.component.continuous_assignments = assigns;

        self.set_rewrites(coloring);

        Ok(Action::Continue)
    }

    fn start_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) -> VisResult {
        let cond_port = &s.port;
        // let group_name = &s.cond.borrow().name;

        // XXX(sam), is just having a single cell -> cell map for
        // rewrites sufficient. or do you need cell, group_id -> cell

        // find rewrite for conditional port cell
        let rewrite = self.get_rewrites().iter().find(|(c, _)| {
            if let ir::PortParent::Cell(cell_wref) = &cond_port.borrow().parent
            {
                return Rc::ptr_eq(c, &cell_wref.upgrade());
            }
            false
        });

        if let Some((_, new_cell)) = rewrite {
            let new_port = new_cell.borrow().get(&cond_port.borrow().name);
            s.port = new_port;
        }

        Ok(Action::Continue)
    }

    // Rewrite the name of the cond port if this group was re-written.
    fn start_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) -> VisResult {
        let cond_port = &s.port;
        // let group_name = &s.cond.borrow().name;
        // Check if the cell associated with the port was rewritten for the cond
        // group.
        let rewrite = self.get_rewrites().iter().find(|(c, _)| {
            if let ir::PortParent::Cell(cell_wref) = &cond_port.borrow().parent
            {
                return Rc::ptr_eq(c, &cell_wref.upgrade());
            }
            false
        });

        if let Some((_, new_cell)) = rewrite {
            let new_port = new_cell.borrow().get(&cond_port.borrow().name);
            s.port = new_port;
        }
        Ok(Action::Continue)
    }
}
