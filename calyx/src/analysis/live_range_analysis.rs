use crate::{
    analysis::{ReadWriteSet, VariableDetection},
    ir::{self, RRC},
};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    ops::BitOr,
    rc::Rc,
};

/// The data structure that is passed through the visitor functions.
/// We need to explicitly pass `gen` and `live` between control statements because
/// `par` needs this information to implement it's `meet` function correctly.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct Data {
    /// Represents the registers that are generated from this control statement.
    gen: HashSet<ir::Id>,
    /// Represents the registers that are killed by this control statement.
    kill: HashSet<ir::Id>,
    /// Represents the registers that are live at this control statement.
    live: HashSet<ir::Id>,
    /// Keeps track of registers alive in par statements so that they can
    /// be shared between siblings.
    local_live: HashSet<ir::Id>,
}

impl BitOr<&Data> for &Data {
    type Output = Data;
    fn bitor(self, rhs: &Data) -> Self::Output {
        Data {
            gen: &self.gen | &rhs.gen,
            kill: &self.kill | &rhs.kill,
            live: &self.live | &rhs.live,
            local_live: &self.local_live | &rhs.local_live,
        }
    }
}

impl Data {
    /// Defines the dataflow transfer function.
    /// This is the standard definition for liveness.
    fn transfer(&mut self) {
        self.live = &(&self.live - &self.kill) | &self.gen;
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
    /// Map from group names to the components live inside them.
    live: HashMap<ir::Id, HashSet<ir::Id>>,
}

impl LiveRangeAnalysis {
    /// Look up the set of things live at a group definition.
    pub fn get(&self, group: &ir::Group) -> &HashSet<ir::Id> {
        &self.live[&group.name]
    }

    /// Get a unique list of all live registers in `component`.
    pub fn get_all(&self) -> impl Iterator<Item = ir::Id> + '_ {
        self.live
            .iter()
            .map(|(_name, set)| set.iter())
            .flatten()
            .unique()
            .cloned()
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
        group_ref: &RRC<ir::Group>,
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
                .filter(|asgn| {
                    if let ir::Guard::Port(port) = &asgn.guard {
                        !(port.borrow().get_parent_name() == variable
                            && port.borrow().name == "done")
                    } else {
                        true
                    }
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

fn build_live_ranges(
    c: &ir::Control,
    alive: &mut Data,
    lr: &mut LiveRangeAnalysis,
) {
    match c {
        ir::Control::Empty(_) => (),
        ir::Control::Invoke(_) => unimplemented!(),
        ir::Control::Enable(ir::Enable { group }) => {
            // XXX(sam) no reason to compute this every time
            let (reads, writes) = LiveRangeAnalysis::find_gen_kill(&group);
            alive.gen = reads;
            alive.kill = writes;

            // compute transfer function
            alive.transfer();

            // add things live out of this enable to the local lives.
            alive.local_live = &alive.local_live | &alive.live;

            // set the live set of this node to be the things live on the
            // output of this node plus the things written to in this group
            lr.live.insert(
                group.borrow().name.clone(),
                &(&alive.live | &alive.kill) | &alive.local_live,
            );
        }
        ir::Control::Seq(ir::Seq { stmts }) => stmts
            .iter()
            .rev()
            .for_each(|c| build_live_ranges(&c, alive, lr)),
        ir::Control::If(ir::If {
            cond,
            tbranch,
            fbranch,
            ..
        }) => {
            // compute each branch
            let mut t_alive = alive.clone();
            let mut f_alive = alive.clone();
            build_live_ranges(&tbranch, &mut t_alive, lr);
            build_live_ranges(&fbranch, &mut f_alive, lr);

            // take union
            alive.live = &alive.live | &t_alive.live;
            alive.live = &alive.live | &f_alive.live;

            // feed to condition to compute
            build_live_ranges(&ir::Control::enable(cond.clone()), alive, lr)
        }
        ir::Control::Par(ir::Par { stmts }) => {
            // drain gens so that we don't mix them with the gens we gather from the par
            alive.gen.drain();
            // record the things locally live coming into this par.
            // we first visit our children without the local lives
            // to gather the local lives they generate. we then pass
            // the union of the local lives to the children as we visit
            // them again. this has the effect of communicating live registers
            // between siblings in a par.
            let saved = alive.local_live.drain().collect::<HashSet<_>>();
            for child in stmts {
                let mut child_data = alive.clone();
                build_live_ranges(&child, &mut child_data, lr);
                *alive = &*alive | &child_data;
            }

            // compute transfer function using
            //  - gen = union(gen(children))
            //  - kill = union(kill(children))
            alive.transfer();

            // we remove registers that we killed from the local live and recombine
            // it with the saved local lives so that an pars above this one have
            // the correct local lives.
            alive.local_live = &(&alive.local_live - &alive.kill) | &saved;
        }
        ir::Control::While(ir::While { body, cond, .. }) => {
            let mut next;

            loop {
                next = alive.clone();
                build_live_ranges(&body, &mut next, lr);
                build_live_ranges(
                    &ir::Control::enable(cond.clone()),
                    &mut next,
                    lr,
                );

                if *alive == next {
                    *alive = next;
                    break;
                }
                *alive = next;
            }
        }
    }
}

impl From<&ir::Control> for LiveRangeAnalysis {
    fn from(control: &ir::Control) -> Self {
        let mut ranges = LiveRangeAnalysis::default();
        build_live_ranges(control, &mut Data::default(), &mut ranges);
        ranges
    }
}
