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
use std::collections::{HashMap, HashSet};

type AnalysisSet = HashMap<ir::Id, HashSet<ir::Id>>;

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
    /// The variables that this ctrl statement generates
    gen: AnalysisSet,
    /// The variables that this ctrl statement kills
    kill: AnalysisSet,
    /// The variables that are live at this statement
    live: AnalysisSet,
}

impl Named for LiveRangeAnalysis {
    fn name() -> &'static str {
        "live-range-analysis"
    }
    fn description() -> &'static str {
        "compute the liveness of each register for every group"
    }
}

impl Visitor<()> for LiveRangeAnalysis {
    /// Runs first. Use this to build up the `gen/kill` sets for every
    fn start(
        &mut self,
        comp: &mut Component,
        _sigs: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        for group_ref in &comp.groups {
            let group = group_ref.borrow();
            let reads = ReadWriteSet::read_set(&group.assignments)
                .into_iter()
                .map(|c| c.borrow().name.clone());
            self.gen.insert(group.name.clone(), reads.collect());

            let writes = ReadWriteSet::write_set(&group.assignments)
                .into_iter()
                .map(|c| c.borrow().name.clone());
            self.kill.insert(group.name.clone(), writes.collect());
        }

        // we want to continue the traversal
        Ok(Action::continue_default())
    }

    fn start_seq(
        &mut self,
        seq: &mut ir::Seq,
        comp: &mut Component,
        sigs: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        // iterate over the seq children in reverse order
        for child in seq.stmts.iter_mut().rev() {
            child.visit(self, comp, sigs)?;
        }

        Ok(Action::skipchildren_default())
    }

    fn finish_seq(
        &mut self,
        seq: &mut ir::Seq,
        _comp: &mut Component,
        _sigs: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        let _lives: HashSet<ir::Id> = HashSet::new();
        // for child in seq.stmts.iter_mut().rev() {
        //     let child_name = child.borrow().name;
        //     *lives = lives
        //         .intersection(self.kill[child])
        //         .union(self.gen[child])
        //         .cloned()
        //         .collect();
        // }

        Ok(Action::continue_default())
    }

    fn start_enable(
        &mut self,
        enable: &mut ir::Enable,
        _comp: &mut Component,
        _sigs: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        let group = enable.group.borrow();
        let group_name = &group.name;
        eprintln!("{}", group.name.to_string());

        let reads: HashSet<_> = ReadWriteSet::read_set(&group.assignments)
            .into_iter()
            .map(|c| c.borrow().name.clone())
            .collect();
        let writes: HashSet<_> = ReadWriteSet::write_set(&group.assignments)
            .into_iter()
            .map(|c| c.borrow().name.clone())
            .collect();

        self.gen
            .entry(group_name.clone())
            .and_modify(|hs| *hs = hs.union(&reads).cloned().collect())
            .or_insert(reads);
        self.kill
            .entry(group_name.clone())
            .and_modify(|hs| *hs = hs.union(&writes).cloned().collect())
            .or_insert(writes);

        eprintln!("  gen: {:?}", self.gen);
        eprintln!("  kil: {:?}", self.kill);

        Ok(Action::continue_default())
    }
}
