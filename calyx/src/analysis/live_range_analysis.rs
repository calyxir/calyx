use crate::{
    analysis::{ReadWriteSet, VariableDetection},
    ir::{self, CloneName, RRC},
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

/// Conversion to Prop from things that can be converted to HashSet<ir::Id>.
impl<T: Into<HashSet<ir::Id>>> From<T> for Prop {
    fn from(t: T) -> Self {
        Prop { set: t.into() }
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
    /// We use the standard definition for liveness:
    ///   `(alive - kill) + gen`
    fn transfer(self, gen: &Prop, kill: &Prop) -> Self {
        &(&self - kill) | gen
    }
}

/// This analysis implements a parallel version of a classic liveness analysis.
/// For each group, it returns a list of the registers that are "alive" during
/// an execution of a group.
///
/// ## Parallel Analog to a CFG
/// The `par` statement introduces a new kind of control branching that can
/// not be captured by a traditional CFG.
///
/// Consider whether `x` is alive at `foo` in the following program.
/// ```
/// seq {
///   wr_x; // writes register x
///   foo;
///   par {
///     wr_x2; // writes register x
///     bar;
///   }
///   rd_x; // reads register x
/// }
/// ```
/// `x` is not alive at `foo` because there are no reads to `x` before
/// `wr_x2` is executed which writes to `x` again. Note that `wr_x2` is always
/// executed.
///
/// We might try and represent the `par` branching with a normal CFG like this:
/// ```
///       +------+
///       | wr_x |
///       +--+---+
///          |
///          v
///       +--+--+
///    +--+ foo +--+
///    |  +-----+  |
///    |           |
///    v           v
/// +--+----+   +--+--+
/// | wr_x2 |   | bar |
/// +--+----+   +--+--+
///    |           |
///    +------+----+
///           |
///           v
///       +------+
///       | rd_x |
///       +------+
/// ```
/// But then this program is identical to
/// ```
/// seq {
///   wr_x; // writes register x
///   foo;
///   if blah.out with B {
///     wr_x2; // writes register x
///   } else {
///     bar;
///   }
///   rd_x; // reads register x
/// }
/// ```
/// which has different semantics. In particular `x` is still alive at `foo` because
/// `wr_x2` may not be executed.
///
/// We need to augment the traditional CFG to account for `par`.
///
/// ## A Parallel CFG
/// The representation should:
///  1) Have the same properties as a normal CFG when no parallelism is present.
///  2) Threads of a `par` block should not have to know that they are in a `par` (i.e. are just CFGs themselves)
///  3) External to the `par` block, the information of running all threads in `par` should be visible.
///
/// To address these concerns, we use a parallel CFG (pCFG) based on
/// [Analyzing programs with explicit parallelism](https://link.springer.com/chapter/10.1007%2FBFb0038679).
/// We introduce a new kind of node in the CFG called a `par node`. A `par node` represents an entire
/// `par` block. The above program with `par` would look like:
/// ```
/// +------+
/// | wr_x |
/// +--+---+
///    |
///    v
/// +--+--+
/// | foo |
/// +--+--+
///    |
///    v
/// +--+---+
/// | par1 |
/// +--+---+
///    |
///    v
/// +--+---+
/// | rd_x |
/// +------+
/// ```
/// For each `par node`, we associate a list of pCFGs where each pCFG represents a thread.
/// Each thread starts with a `begin par` node and ends with a `end par` node.
///
/// These are the graphs associated with `par1`.
/// ```
/// First thread:    Second thread:
/// +----------+      +----------+
/// |begin par1|      |begin par1|
/// +--+-------+      +-+--------+
///    |                |
///    v                v
/// +--+--+           +-+-+
/// |wr_x2|           |bar|
/// +--+--+           +-+-+
///    |                |
///    v                v
/// +--+-----+        +-+------+
/// |end par1|        |end par1|
/// +--------+        +--------+
/// ```
///
/// The idea with the `begin/end parx` nodes is that these will handle the flow
/// of information in and out of the threads. For example, you could write these equations:
/// ```
/// out(begin par1) = in(par1)
/// out(par1) = join over all in(end par1)
/// ```
///
/// ## Definition of Liveness
/// Now we finally come to the definition of "liveness" and how we use the pCFG to compute this.
///
/// We say a register `x` is "live" at a node `p` in the CFG if there is a write to `x` ordered before
/// `p` (such that there are no more writes to `x` at a point between that and `p`) and if there is a read
/// of `x` ordered after `p` (such that there are no writes between that and `p`).
///
/// We define the following equations (assuming a reversed direction dataflow analysis):
/// ```
/// for some node n:
///   gen(n) = registers that may be read in n
///   kill(n) = register that must be written to in n
///   live_in(n) = union over live_out(pred(n))
///   live_out(n) = (live_in(n) - kill(n)) + gen(n)
/// for some par node p:
///   gen(p) = union over gen(n) for sub-nodes n in p
///   kill(p) = union over kill(n) for sub-nodes n in p
///   live_in(p) = union over live_out(pred(p))
///   live_out(p) = (live_in(p) - kill(p)) + gen(p)
/// ```
/// The main place this analysis differs from traditional liveness analysis
/// is the definition of `gen(p)` and `kill(p)` for `par` nodes. These are the
/// union of the `gen`s and `kill`s of all of their sub-nodes. Intuitively we
/// are treating `par` blocks as if they were just a single group. Note that this
/// is overly conservative because we are potentially ignoring ordering
/// information of the threads.
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
                .map(|c| c.clone_name())
                .collect::<HashSet<_>>()
                .into();
        for (_, prop) in ranges.live.iter_mut() {
            *prop = &*prop | &global_reads;
        }

        ranges
    }
    /// Look up the set of things live at a group definition.
    pub fn get(&self, group: &ir::Id) -> &HashSet<ir::Id> {
        &self.live[group].set
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
    fn find_gen_kill_group(group_ref: &RRC<ir::Group>) -> (Prop, Prop) {
        let group = group_ref.borrow();
        // if the group contains what looks like a variable write,
        // then just add variable to write set
        if let Some(variable) =
            VariableDetection::variable_like(Rc::clone(group_ref))
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
                .map(|c| c.clone_name())
                .collect();

            let mut writes = HashSet::new();
            writes.insert(variable);

            (reads.into(), writes.into())
        } else {
            let reads: HashSet<_> = ReadWriteSet::read_set(&group.assignments)
                .into_iter()
                .filter(|c| c.borrow().type_name() == Some(&"std_reg".into()))
                .map(|c| c.clone_name())
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
                .map(|c| c.clone_name())
                .collect();

            (reads.into(), writes.into())
        }
    }

    fn find_gen_kill_invoke(invoke: &ir::Invoke) -> (Prop, Prop) {
        let register_filter = |port: &RRC<ir::Port>| {
            if let ir::PortParent::Cell(cell_wref) = &port.borrow().parent {
                cell_wref.upgrade().borrow().type_name()
                    == Some(&"std_reg".into())
            } else {
                false
            }
        };

        let reads: Prop = invoke
            .inputs
            .iter()
            .filter(|(_, src)| register_filter(src))
            .map(|(_, src)| src.borrow().get_parent_name())
            .collect::<HashSet<ir::Id>>()
            .into();

        let writes: Prop = invoke
            .outputs
            .iter()
            .filter(|(_, src)| register_filter(src))
            .map(|(_, dest)| dest.borrow().get_parent_name())
            .collect::<HashSet<ir::Id>>()
            .into();

        (reads, writes)
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
        ir::Control::Invoke(invoke) => {
            let (reads, writes) =
                LiveRangeAnalysis::find_gen_kill_invoke(invoke);
            let alive = alive.transfer(&reads, &writes);
            (alive, &gens | &reads, &kills | &writes)
        }
        ir::Control::Enable(ir::Enable { group, .. }) => {
            // XXX(sam) no reason to compute this every time
            let (reads, writes) = LiveRangeAnalysis::find_gen_kill_group(group);

            // compute transfer function
            let alive = alive.transfer(&reads, &writes);

            // set the live set of this node to be the things live on the
            // output of this node plus the things written to in this group
            lr.live.insert(group.clone_name(), &alive | &writes);
            (alive, &gens | &reads, &kills | &writes)
        }
        ir::Control::Seq(ir::Seq { stmts, .. }) => stmts.iter().rev().fold(
            (alive, gens, kills),
            |(alive, gens, kills), e| {
                build_live_ranges(e, alive, gens, kills, lr)
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
                tbranch,
                alive.clone(),
                gens.clone(),
                kills.clone(),
                lr,
            );
            let (f_alive, f_gens, f_kills) =
                build_live_ranges(fbranch, alive, gens, kills, lr);

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
                build_live_ranges(body, alive, gens, kills, lr);
            let (alive, gens, kills) = build_live_ranges(
                &ir::Control::enable(cond.clone()),
                alive,
                gens,
                kills,
                lr,
            );
            build_live_ranges(body, alive, gens, kills, lr)
        }
    }
}
