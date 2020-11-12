use crate::frontend::library::ast as lib;
use crate::{
    analysis::ReadWriteSet,
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
use std::collections::{HashMap, HashSet};

type AnalysisSet = HashSet<ir::Id>;

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
    live: HashMap<ir::Id, AnalysisSet>,
}

impl Named for LiveRangeAnalysis {
    fn name() -> &'static str {
        "live-range-analysis"
    }
    fn description() -> &'static str {
        "compute the liveness of each register for every group"
    }
}

pub type Data = AnalysisSet;

impl Visitor<Data> for LiveRangeAnalysis {
    fn start_seq(
        &mut self,
        seq: &mut ir::Seq,
        mut alive: Data,
        comp: &mut Component,
        sigs: &lib::LibrarySignatures,
    ) -> VisResult<Data> {
        eprintln!("start seq");
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
        eprintln!("start par");
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
        eprintln!("start if");
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
        eprintln!("start while");
        let mut start = alive.clone();
        let mut next;

        loop {
            next = while_s.body.visit(self, start.clone(), comp, sigs)?.data;
            next = ir::Control::Enable(ir::Enable::from(while_s.cond.clone()))
                .visit(self, next, comp, sigs)?
                .data;

            if start == next {
                start = next;
                break;
            }

            start = next;
        }

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

        eprintln!("hi: {}", group.name.to_string());

        let reads: HashSet<_> = ReadWriteSet::read_set(&group.assignments)
            .into_iter()
            .filter(|c| c.borrow().type_name() == Some(&"std_reg".into()))
            .map(|c| c.borrow().name.clone())
            .collect();
        let writes: HashSet<_> = ReadWriteSet::write_set(&group.assignments)
            .into_iter()
            .filter(|c| c.borrow().type_name() == Some(&"std_reg".into()))
            .map(|c| c.borrow().name.clone())
            .collect();

        eprintln!("  alive {:?}", alive);
        eprintln!("  reads {:?}", reads);
        eprintln!(" writes {:?}", writes);

        alive = &(&alive - &writes) | &reads;
        eprintln!("  alive {:?}", alive);

        // set the alive set for this enable to be `alive`
        self.live
            .entry(group.name.clone())
            .and_modify(|hs| *hs = alive.clone())
            .or_insert(alive.clone());

        Ok(Action::continue_with(alive))
    }

    fn finish(
        &mut self,
        _alive: Data,
        _comp: &mut Component,
        _sigs: &lib::LibrarySignatures,
    ) -> VisResult<Data> {
        for (key, values) in self.live.iter() {
            eprintln!("{}", key.to_string());
            eprintln!(
                "  {}",
                values
                    .into_iter()
                    .map(|x| x.to_string())
                    .sorted()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        Ok(Action::continue_default())
    }
}
