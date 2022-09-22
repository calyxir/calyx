use crate::{
    analysis::{ControlId, ReadWriteSet, ShareSet, VariableDetection},
    ir::{self, CloneName, RRC},
};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

type TypeNameSet = HashSet<(ir::CellType, ir::Id)>;
type CellsByType = HashMap<ir::CellType, HashSet<ir::Id>>;
// maps cell type to maps that map cell name to control statement
type LiveMapByType = HashMap<ir::CellType, HashMap<ir::Id, HashSet<u64>>>;

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

    // edits self to equal self | rhs. Faster than self | rhs  but must take rhs
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

impl Debug for LiveRangeAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Live variables {{")?;
        for (k, v) in self.live.iter() {
            writeln!(f, "  {}: {:?}", k, v)?;
        }
        write!(f, "}}")
    }
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

    /// Updates live_once_map and par_thread_map.
    /// live_once_map should map celltypes to a map, which should map cells of
    /// celltype to control statements in which it is live for at least one group
    /// or invoke in the control. We only map to control statements that are
    /// direct children of par blocks.
    /// parents is the list of current control statements (that are direct children
    /// of par blocks) that are parents (at any level of neesting) of c.
    pub fn get_live_control_data(
        &self,
        live_once_map: &mut LiveMapByType,
        par_thread_map: &mut HashMap<u64, u64>,
        live_cell_map: &mut LiveMapByType,
        parents: &HashSet<u64>,
        c: &ir::Control,
    ) {
        match c {
            ir::Control::Empty(_) => (),
            ir::Control::Par(ir::Par { stmts, .. }) => {
                let parent_id = ControlId::get_guaranteed_id(c);
                let mut new_parents = parents.clone();
                for stmt in stmts {
                    // building par_thread_map
                    let child_id = ControlId::get_guaranteed_id(stmt);
                    par_thread_map.insert(child_id, parent_id);

                    // building live_once_map by adding child_id to parents when
                    // we recursively call get_live_control_data on each child
                    new_parents.insert(child_id);
                    self.get_live_control_data(
                        live_once_map,
                        par_thread_map,
                        live_cell_map,
                        &new_parents,
                        stmt,
                    );
                    new_parents.remove(&child_id);
                }
            }
            ir::Control::Seq(ir::Seq { stmts, .. }) => {
                for stmt in stmts {
                    self.get_live_control_data(
                        live_once_map,
                        par_thread_map,
                        live_cell_map,
                        parents,
                        stmt,
                    );
                }
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                self.get_live_control_data(
                    live_once_map,
                    par_thread_map,
                    live_cell_map,
                    parents,
                    tbranch,
                );
                self.get_live_control_data(
                    live_once_map,
                    par_thread_map,
                    live_cell_map,
                    parents,
                    fbranch,
                );
                let id = ControlId::get_guaranteed_id(c);
                // Examining all the cells used at the comb group of the if stmt
                if let Some(comb_group_uses) = self.cgroup_uses_map.get(&id) {
                    for (cell_type, cell_name) in comb_group_uses {
                        // add cells as live within whichever direct children of
                        // par blocks they're located within
                        if !parents.is_empty() {
                            live_once_map
                                .entry(cell_type.clone())
                                .or_default()
                                .entry(cell_name.clone())
                                .or_default()
                                .extend(parents);
                        }
                        // mark cell as live in the control id of the if statement.
                        // What this really means, though, is that the cell is live
                        // at the comb group/port guard of the if statement
                        live_cell_map
                            .entry(cell_type.clone())
                            .or_default()
                            .entry(cell_name.clone())
                            .or_default()
                            .insert(id);
                    }
                }
            }
            ir::Control::While(ir::While { body, .. }) => {
                self.get_live_control_data(
                    live_once_map,
                    par_thread_map,
                    live_cell_map,
                    parents,
                    body,
                );
                let id = ControlId::get_guaranteed_id(c);
                if let Some(comb_group_uses) = self.cgroup_uses_map.get(&id) {
                    for (cell_type, cell_name) in comb_group_uses {
                        if !parents.is_empty() {
                            live_once_map
                                .entry(cell_type.clone())
                                .or_default()
                                .entry(cell_name.clone())
                                .or_default()
                                .extend(parents);
                        }
                        live_cell_map
                            .entry(cell_type.clone())
                            .or_default()
                            .entry(cell_name.clone())
                            .or_default()
                            .insert(id);
                    }
                }
            }
            ir::Control::Enable(_) | ir::Control::Invoke(_) => {
                let id = ControlId::get_guaranteed_id(c);
                let live_set = self.live.get(&id).unwrap().map.clone();
                for (cell_type, live_cells) in live_set {
                    let cell_to_node =
                        live_cell_map.entry(cell_type.clone()).or_default();
                    let cell_to_control =
                        live_once_map.entry(cell_type).or_default();
                    for cell in live_cells {
                        cell_to_node
                            .entry(cell.clone())
                            .or_default()
                            .insert(id);
                        cell_to_control
                            .entry(cell)
                            .or_default()
                            .extend(parents);
                    }
                }
            }
        }
    }

    /// Look up the set of things live at a node (i.e. group or invoke) definition.
    pub fn get(&self, node: &u64) -> &CellsByType {
        &self
            .live
            .get(node)
            .unwrap_or_else(|| panic!("Live set missing for {}", node))
            .map
    }

    /// Get a unique list of all live cells in `component`.
    pub fn get_all(&self) -> impl Iterator<Item = ir::Id> + '_ {
        self.live
            .iter()
            .flat_map(|(_, set)| {
                set.map.iter().fold(HashSet::new(), |mut acc, (_, set)| {
                    acc.extend(set);
                    acc
                })
            })
            .unique()
            .cloned()
    }

    fn variable_like(
        &mut self,
        grp: &RRC<ir::Group>,
    ) -> &Option<(ir::CellType, ir::Id)> {
        let group = grp.borrow();
        let name = group.name();
        if !self.variable_like.contains_key(name) {
            let res = VariableDetection::variable_like(grp, &self.state_share);
            self.variable_like.insert(grp.clone_name(), res);
        }
        &self.variable_like[name]
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
    fn find_gen_kill_group(
        &mut self,
        group_ref: &RRC<ir::Group>,
    ) -> (TypeNameSet, TypeNameSet) {
        let group = group_ref.borrow();
        let maybe_var = self.variable_like(group_ref).clone();
        let sc_clone = &self.state_share;
        // if the group contains what looks like a variable write,
        // then just add variable to write set
        if let Some((cell_type, variable)) = maybe_var {
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
                });

            // calculate reads, but ignore `variable`. we've already dealt with that
            let reads: HashSet<_> = ReadWriteSet::read_set(assignments)
                .filter(|c| sc_clone.is_shareable_component(c))
                .map(|c| (c.borrow().prototype.clone(), c.clone_name()))
                .collect();

            let mut writes = HashSet::new();
            writes.insert((cell_type, variable));

            (reads, writes)
        } else {
            let reads: HashSet<_> =
                ReadWriteSet::read_set(group.assignments.iter())
                    .filter(|c| sc_clone.is_shareable_component(c))
                    .map(|c| (c.borrow().prototype.clone(), c.clone_name()))
                    .collect();

            // only consider write assignments where the guard is true
            let assignments = group
                .assignments
                .iter()
                .filter(|asgn| *asgn.guard == ir::Guard::True)
                .cloned()
                .collect::<Vec<_>>();

            let writes: HashSet<_> =
                ReadWriteSet::write_set(assignments.iter())
                    .filter(|c| sc_clone.is_shareable_component(c))
                    .map(|c| (c.borrow().prototype.clone(), c.clone_name()))
                    .collect();

            (reads, writes)
        }
    }

    fn find_uses_group(
        group_ref: &RRC<ir::Group>,
        shareable_components: &ShareSet,
    ) -> TypeNameSet {
        let group = group_ref.borrow();
        ReadWriteSet::uses(group.assignments.iter())
            .filter(|cell| shareable_components.is_shareable_component(cell))
            .map(|cell| (cell.borrow().prototype.clone(), cell.clone_name()))
            .collect::<HashSet<_>>()
    }

    // returns (share_uses, state_reads), which are the uses of shareable components
    // and reads of state shareable components
    fn uses_reads_cgroup(
        group_ref: &RRC<ir::CombGroup>,
        shareable: &ShareSet,
        state_shareable: &ShareSet,
    ) -> (TypeNameSet, TypeNameSet) {
        let group = group_ref.borrow();
        let share_uses = ReadWriteSet::uses(group.assignments.iter())
            .filter(|cell| shareable.is_shareable_component(cell))
            .map(|cell| (cell.borrow().prototype.clone(), cell.clone_name()))
            .collect::<HashSet<_>>();
        let state_reads = ReadWriteSet::read_set(group.assignments.iter())
            .filter(|cell| state_shareable.is_shareable_component(cell))
            .map(|cell| (cell.borrow().prototype.clone(), cell.clone_name()))
            .collect::<HashSet<_>>();
        (share_uses, state_reads)
    }

    fn port_to_cell_name(
        port: &RRC<ir::Port>,
        shareable_components: &ShareSet,
    ) -> Option<(ir::CellType, ir::Id)> {
        if let ir::PortParent::Cell(cell_wref) = &port.borrow().parent {
            let cell = cell_wref.upgrade();
            if shareable_components.is_shareable_component(&cell) {
                return Some((
                    cell.borrow().prototype.clone(),
                    cell.borrow().clone_name(),
                ));
            }
        }
        None
    }

    /// Returns (reads, writes) that occur in the [ir::Invoke] statement.
    fn find_gen_kill_invoke(
        invoke: &ir::Invoke,
        shareable_components: &ShareSet,
    ) -> (TypeNameSet, TypeNameSet) {
        //The reads of the invoke include its inputs plus the cell itself, if the
        //outputs are not empty.
        let mut read_set: TypeNameSet = invoke
            .inputs
            .iter()
            .filter_map(|(_, src)| {
                Self::port_to_cell_name(src, shareable_components)
            })
            .collect();
        if !invoke.outputs.is_empty()
            && shareable_components.is_shareable_component(&invoke.comp)
        {
            read_set.insert((
                invoke.comp.borrow().prototype.clone(),
                invoke.comp.borrow().clone_name(),
            ));
        }

        if let Some(comb_group) = &invoke.comb_group {
            read_set.extend(
                ReadWriteSet::read_set(comb_group.borrow().assignments.iter())
                    .filter(|cell| {
                        shareable_components.is_shareable_component(cell)
                    })
                    .map(|cell| {
                        (cell.borrow().prototype.clone(), cell.clone_name())
                    }),
            );
        }

        //The writes of the invoke include its outpus plus the cell itself, if the
        //inputs are not empty.
        let mut write_set: TypeNameSet = invoke
            .outputs
            .iter()
            .filter_map(|(_, src)| {
                Self::port_to_cell_name(src, shareable_components)
            })
            .collect();
        if !invoke.inputs.is_empty()
            && shareable_components.is_shareable_component(&invoke.comp)
        {
            write_set.insert((
                invoke.comp.borrow().prototype.clone(),
                invoke.comp.borrow().clone_name(),
            ));
        }

        (read_set, write_set)
    }

    fn find_uses_invoke(
        invoke: &ir::Invoke,
        shareable_components: &ShareSet,
    ) -> TypeNameSet {
        // uses of shareable components in the invoke statement
        let mut invoke_uses: TypeNameSet = invoke
            .inputs
            .iter()
            .chain(invoke.outputs.iter())
            .filter_map(|(_, src)| {
                Self::port_to_cell_name(src, shareable_components)
            })
            .collect();
        // uses of shareable components in the comb group (if it exists)
        if let Some(comb_group) = &invoke.comb_group {
            invoke_uses.extend(
                ReadWriteSet::uses(comb_group.borrow().assignments.iter())
                    .filter(|cell| {
                        shareable_components.is_shareable_component(cell)
                    })
                    .map(|cell| {
                        (cell.borrow().prototype.clone(), cell.clone_name())
                    }),
            );
        }
        invoke_uses
    }

    /// Implements the parallel dataflow analysis that computes the liveness of every state shareable component
    /// at every point in the program.
    fn build_live_ranges(
        &mut self,
        c: &ir::Control,
        mut alive: Prop,
        mut gens: Prop,
        mut kills: Prop,
    ) -> (Prop, Prop, Prop) {
        match c {
            ir::Control::Empty(_) => (alive, gens, kills),
            ir::Control::Invoke(invoke) => {
                //get the shareable components used in the invoke stmt
                let uses_share =
                    LiveRangeAnalysis::find_uses_invoke(invoke, &self.share);
                self.invokes_enables_map
                    .entry(ControlId::get_guaranteed_id(c))
                    .or_default()
                    .extend(uses_share);

                let (reads, writes) = LiveRangeAnalysis::find_gen_kill_invoke(
                    invoke,
                    &self.state_share,
                );

                alive.transfer_set(reads.clone(), writes.clone());
                let alive_out = alive.clone();

                // set the live set of this node to be the things live on the
                // output of this node plus the things written to in this invoke
                // plus all shareable components used
                self.live.insert(ControlId::get_guaranteed_id(c), {
                    alive.or_set(writes.clone());
                    alive
                });
                (
                    alive_out,
                    {
                        gens.or_set(reads);
                        gens
                    },
                    {
                        kills.or_set(writes);
                        kills
                    },
                )
            }
            ir::Control::Enable(ir::Enable { group, .. }) => {
                let uses_share =
                    LiveRangeAnalysis::find_uses_group(group, &self.share);
                self.invokes_enables_map
                    .entry(ControlId::get_guaranteed_id(c))
                    .or_default()
                    .extend(uses_share);
                // XXX(sam) no reason to compute this every time
                let (reads, writes) = self.find_gen_kill_group(group);

                // compute transfer function
                alive.transfer_set(reads.clone(), writes.clone());
                let alive_out = alive.clone();

                // set the live set of this node to be the things live on the
                // output of this node plus the things written to in this group
                self.live.insert(ControlId::get_guaranteed_id(c), {
                    alive.or_set(writes.clone());
                    alive
                });
                (
                    alive_out,
                    {
                        gens.or_set(reads);
                        gens
                    },
                    {
                        kills.or_set(writes);
                        kills
                    },
                )
            }
            ir::Control::Seq(ir::Seq { stmts, .. }) => stmts.iter().rev().fold(
                (alive, gens, kills),
                |(alive, gens, kills), e| {
                    self.build_live_ranges(e, alive, gens, kills)
                },
            ),
            ir::Control::If(ir::If {
                tbranch,
                fbranch,
                port,
                cond,
                ..
            }) => {
                // compute each branch
                let (mut t_alive, mut t_gens, mut t_kills) = self
                    .build_live_ranges(
                        tbranch,
                        alive.clone(),
                        gens.clone(),
                        kills.clone(),
                    );
                let (f_alive, f_gens, f_kills) =
                    self.build_live_ranges(fbranch, alive, gens, kills);

                // take union
                t_alive.or(f_alive);
                t_gens.or(f_gens);
                t_kills.or(f_kills);

                let id = ControlId::get_guaranteed_id(c);

                // reads from state shareable components in the comb group
                // These should get "passed on" as live/gens as we go up the
                // control flow of the program
                let mut cgroup_reads: TypeNameSet = HashSet::new();
                // Any uses of any shareable components in the comb group.
                let mut shareable_uses: TypeNameSet = HashSet::new();

                if let Some(comb_group) = cond {
                    let (share_uses, state_reads) = Self::uses_reads_cgroup(
                        comb_group,
                        &self.share,
                        &self.state_share,
                    );
                    shareable_uses = share_uses;
                    cgroup_reads = state_reads;
                }

                if let Some(cell_info) = LiveRangeAnalysis::port_to_cell_name(
                    port,
                    &self.state_share,
                ) {
                    // If we read from a state shareable component (like a register)
                    // in the port, then we add it to cgroup_reads.
                    cgroup_reads.insert(cell_info);
                }
                if !cgroup_reads.is_empty() || !shareable_uses.is_empty() {
                    let mut all_uses = cgroup_reads.clone();
                    all_uses.extend(shareable_uses);
                    // add all uses of both shareable and state-shareable components
                    // in the cgroup_uses_map.
                    self.cgroup_uses_map.insert(id, all_uses);
                }
                // adding cgroup_reads as live on output of if stmt
                t_alive.or_set(cgroup_reads.clone());
                t_gens.or_set(cgroup_reads);
                (t_alive, t_gens, t_kills)
            }
            ir::Control::Par(ir::Par { stmts, .. }) => {
                let (mut alive, gens, kills) = stmts
                    .iter()
                    .rev()
                    .map(|e| {
                        self.build_live_ranges(
                            e,
                            alive.clone(),
                            Prop::default(),
                            Prop::default(),
                        )
                    })
                    .fold(
                        (Prop::default(), Prop::default(), Prop::default()),
                        |(mut acc_alive, mut acc_gen, mut acc_kill),
                         (alive, gen, kill)| {
                            (
                                // Doing in place operations saves time
                                {
                                    acc_alive.or(alive);
                                    acc_alive
                                },
                                {
                                    acc_gen.or(gen);
                                    acc_gen
                                },
                                {
                                    acc_kill.or(kill);
                                    acc_kill
                                },
                            )
                        },
                    );
                alive.transfer(gens.clone(), kills.clone());
                (alive, gens, kills)
            }
            ir::Control::While(ir::While {
                body, port, cond, ..
            }) => {
                let id = ControlId::get_guaranteed_id(c);
                // need this info twice, so just pre-calculate whether port is
                // a state shareable component.
                let port_if_shareable: Option<(ir::CellType, ir::Id)> =
                    LiveRangeAnalysis::port_to_cell_name(
                        port,
                        &self.state_share,
                    );
                // all reads from state shareable components in the comb group or port
                let mut cgroup_reads: TypeNameSet = HashSet::new();
                // all uses of shareable components in the comb group or port
                let mut shareable_uses: TypeNameSet = HashSet::new();
                // Go through while body and while port + comb group once
                let (mut alive, mut gens, kills) =
                    self.build_live_ranges(body, alive, gens, kills);
                if let Some(cell_info) = port_if_shareable {
                    // adds port to cgroup_reads if state_shareable.
                    cgroup_reads.insert(cell_info);
                }
                if let Some(comb_group) = cond {
                    let (share_uses, state_reads) = Self::uses_reads_cgroup(
                        comb_group,
                        &self.share,
                        &self.state_share,
                    );
                    shareable_uses = share_uses;
                    cgroup_reads.extend(state_reads);
                }
                // setting alive and gens appropriately based on the updated info
                // from the comb group + port.
                alive.or_set(cgroup_reads.clone());
                gens.or_set(cgroup_reads.clone());

                if !cgroup_reads.is_empty() || !shareable_uses.is_empty() {
                    // add all uses of shareable and non-shareable components into
                    // cgroup_uses_map
                    let mut all_uses = cgroup_reads.clone();
                    all_uses.extend(shareable_uses);
                    self.cgroup_uses_map.insert(id, all_uses);
                }

                // Going through the while body and guard + port once again
                let (mut alive, mut gens, kills) =
                    self.build_live_ranges(body, alive, gens, kills);
                alive.or_set(cgroup_reads.clone());
                gens.or_set(cgroup_reads);
                (alive, gens, kills)
            }
        }
    }
}
