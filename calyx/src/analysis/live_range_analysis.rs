use crate::{
    analysis::{ReadWriteSet, VariableDetection},
    ir::{self, RRC},
};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    ops::{BitOr, Sub},
    rc::Rc,
};

/// The data structure used to represent sets of ids. This is used to represent
/// the `live`, `gen`, and `kill` sets.
#[derive(Default, Clone)]
pub struct Prop {
    set: HashSet<ir::Id>,
}

/// Conversion from HashSet<ir::Id>
impl From<HashSet<ir::Id>> for Prop {
    fn from(set: HashSet<ir::Id>) -> Self {
        Prop { set }
    }
}

/// Implement convenience math operators for Prop
impl BitOr<&Prop> for &Prop {
    type Output = Prop;
    fn bitor(self, rhs: &Prop) -> Self::Output {
        Prop {
            set: &self.set | &rhs.set,
        }
    }
}

impl Sub<&Prop> for &Prop {
    type Output = Prop;
    fn sub(self, rhs: &Prop) -> Self::Output {
        Prop {
            set: &self.set - &rhs.set,
        }
    }
}

/// Implement nice printing for prop for debugging purposes.
impl Debug for Prop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        let names = self.set.iter().map(|id| &id.id).join(", ");
        write!(f, "{}", names)?;
        write!(f, "}}")
    }
}

impl Prop {
    /// Defines the dataflow transfer function.
    /// This is the standard definition for liveness.
    fn transfer(self, gen: &Prop, kill: &Prop) -> Self {
        &(&self - kill) | gen
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
    live: HashMap<ir::Id, Prop>,
}

impl Debug for LiveRangeAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Live variables {{")?;
        for (k, v) in self.live.iter() {
            writeln!(f, "  {}: {:?}", k.id, v)?;
        }
        write!(f, "}}")
    }
}

impl LiveRangeAnalysis {
    /// Construct a live range analysis.
    pub fn new(comp: &ir::Component, control: &ir::Control) -> Self {
        let mut ranges = LiveRangeAnalysis::default();

        build_live_ranges(
            control,
            Prop::default(),
            Prop::default(),
            Prop::default(),
            &mut ranges,
        );

        // add global reads to every point
        let global_reads: Prop =
            ReadWriteSet::read_set(&comp.continuous_assignments)
                .into_iter()
                .filter(|c| c.borrow().type_name() == Some(&"std_reg".into()))
                .map(|c| c.borrow().name.clone())
                .collect::<HashSet<_>>()
                .into();
        for (_, prop) in ranges.live.iter_mut() {
            *prop = &*prop | &global_reads;
        }

        ranges
    }
    /// Look up the set of things live at a group definition.
    pub fn get(&self, group: &ir::Id) -> &HashSet<ir::Id> {
        &self.live[&group].set
    }

    /// Get a unique list of all live registers in `component`.
    pub fn get_all(&self) -> impl Iterator<Item = ir::Id> + '_ {
        self.live
            .iter()
            .map(|(_name, set)| set.set.iter())
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
    fn find_gen_kill(group_ref: &RRC<ir::Group>) -> (Prop, Prop) {
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
                    if let ir::Guard::Port(port) = &*asgn.guard {
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

            (reads.into(), writes.into())
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
                .filter(|asgn| *asgn.guard == ir::Guard::True)
                .cloned()
                .collect::<Vec<_>>();

            let writes: HashSet<_> = ReadWriteSet::write_set(&assignments)
                .into_iter()
                .filter(|c| c.borrow().type_name() == Some(&"std_reg".into()))
                .map(|c| c.borrow().name.clone())
                .collect();

            (reads.into(), writes.into())
        }
    }
}

/// Implements the parallel dataflow analysis that computes the liveness of every register
/// at every point in the program.
fn build_live_ranges(
    c: &ir::Control,
    alive: Prop,
    gens: Prop,
    kills: Prop,
    lr: &mut LiveRangeAnalysis,
) -> (Prop, Prop, Prop) {
    match c {
        ir::Control::Empty(_) => (alive, gens, kills),
        ir::Control::Invoke(_) => unimplemented!(),
        ir::Control::Enable(ir::Enable { group, .. }) => {
            // XXX(sam) no reason to compute this every time
            let (reads, writes) = LiveRangeAnalysis::find_gen_kill(&group);

            // compute transfer function
            let alive = alive.transfer(&reads, &writes);

            // set the live set of this node to be the things live on the
            // output of this node plus the things written to in this group
            lr.live
                .insert(group.borrow().name.clone(), &alive | &writes);
            (alive, &gens | &reads, &kills | &writes)
        }
        ir::Control::Seq(ir::Seq { stmts, .. }) => stmts.iter().rev().fold(
            (alive, gens, kills),
            |(alive, gens, kills), e| {
                build_live_ranges(&e, alive, gens, kills, lr)
            },
        ),
        ir::Control::If(ir::If {
            cond,
            tbranch,
            fbranch,
            ..
        }) => {
            // compute each branch
            let (t_alive, t_gens, t_kills) = build_live_ranges(
                &tbranch,
                alive.clone(),
                gens.clone(),
                kills.clone(),
                lr,
            );
            let (f_alive, f_gens, f_kills) =
                build_live_ranges(&fbranch, alive, gens, kills, lr);

            // take union
            let alive = &t_alive | &f_alive;
            let gens = &t_gens | &f_gens;
            let kills = &t_kills | &f_kills;

            // feed to condition to compute
            build_live_ranges(
                &ir::Control::enable(cond.clone()),
                alive,
                gens,
                kills,
                lr,
            )
        }
        ir::Control::Par(ir::Par { stmts, .. }) => {
            let (alive, gens, kills) = stmts
                .iter()
                .rev()
                .map(|e| {
                    build_live_ranges(
                        e,
                        alive.clone(),
                        Prop::default(),
                        Prop::default(),
                        lr,
                    )
                })
                .fold(
                    (Prop::default(), Prop::default(), Prop::default()),
                    |(acc_alive, acc_gen, acc_kill), (alive, gen, kill)| {
                        (
                            &acc_alive | &alive,
                            &acc_gen | &gen,
                            &acc_kill | &kill,
                        )
                    },
                );

            let alive = alive.transfer(&gens, &kills);
            (alive, gens, kills)
        }
        ir::Control::While(ir::While { body, cond, .. }) => {
            let (alive, gens, kills) =
                build_live_ranges(&body, alive, gens, kills, lr);
            let (alive, gens, kills) = build_live_ranges(
                &ir::Control::enable(cond.clone()),
                alive,
                gens,
                kills,
                lr,
            );
            build_live_ranges(&body, alive, gens, kills, lr)
        }
    }
}
