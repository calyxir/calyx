use crate::frontend::library::ast as lib;
use crate::{
    analysis::{ReadWriteSet, VariableDetection},
    ir::{
        self,
        traversal::{Named, Visitor},
    },
};
use ir::{
    traversal::{Action, VisResult, Visitable},
    Component,
};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

pub type Data = HashSet<ir::Id>;

/// Computes the control statements that a stateful cell is 'live' for.
///
/// Each group has a `writes-to` and `reads-from` list.
///  - `writes-to` is defined as all the cells that appear on the lhs of all the assignments in a group
///  - `reads-from` is defined as all the cells on the rhs of all the assignments in a group
///
/// A stateful cell is live if it is read from before it is written to again.
///
#[derive(Default)]
pub struct LiveRangeAnalysis {
    /// The variables that are live at this statement
    live: HashMap<ir::Id, Data>,
}

impl LiveRangeAnalysis {
    pub fn get(&self, group: &ir::Group) -> &Data {
        &self.live[&group.name]
    }

    fn data_string(data: &Data) -> String {
        data.iter()
            .map(|x| x.to_string())
            .sorted()
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl Named for LiveRangeAnalysis {
    fn name() -> &'static str {
        "live-range-analysis"
    }
    fn description() -> &'static str {
        "compute the liveness of each register for every group"
    }
}

impl Visitor<Data> for LiveRangeAnalysis {
    fn start_seq(
        &mut self,
        seq: &mut ir::Seq,
        mut alive: Data,
        comp: &mut Component,
        sigs: &lib::LibrarySignatures,
    ) -> VisResult<Data> {
        // iterate over the seq children in reverse order
        for child in seq.stmts.iter_mut().rev() {
            alive = child.visit(self, alive, comp, sigs)?.data;
        }

        Ok(Action::skipchildren_with(alive))
    }

    fn start_par(
        &mut self,
        par: &mut ir::Par,
        data: Data,
        comp: &mut Component,
        sigs: &lib::LibrarySignatures,
    ) -> VisResult<Data> {
        let mut alive = data.clone();
        for child in par.stmts.iter_mut() {
            alive = &alive | &child.visit(self, data.clone(), comp, sigs)?.data;
        }
        Ok(Action::skipchildren_with(alive))
    }

    fn start_if(
        &mut self,
        if_s: &mut ir::If,
        mut alive: Data,
        comp: &mut Component,
        sigs: &lib::LibrarySignatures,
    ) -> VisResult<Data> {
        // xxx deal with condition
        let t_alive = if_s.tbranch.visit(self, alive.clone(), comp, sigs)?.data;
        let f_alive = if_s.fbranch.visit(self, alive.clone(), comp, sigs)?.data;

        alive = &alive | &t_alive;
        alive = &alive | &f_alive;

        Ok(Action::skipchildren_with(alive))
    }

    fn start_while(
        &mut self,
        while_s: &mut ir::While,
        alive: Data,
        comp: &mut Component,
        sigs: &lib::LibrarySignatures,
    ) -> VisResult<Data> {
        let mut start = alive.clone();
        let mut next;

        eprintln!("start while");
        loop {
            eprintln!("  start: {}", LiveRangeAnalysis::data_string(&start));
            next = while_s.body.visit(self, start.clone(), comp, sigs)?.data;
            next = ir::Control::Enable(ir::Enable::from(while_s.cond.clone()))
                .visit(self, next, comp, sigs)?
                .data;
            eprintln!("  next: {}", LiveRangeAnalysis::data_string(&next));

            if start == next {
                start = next;
                break;
            }

            start = next;
        }
        eprintln!("end while");

        Ok(Action::skipchildren_with(start))
    }

    fn start_enable(
        &mut self,
        enable: &mut ir::Enable,
        mut alive: Data,
        _comp: &mut Component,
        _sigs: &lib::LibrarySignatures,
    ) -> VisResult<Data> {
        let group = enable.group.borrow();
        self.live
            .entry(group.name.clone())
            .and_modify(|hs| *hs = &*hs | &alive)
            .or_insert(alive.clone());

        // if the group contains what looks like a variable write,
        // then just add variable to write set
        if let Some(variable) =
            VariableDetection::variable_like(Rc::clone(&enable.group))
        {
            alive.remove(&variable);
        } else {
            let reads: HashSet<_> = ReadWriteSet::read_set(&group.assignments)
                .into_iter()
                .filter(|c| c.borrow().type_name() == Some(&"std_reg".into()))
                .map(|c| c.borrow().name.clone())
                .collect();

            // XXX(sam) the writes are incorrect atm
            let writes: HashSet<_> =
                ReadWriteSet::write_set(&group.assignments)
                    .into_iter()
                    .filter(|c| {
                        c.borrow().type_name() == Some(&"std_reg".into())
                    })
                    .map(|c| c.borrow().name.clone())
                    .collect();

            alive = &(&alive - &writes) | &reads;
        }

        Ok(Action::continue_with(alive))
    }
}
