use crate::analysis::{
    AssignmentAnalysis, ControlId, ReadWriteSet, ShareSet, VariableDetection,
};
use calyx_ir::{self as ir, Id, RRC};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    rc::Rc,
};

type TypeNameSet = HashSet<(ir::CellType, ir::Id)>;
type CellsByType = HashMap<ir::CellType, HashSet<ir::Id>>;
// maps cell type to maps that map cell name to control statement
type LiveMapByType = HashMap<ir::CellType, HashMap<ir::Id, HashSet<u64>>>;
type ReadWriteInfo = (
    HashSet<(ir::CellType, ir::Id)>,
    HashSet<(ir::CellType, ir::Id)>,
);
type InvokeInfo<'a> = (
    &'a [(ir::Id, ir::RRC<ir::Port>)],
    &'a [(ir::Id, ir::RRC<ir::Port>)],
);

/// Returns [ir::Cell] which are read from in the assignments.
/// **Ignores** reads from group holes, and reads from done signals, when it
/// is safe to do so.
/// To ignore a read from a done signal:
/// the `@go` signal for the same cell *must* be written to in the group
pub fn meaningful_read_set<'a, T: 'a>(
    assigns: impl Iterator<Item = &'a ir::Assignment<T>> + Clone + 'a,
) -> impl Iterator<Item = RRC<ir::Cell>> + 'a {
    meaningful_port_read_set(assigns)
        .map(|port| Rc::clone(&port.borrow().cell_parent()))
        .unique_by(|cell| cell.borrow().name())
}

/// Returns the "meaningful" [ir::Port] which are read from in the assignments.
/// "Meaningful" means we just exclude the following `@done` reads:
/// the `@go` signal for the same cell *must* be written to in the group
pub fn meaningful_port_read_set<'a, T: 'a>(
    assigns: impl Iterator<Item = &'a ir::Assignment<T>> + Clone + 'a,
) -> impl Iterator<Item = RRC<ir::Port>> + 'a {
    // go_writes = all cells which are guaranteed to have their go port written to in assigns
    let go_writes: Vec<RRC<ir::Cell>> = assigns
        .clone()
        .filter(|asgn| {
            // to be included in go_writes, one of the following must hold:
            // a) guard is true
            // b cell.go = !cell.done ? 1'd1
            if asgn.guard.is_true() {
                return true;
            }

            // checking cell.go = !cell.done! 1'd1
            asgn.dst.borrow().attributes.has(ir::NumAttr::Go)
                && asgn.guard.is_not_done(
                    &asgn.dst.borrow().cell_parent().borrow().name(),
                )
                && asgn.src.borrow().is_constant(1, 1)
        })
        .analysis()
        .writes()
        .filter(|port| port.borrow().attributes.has(ir::NumAttr::Go))
        .map(|port| Rc::clone(&port.borrow().cell_parent()))
        .collect();

    // if we have a done port that overlaps with go_writes, then can remove the
    // done port. Otherwise, we should keep it.
    assigns
        .flat_map(ReadWriteSet::port_reads)
        .filter(move |port| {
            if port.borrow().attributes.has(ir::NumAttr::Done) {
                let done_parent = Rc::clone(&port.borrow().cell_parent());
                go_writes
                    .iter()
                    .all(|go_parent| !Rc::ptr_eq(go_parent, &done_parent))
            } else {
                true
            }
        })
}

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

    fn insert(&mut self, (cell_type, cell_name): (ir::CellType, ir::Id)) {
        self.map.entry(cell_type).or_default().insert(cell_name);
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
    /// maps invokes/enable ids to the shareable cell types/names used in them
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

        ranges
    }

    // updates live_cell_map and live_once_map
    // maps all cells live at node `id` to node `id` in `live_cells_map`,
    // and maps all cells live at node `id` to `parents` in `live_once_map`.
    fn update_live_control_data(
        &self,
        id: u64,
        live_once_map: &mut LiveMapByType,
        live_cell_map: &mut LiveMapByType,
        parents: &HashSet<u64>,
    ) {
        let live_set = self.live.get(&id).unwrap().map.clone();
        for (cell_type, live_cells) in live_set {
            let cell_to_node =
                live_cell_map.entry(cell_type.clone()).or_default();
            let cell_to_control = live_once_map.entry(cell_type).or_default();
            for cell in live_cells {
                cell_to_node.entry(cell).or_default().insert(id);
                cell_to_control.entry(cell).or_default().extend(parents);
            }
        }
    }

    fn add_cell_to_control_data(
        id: u64,
        (cell_type, cell_name): &(ir::CellType, ir::Id),
        live_once_map: &mut LiveMapByType,
        live_cell_map: &mut LiveMapByType,
        parents: &HashSet<u64>,
    ) {
        // add cell as live within whichever direct children of
        // par blocks they're located within
        if !parents.is_empty() {
            live_once_map
                .entry(cell_type.clone())
                .or_default()
                .entry(*cell_name)
                .or_default()
                .extend(parents);
        }
        // mark cell as live in the control id
        // If id corresponds to an if/while guard,
        // what is really means, is that the cell is live
        // at the comb group/port guard of the if/while statement
        live_cell_map
            .entry(cell_type.clone())
            .or_default()
            .entry(*cell_name)
            .or_default()
            .insert(id);
    }

    fn get_live_control_data_static(
        &self,
        live_once_map: &mut LiveMapByType,
        par_thread_map: &mut HashMap<u64, u64>,
        live_cell_map: &mut LiveMapByType,
        parents: &HashSet<u64>,
        sc: &ir::StaticControl,
    ) {
        match sc {
            ir::StaticControl::Empty(_) => (),
            ir::StaticControl::Enable(_) | ir::StaticControl::Invoke(_) => {
                let id = ControlId::get_guaranteed_id_static(sc);
                self.update_live_control_data(
                    id,
                    live_once_map,
                    live_cell_map,
                    parents,
                )
            }
            ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
                self.get_live_control_data_static(
                    live_once_map,
                    par_thread_map,
                    live_cell_map,
                    parents,
                    body,
                );
            }
            ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => {
                for stmt in stmts {
                    self.get_live_control_data_static(
                        live_once_map,
                        par_thread_map,
                        live_cell_map,
                        parents,
                        stmt,
                    );
                }
            }
            ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
                let parent_id = ControlId::get_guaranteed_id_static(sc);
                let mut new_parents = parents.clone();
                for stmt in stmts {
                    // building par_thread_map
                    let child_id = ControlId::get_guaranteed_id_static(stmt);
                    par_thread_map.insert(child_id, parent_id);

                    // building live_once_map by adding child_id to parents when
                    // we recursively call get_live_control_data on each child
                    new_parents.insert(child_id);
                    self.get_live_control_data_static(
                        live_once_map,
                        par_thread_map,
                        live_cell_map,
                        &new_parents,
                        stmt,
                    );
                    new_parents.remove(&child_id);
                }
            }
            ir::StaticControl::If(ir::StaticIf {
                port,
                tbranch,
                fbranch,
                ..
            }) => {
                self.get_live_control_data_static(
                    live_once_map,
                    par_thread_map,
                    live_cell_map,
                    parents,
                    tbranch,
                );
                self.get_live_control_data_static(
                    live_once_map,
                    par_thread_map,
                    live_cell_map,
                    parents,
                    fbranch,
                );
                let id = ControlId::get_guaranteed_id_static(sc);
                // Examining the cell read from in the if guard
                if let Some(cell_info) = LiveRangeAnalysis::port_to_cell_name(
                    port,
                    &self.state_share,
                ) {
                    Self::add_cell_to_control_data(
                        id,
                        &cell_info,
                        live_once_map,
                        live_cell_map,
                        parents,
                    )
                }
            }
        }
    }

    /// Updates live_once_map and par_thread_map.
    /// live_once_map should map celltypes to a map, which should map cells of
    /// celltype to control statements in which it is live for at least one group
    /// or invoke in the control. We only map to control statements that are
    /// direct children of par blocks.
    /// par_thread_map maps direct children of par blocks to their parents
    /// live_cell_map maps cells to the nodes in which it is live
    /// par_thread_map maps direct children of par blocks to their parents
    /// parents is the list of current control statements (that are direct children
    /// of par blocks) that are parents (at any level of nesting) of c.
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
                    for cell_info in comb_group_uses {
                        Self::add_cell_to_control_data(
                            id,
                            cell_info,
                            live_once_map,
                            live_cell_map,
                            parents,
                        )
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
                                .entry(*cell_name)
                                .or_default()
                                .extend(parents);
                        }
                        live_cell_map
                            .entry(cell_type.clone())
                            .or_default()
                            .entry(*cell_name)
                            .or_default()
                            .insert(id);
                    }
                }
            }
            ir::Control::Repeat(ir::Repeat { body, .. }) => {
                self.get_live_control_data(
                    live_once_map,
                    par_thread_map,
                    live_cell_map,
                    parents,
                    body,
                );
            }
            ir::Control::Enable(_) | ir::Control::Invoke(_) => {
                let id = ControlId::get_guaranteed_id(c);
                self.update_live_control_data(
                    id,
                    live_once_map,
                    live_cell_map,
                    parents,
                )
            }
            ir::Control::Static(sc) => self.get_live_control_data_static(
                live_once_map,
                par_thread_map,
                live_cell_map,
                parents,
                sc,
            ),
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
        let name = &group.name();
        if !self.variable_like.contains_key(name) {
            let res = VariableDetection::variable_like(grp, &self.state_share);
            self.variable_like.insert(grp.borrow().name(), res);
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
            let reads: HashSet<_> = assignments
                .analysis()
                .cell_reads()
                .filter(|c| sc_clone.is_shareable_component(c))
                .map(|c| (c.borrow().prototype.clone(), c.borrow().name()))
                .collect();

            let mut writes = HashSet::new();
            writes.insert((cell_type, variable));

            (reads, writes)
        } else {
            let reads: HashSet<_> =
                meaningful_read_set(group.assignments.iter())
                    .filter(|c| sc_clone.is_shareable_component(c))
                    .map(|c| (c.borrow().prototype.clone(), c.borrow().name()))
                    .collect();

            // only consider write assignments where the guard is true
            let assignments = group
                .assignments
                .iter()
                .filter(|asgn| *asgn.guard == ir::Guard::True)
                .cloned()
                .collect::<Vec<_>>();

            let writes: HashSet<_> = assignments
                .iter()
                .analysis()
                .cell_writes()
                .filter(|c| sc_clone.is_shareable_component(c))
                .map(|c| (c.borrow().prototype.clone(), c.borrow().name()))
                .collect();

            (reads, writes)
        }
    }

    // TODO(calebmkim) TODO(paili0628): This is similar to find_static_group right now
    // We could eventually try to merge it, but we should do it after we have
    // hammered down the details of the rest of the static IR assignments
    fn find_gen_kill_static_group(
        &mut self,
        group_ref: &RRC<ir::StaticGroup>,
    ) -> (TypeNameSet, TypeNameSet) {
        let group = group_ref.borrow();
        // we don't have to worry about variable like for static groups
        let sc_clone = &self.state_share;
        let reads: HashSet<_> = meaningful_read_set(group.assignments.iter())
            .filter(|c| sc_clone.is_shareable_component(c))
            .map(|c| (c.borrow().prototype.clone(), c.borrow().name()))
            .collect();
        // only consider write assignments where the guard is true
        let assignments = group
            .assignments
            .iter()
            .filter(|asgn| *asgn.guard == ir::Guard::True)
            .cloned()
            .collect::<Vec<_>>();

        let writes: HashSet<_> = assignments
            .iter()
            .analysis()
            .cell_writes()
            .filter(|c| sc_clone.is_shareable_component(c))
            .map(|c| (c.borrow().prototype.clone(), c.borrow().name()))
            .collect();

        (reads, writes)
    }

    fn find_uses_assigns<T>(
        assigns: &[ir::Assignment<T>],
        shareable_components: &ShareSet,
    ) -> TypeNameSet {
        assigns
            .iter()
            .analysis()
            .cell_uses()
            .filter(|cell| shareable_components.is_shareable_component(cell))
            .map(|cell| (cell.borrow().prototype.clone(), cell.borrow().name()))
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
        let share_uses = group
            .assignments
            .iter()
            .analysis()
            .cell_uses()
            .filter(|cell| shareable.is_shareable_component(cell))
            .map(|cell| (cell.borrow().prototype.clone(), cell.borrow().name()))
            .collect::<HashSet<_>>();
        let state_reads = group
            .assignments
            .iter()
            .analysis()
            .reads()
            .cells()
            .filter(|cell| state_shareable.is_shareable_component(cell))
            .map(|cell| (cell.borrow().prototype.clone(), cell.borrow().name()))
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
                    cell.borrow().name(),
                ));
            }
        }
        None
    }

    // gets the gens/kills (aka reads/writes) of the invoke given inputs, outputs, and comb group.
    fn gen_kill_invoke(
        inputs: &[(ir::Id, ir::RRC<ir::Port>)],
        outputs: &[(ir::Id, ir::RRC<ir::Port>)],
        comb_group_info: &Option<ir::RRC<ir::CombGroup>>,
        comp: &ir::RRC<ir::Cell>,
        shareable_components: &ShareSet,
    ) -> (TypeNameSet, TypeNameSet) {
        // The writes of the invoke include its outputs. Also, we count the cell
        // being invoked as being written to.
        let mut write_set: TypeNameSet = outputs
            .iter()
            .filter_map(|(_, src)| {
                Self::port_to_cell_name(src, shareable_components)
            })
            .collect();

        if shareable_components.is_shareable_component(comp) {
            write_set.insert((
                comp.borrow().prototype.clone(),
                comp.borrow().name(),
            ));
        }

        // The reads of the invoke include its inputs.
        // One quick note: since the component is written to, there is no need to include this
        // component as being read from since we know the write to the component
        // precedes the read from it, due to the nature of `invoke` statements.
        // This is "cheating" in a sense, since the component is technically being
        // read from. However, since we know that there is a write to the component
        // that that precedes the read from it within the very same invoke statement,
        // it "appears" to all the other control statements in the program that the
        // component is not being read from in the invoke statement.
        let mut read_set: TypeNameSet = inputs
            .iter()
            .filter_map(|(_, src)| {
                Self::port_to_cell_name(src, shareable_components)
            })
            .collect();

        if let Some(comb_group) = comb_group_info {
            read_set.extend(
                comb_group
                    .borrow()
                    .assignments
                    .iter()
                    .analysis()
                    .reads()
                    .cells()
                    .filter(|cell| {
                        shareable_components.is_shareable_component(cell)
                    })
                    .map(|cell| {
                        (cell.borrow().prototype.clone(), cell.borrow().name())
                    }),
            );
        }

        (read_set, write_set)
    }

    // gets the uses of the invoke given inputs, outputs, and comb group.
    // Should include any cell that is either read from or written to at all
    // in the invoke statement (including the comb group)
    fn uses_invoke(
        inputs: &[(ir::Id, ir::RRC<ir::Port>)],
        outputs: &[(ir::Id, ir::RRC<ir::Port>)],
        comb_group_info: &Option<ir::RRC<ir::CombGroup>>,
        shareable_components: &ShareSet,
    ) -> TypeNameSet {
        // uses of shareable components in the invoke statement
        let mut uses: TypeNameSet = inputs
            .iter()
            .chain(outputs.iter())
            .filter_map(|(_, src)| {
                Self::port_to_cell_name(src, shareable_components)
            })
            .collect();
        // uses of shareable components in the comb group (if it exists)
        if let Some(comb_group) = &comb_group_info {
            uses.extend(
                comb_group
                    .borrow()
                    .assignments
                    .iter()
                    .analysis()
                    .cell_uses()
                    .filter(|cell| {
                        shareable_components.is_shareable_component(cell)
                    })
                    .map(|cell| {
                        (cell.borrow().prototype.clone(), cell.borrow().name())
                    }),
            );
        };
        uses
    }

    // updates liveness for an invoke: build to handle either static or dynamic invokes
    // invoke_info = (inputs, outputs) of invoke
    // comp = comp being invokes
    // comb_group_invo = Option<comb group if invoke has one>
    // liveness_info = (alive, gens, kills) coming into the invoke
    // returns the (alive, gens, kills) based on the invoke info
    // also updates self.invokes_enables_map using the input information
    fn update_invoke_liveness(
        &mut self,
        invoke_info: InvokeInfo,
        comb_group_info: &Option<ir::RRC<ir::CombGroup>>,
        comp: &ir::RRC<ir::Cell>,
        id: u64,
        liveness_info: (Prop, Prop, Prop),
    ) -> (Prop, Prop, Prop) {
        let (inputs, outputs) = invoke_info;
        let (mut alive, mut gens, mut kills) = liveness_info;

        // get uses of all shareable components, and then update self.invokes_enables_map
        let uses_shareable =
            Self::uses_invoke(inputs, outputs, comb_group_info, &self.share);

        self.invokes_enables_map
            .entry(id)
            .or_default()
            .extend(uses_shareable);

        // get the reads and writes of the invoke, and use that to determine livenes propogation
        let (reads, writes) = LiveRangeAnalysis::gen_kill_invoke(
            inputs,
            outputs,
            comb_group_info,
            comp,
            &self.state_share,
        );

        alive.transfer_set(reads.clone(), writes.clone());
        let alive_out = alive.clone();

        // set the live set of this node to be the things live on the
        // output of this node plus the things written to in this invoke
        // plus all shareable components used
        self.live.insert(id, {
            alive.or_set(writes.clone());
            alive
        });
        (
            alive_out,
            {
                gens.sub_set(writes.clone());
                gens.or_set(reads);
                gens
            },
            {
                kills.or_set(writes);
                kills
            },
        )
    }

    // Updates Live Range Analysis
    // id should correspond to the id of an enable, and assigns should correspond
    // to the assignments in that enable
    // reads and writes should be the reads/writes of the assigns
    // alive, gens, kills are the alive, gens, and kills coming into the enable
    // returns the alive, gens, and kills leaving the enable
    // It also updates self.live at id to be the cells live at live
    // It also updates self.invokes_enables_map
    fn update_group_liveness<T>(
        &mut self,
        assigns: &[ir::Assignment<T>],
        id: u64,
        read_write_info: ReadWriteInfo,
        mut alive: Prop,
        mut gens: Prop,
        mut kills: Prop,
    ) -> (Prop, Prop, Prop) {
        let uses_share =
            LiveRangeAnalysis::find_uses_assigns(assigns, &self.share);
        self.invokes_enables_map
            .entry(id)
            .or_default()
            .extend(uses_share);
        let (reads, writes) = read_write_info;
        // compute transfer function
        alive.transfer_set(reads.clone(), writes.clone());
        let alive_out = alive.clone();

        // set the live set of this node to be the things live on the
        // output of this node plus the things written to in this group
        self.live.insert(id, {
            alive.or_set(writes.clone());
            alive
        });
        (
            alive_out,
            {
                gens.sub_set(writes.clone());
                gens.or_set(reads);
                gens
            },
            {
                kills.or_set(writes);
                kills
            },
        )
    }

    fn build_live_ranges_static(
        &mut self,
        sc: &ir::StaticControl,
        alive: Prop,
        gens: Prop,
        kills: Prop,
    ) -> (Prop, Prop, Prop) {
        match sc {
            ir::StaticControl::Empty(_) => (alive, gens, kills),
            ir::StaticControl::Enable(ir::StaticEnable { group, .. }) => {
                // XXX(sam) no reason to compute this every time
                let (reads, writes) = self.find_gen_kill_static_group(group);
                self.update_group_liveness(
                    &group.borrow().assignments,
                    ControlId::get_guaranteed_id_static(sc),
                    (reads, writes),
                    alive,
                    gens,
                    kills,
                )
            }
            ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
                let (a, g, k) =
                    self.build_live_ranges_static(body, alive, gens, kills);
                // Have to go through the repeat body twice in order to get a
                // correct live range analysis
                self.build_live_ranges_static(body, a, g, k)
            }
            ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => stmts
                .iter()
                .rev()
                .fold((alive, gens, kills), |(alive, gens, kills), e| {
                    self.build_live_ranges_static(e, alive, gens, kills)
                }),
            ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
                let (mut alive, gens, kills) = stmts
                    .iter()
                    .rev()
                    .map(|e| {
                        self.build_live_ranges_static(
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
                // should only count as a "gen" if it is alive on at least one
                // of the outputs of the child node
                alive.transfer(gens.clone(), kills.clone());
                (alive, gens, kills)
            }
            ir::StaticControl::If(ir::StaticIf {
                tbranch,
                fbranch,
                port,
                ..
            }) => {
                // compute each branch
                let (mut t_alive, mut t_gens, mut t_kills) = self
                    .build_live_ranges_static(
                        tbranch,
                        alive.clone(),
                        gens.clone(),
                        kills.clone(),
                    );
                let (f_alive, f_gens, f_kills) =
                    self.build_live_ranges_static(fbranch, alive, gens, kills);

                // take union
                t_alive.or(f_alive);
                t_gens.or(f_gens);
                // kills must take intersection to be conservative
                t_kills.intersect(f_kills);

                // add if guard cell to the alive/gens sets
                if let Some(cell_info) = LiveRangeAnalysis::port_to_cell_name(
                    port,
                    &self.state_share,
                ) {
                    t_alive.insert(cell_info.clone());
                    t_gens.insert(cell_info);
                }

                (t_alive, t_gens, t_kills)
            }
            ir::StaticControl::Invoke(ir::StaticInvoke {
                inputs,
                outputs,
                comp,
                ..
            }) => {
                //get the shareable components used in the invoke stmt
                self.update_invoke_liveness(
                    (inputs, outputs),
                    &None,
                    comp,
                    ControlId::get_guaranteed_id_static(sc),
                    (alive, gens, kills),
                )
            }
        }
    }

    /// Implements the parallel dataflow analysis that computes the liveness of every state shareable component
    /// at every point in the program.
    fn build_live_ranges(
        &mut self,
        c: &ir::Control,
        alive: Prop,
        gens: Prop,
        kills: Prop,
    ) -> (Prop, Prop, Prop) {
        match c {
            ir::Control::Empty(_) => (alive, gens, kills),
            ir::Control::Invoke(ir::Invoke {
                inputs,
                outputs,
                comb_group,
                comp,
                ..
            }) => self.update_invoke_liveness(
                (inputs, outputs),
                comb_group,
                comp,
                ControlId::get_guaranteed_id(c),
                (alive, gens, kills),
            ),
            ir::Control::Enable(ir::Enable { group, .. }) => {
                // XXX(sam) no reason to compute this every time
                let (reads, writes) = self.find_gen_kill_group(group);

                self.update_group_liveness(
                    &group.borrow().assignments,
                    ControlId::get_guaranteed_id(c),
                    (reads, writes),
                    alive,
                    gens,
                    kills,
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
                // kills must be intersection to be conservative
                t_kills.intersect(f_kills);

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
                // should only count as a "gen" if it is alive on at least one
                // of the outputs of the child node
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

                let input_kills = kills.clone();
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

                // we can only inlcude the kills if we know the while loop executes
                // at least once
                if let Some(val) = c.get_attribute(ir::NumAttr::Bound) {
                    if val > 0 {
                        return (alive, gens, kills);
                    }
                }

                (alive, gens, input_kills)
            }
            ir::Control::Repeat(ir::Repeat { body, .. }) => {
                let (a, g, k) =
                    self.build_live_ranges(body, alive, gens, kills);
                // need to feed the live nodes on the output of the body
                // back into the body to get correct live range analysis
                self.build_live_ranges(body, a, g, k)
            }
            ir::Control::Static(sc) => {
                self.build_live_ranges_static(sc, alive, gens, kills)
            }
        }
    }
}
