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
    Component, RRC,
};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    ops::BitOr,
    rc::Rc,
};

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Data {
    gen: HashSet<ir::Id>,
    kill: HashSet<ir::Id>,
    live: HashSet<ir::Id>,
}

impl BitOr<&Data> for &Data {
    type Output = Data;
    fn bitor(self, rhs: &Data) -> Self::Output {
        Data {
            gen: &self.gen | &rhs.gen,
            kill: &self.kill | &rhs.kill,
            live: &self.live | &rhs.live,
        }
    }
}

impl Data {
    fn transfer(mut self) -> Self {
        self.live = &(&self.live - &self.kill) | &self.gen;
        self
    }
}

/// Computes the control statements that a stateful cell is 'live' for.
///
/// Each group has a `writes-to` and `reads-from` list.
///  - `writes-to` is defined as all the cells that appear on the lhs of all the assignments in a group
///  - `reads-from` is defined as all the cells on the rhs of all the assignments in a group
///
/// A stateful cell is live if it is read from before it is written to again.
#[derive(Default)]
pub struct LiveRangeAnalysis {
    /// The variables that are live at this statement
    live: HashMap<ir::Id, HashSet<ir::Id>>,
}

impl LiveRangeAnalysis {
    pub fn get(&self, group: &ir::Group) -> &HashSet<ir::Id> {
        &self.live[&group.name]
    }

    /// Compute the `gen` and `kill` sets for a given group definition. Because
    /// we can't always know if a group will *definitely* kill something or *definitely*
    /// read something, this function is conservative.
    ///
    /// However, it is conservative in different directions for `gens` and `kills`.
    /// In particular, it is always ok to accidentally put something in `gens` because
    /// in the worst case we say something is alive when it isn't.
    ///
    /// However, it is **never** ok to accidentally put something in `writes` because
    /// we might accidentally kill something that should be alive.
    ///
    /// To implement this, we say that something is being read if it shows up on the rhs
    /// of any assignment in a group. Something is written if it it's guard is `1` or if it has no guard.
    fn find_gen_kill(
        group_ref: RRC<ir::Group>,
    ) -> (HashSet<ir::Id>, HashSet<ir::Id>) {
        let group = group_ref.borrow();
        // if the group contains what looks like a variable write,
        // then just add variable to write set
        if let Some(variable) =
            VariableDetection::variable_like(Rc::clone(&group_ref))
        {
            // we don't want to read the control signal of `variable`
            let assignments = group
                .assignments
                .iter()
                .filter(|asgn| {
                    !(asgn.src.borrow().get_parent_name() == variable
                        && asgn.src.borrow().name == "done")
                })
                .cloned()
                .collect::<Vec<_>>();

            // calculate reads, but ignore `variable`. we've already dealt with that
            let reads: HashSet<_> = ReadWriteSet::read_set(&assignments)
                .into_iter()
                .filter(|c| c.borrow().type_name() == Some(&"std_reg".into()))
                .map(|c| c.borrow().name.clone())
                .collect();

            let mut writes = HashSet::new();
            writes.insert(variable);

            (reads, writes)
        } else {
            let reads: HashSet<_> = ReadWriteSet::read_set(&group.assignments)
                .into_iter()
                .filter(|c| c.borrow().type_name() == Some(&"std_reg".into()))
                .map(|c| c.borrow().name.clone())
                .collect();

            // only consider write assignments where the guard is true
            let assignments = group
                .assignments
                .iter()
                .filter(|asgn| asgn.guard == ir::Guard::True)
                .cloned()
                .collect::<Vec<_>>();

            let writes = ReadWriteSet::write_set(&assignments)
                .into_iter()
                .filter(|c| c.borrow().type_name() == Some(&"std_reg".into()))
                .map(|c| c.borrow().name.clone())
                .collect();

            (reads, writes)
        }
    }
}

impl ToString for Data {
    fn to_string(&self) -> String {
        self.live
            .iter()
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
        mut data: Data,
        comp: &mut Component,
        sigs: &lib::LibrarySignatures,
    ) -> VisResult<Data> {
        // drain gens so that we don't mix them with the gens we gather from the par
        data.gen.drain();
        let mut par_data = data.clone();
        for child in par.stmts.iter_mut() {
            par_data =
                &par_data | &child.visit(self, data.clone(), comp, sigs)?.data;
        }

        // compute transfer function using
        //  - gen = union(gen(children))
        //  - kill = union(kill(children))
        par_data = par_data.transfer();

        Ok(Action::skipchildren_with(par_data))
    }

    fn start_if(
        &mut self,
        if_s: &mut ir::If,
        mut alive: Data,
        comp: &mut Component,
        sigs: &lib::LibrarySignatures,
    ) -> VisResult<Data> {
        // compute each branch
        let t_alive = if_s.tbranch.visit(self, alive.clone(), comp, sigs)?.data;
        let f_alive = if_s.fbranch.visit(self, alive.clone(), comp, sigs)?.data;

        // take union
        alive.live = &alive.live | &t_alive.live;
        alive.live = &alive.live | &f_alive.live;

        // feed to condition to compute
        let cond_alive =
            ir::Control::Enable(ir::Enable::from(if_s.cond.clone()))
                .visit(self, alive, comp, sigs)?
                .data;

        // return liveness from condition
        Ok(Action::skipchildren_with(cond_alive))
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
            eprintln!("  start: {}", start.to_string());
            next = while_s.body.visit(self, start.clone(), comp, sigs)?.data;
            next = ir::Control::Enable(ir::Enable::from(while_s.cond.clone()))
                .visit(self, next, comp, sigs)?
                .data;
            eprintln!("  next: {}", next.to_string());

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
        let name = enable.group.borrow().name.to_string();
        eprintln!(" {} in( {} )", name, alive.to_string());
        // no reason to compute this every time
        let (reads, writes) =
            LiveRangeAnalysis::find_gen_kill(Rc::clone(&enable.group));
        alive.gen = reads;
        alive.kill = writes;

        eprintln!("    gen: {:?}, kill: {:?}", alive.gen, alive.kill);

        // compute transfer function
        alive = alive.transfer();

        // set the live set of this node to be the things live on the output of this node
        self.live.insert(
            enable.group.borrow().name.clone(),
            &alive.live | &alive.kill,
        );

        eprintln!(" {} out( {} )", name, alive.to_string());

        Ok(Action::continue_with(alive))
    }
}
