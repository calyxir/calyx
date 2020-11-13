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
    par_kill: HashSet<ir::Id>,
    live: HashSet<ir::Id>,
}

impl BitOr<&Data> for &Data {
    type Output = Data;
    fn bitor(self, rhs: &Data) -> Self::Output {
        Data {
            gen: &self.gen | &rhs.gen,
            kill: &self.kill | &rhs.kill,
            par_kill: &self.par_kill | &rhs.kill,
            live: &self.live | &rhs.live,
        }
    }
}

impl Data {
    fn transfer(mut self) -> Self {
        self.live = &(&(&self.live - &self.kill) - &self.par_kill) | &self.gen;
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

    fn find_gen_kill(
        group_ref: RRC<ir::Group>,
    ) -> (HashSet<ir::Id>, HashSet<ir::Id>) {
        let group = group_ref.borrow();
        // if the group contains what looks like a variable write,
        // then just add variable to write set
        if let Some(variable) =
            VariableDetection::variable_like(Rc::clone(&group_ref))
        {
            eprintln!(" variable! {:?}", variable.to_string());
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

            // XXX(sam): should also consider other writes
            let mut writes = HashSet::new();
            writes.insert(variable);

            (reads, writes)
        } else {
            let reads: HashSet<_> = ReadWriteSet::read_set(&group.assignments)
                .into_iter()
                .filter(|c| c.borrow().type_name() == Some(&"std_reg".into()))
                .map(|c| c.borrow().name.clone())
                .collect();

            // XXX(sam) the writes are incorrect atm
            let writes = ReadWriteSet::write_set(&group.assignments)
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
        eprintln!("start seq");
        // iterate over the seq children in reverse order
        for child in seq.stmts.iter_mut().rev() {
            alive = child.visit(self, alive, comp, sigs)?.data;
        }
        eprintln!("finish seq");

        Ok(Action::skipchildren_with(alive))
    }

    fn start_par(
        &mut self,
        par: &mut ir::Par,
        data: Data,
        comp: &mut Component,
        sigs: &lib::LibrarySignatures,
    ) -> VisResult<Data> {
        // // x dead
        // par { w x, r y}
        // r x
        //
        //
        //    { y, z }
        //   /     \
        // { z }  { x, y, z }
        //   ^      ^
        // { x, z }  { x, z }
        //
        //
        // { x } - ??? = { }
        // { x } + { } = ???
        //
        //
        //
        // seq {
        //   ... // y alive
        //   par {
        //     wr x, // x alive
        //     rd y  // y alive, is x alive here?
        //   }
        //   rd x // x alive
        // }
        //
        //
        //
        // # normal
        // live(n) = (live_in(n) - kill(n)) + gen(n)
        //
        // # parallel
        // we define cobegin(n) to be the parent `par` node of a ctrl stmt
        // par_live(live_in(n) - kill(n) - kill(cobegin(n))) + gen(n)

        let mut par_data = data.clone();
        for child in par.stmts.iter_mut() {
            let child_data = &child.visit(self, data.clone(), comp, sigs)?.data;
            par_data.par_kill = &par_data.par_kill | &child_data.kill;
        }

        let mut res = Data::default();
        res.par_kill = par_data.par_kill.clone();
        for child in par.stmts.iter_mut() {
            let child_data =
                &child.visit(self, par_data.clone(), comp, sigs)?.data;
            res = &res | &child_data;
        }
        res.par_kill.drain();

        Ok(Action::skipchildren_with(res))
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

        eprintln!(
            "    gen: {:?}, kill: {:?}, par_kill: {:?}",
            alive.gen, alive.kill, alive.par_kill
        );

        // compute transfer function
        alive = alive.transfer();

        // set the live set of this node to be the things live on the output of this node
        self.live
            .entry(enable.group.borrow().name.clone())
            .and_modify(|hs| *hs = &alive.live | &alive.kill)
            .or_insert(alive.live.clone());

        eprintln!(" {} out( {} )", name, alive.to_string());

        Ok(Action::continue_with(alive))
    }
}
