use crate::{
    analysis::{ControlId, ReadWriteSet, ShareSet, VariableDetection},
    ir::{self, CloneName, Id, RRC},
};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    rc::Rc,
};

/// The data structure used to represent sets of ids. This is used to represent
/// the `live`, `gen`, and `kill` sets.
#[derive(Default, Clone)]
pub struct Prop {
    map: CellsByType,
}

/// Implement nice printing for prop for debugging purposes.
impl Debug for Prop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        let names = self.map.iter().flat_map(|(_, ids)| ids).join(", ");
        write!(f, "{}", names)?;
        write!(f, "}}")
    }
}

impl Prop {
    /// Defines the dataflow transfer function.
    /// We use the standard definition for liveness:
    /// `(alive - kill) + gen`
    fn transfer(&mut self, gen: Prop, kill: Prop) {
        self.sub(kill);
        self.or(gen);
    }

    /// Defines the data_flow transfer function. `(alive - kill) + gen`.
    /// However, this is for when gen and kill are sets, and self is a map.
    fn transfer_set(&mut self, gen: TypeNameSet, kill: TypeNameSet) {
        self.sub_set(kill);
        self.or_set(gen);
    }

    // The or operation, but when the self is a map and rhs is a set of tuples.
    fn or_set(&mut self, rhs: TypeNameSet) {
        for (cell_type, cell_name) in rhs {
            self.map.entry(cell_type).or_default().insert(cell_name);
        }
    }

    // The sub operation, but when the self is a map and rhs is a set of tuples.
    fn sub_set(&mut self, rhs: TypeNameSet) {
        for (cell_type, cell_name) in rhs {
            self.map.entry(cell_type).or_default().remove(&cell_name);
        }
    }

    // edits self to equal self | rhs. Faster than self | rhs  but must take rhs
    // ownership and not &rhs.
    fn or(&mut self, rhs: Prop) {
        for (cell_type, cell_names) in rhs.map {
            self.map.entry(cell_type).or_default().extend(cell_names);
        }
    }

    // edits self to equal self intersect rhs. Must take ownership of rhs
    // ownership and not &rhs.
    fn intersect(&mut self, mut rhs: Prop) {
        for (cell_type, cell_names) in self.map.iter_mut() {
            let empty_hash = HashSet::new();
            let entry: HashSet<Id> =
                rhs.map.remove(cell_type).unwrap_or(empty_hash);
            cell_names.retain(|cell| entry.contains(cell));
        }
    }

    // edits self to equal self - rhs. Faster than self - rhs  but must take rhs
    // ownership and not &rhs.
    fn sub(&mut self, rhs: Prop) {
        for (cell_type, cell_names) in rhs.map {
            self.map
                .entry(cell_type)
                .or_default()
                .retain(|cell| !cell_names.contains(cell));
        }
    }
}

/// This analysis implements a parallel version of a classic liveness analysis.
/// For each group or invoke, it returns a list of the state shareable cells
/// that are "alive" during an execution of a group or invoke statement (we
/// identify an invoke statement by the cell that is being invoked, and groups
/// by the name of the group).
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
/// We say a cell `x` is "live" at a node `p` in the CFG if there is a write to `x` ordered before
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
    /// Map from node ids (i.e., group enables or invokes) names
    /// to the components live inside them.
    live: HashMap<u64, Prop>,
    /// Groups that have been identified as variable-like.
    /// Mapping from group name to Some(type, name) where type is the cell type and
    /// name is the cell name. If group is not variable like, maps to None.
    variable_like: HashMap<ir::Id, Option<(ir::CellType, ir::Id)>>,
    /// Set of state shareable components (as type names)
    state_share: ShareSet,
    /// Set of shareable components (as type names)
    share: ShareSet,
    /// maps invokes/enable ids to the shareable cell types/names live in them
    invokes_enables_map: HashMap<u64, TypeNameSet>,
    /// maps comb groups of if/while statements to the cell types/
    /// names used in them
    cgroup_uses_map: HashMap<u64, TypeNameSet>,
}

impl LiveRangeAnalysis {
    /// Construct a live range analysis.
    pub fn new(
        control: &mut ir::Control,
        state_share: ShareSet,
        share: ShareSet,
    ) -> Self {
        let mut ranges = LiveRangeAnalysis {
            state_share,
            share,
            ..Default::default()
        };

        // Give each control statement a unique "NODE_ID" attribute.
        ControlId::compute_unique_ids(control, 0, false);

        ranges.build_live_ranges(
            control,
            Prop::default(),
            Prop::default(),
            Prop::default(),
        );

        for (node, cells_by_type) in &ranges.invokes_enables_map {
            if let Some(prop) = ranges.live.get_mut(node) {
                prop.or_set(cells_by_type.clone());
            }
        }

        // LivRangeAnalysis does not handle comb groups currently. Eventually, we want to and make
        // remove-comb-groups optional.

        ranges
    }
}
