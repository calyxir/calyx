use crate::{
    analysis::{ReadWriteSet, ShareSet, VariableDetection},
    ir::{self, CloneName, RRC},
};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    ops::{BitOr, Sub},
};

type LiveCellRepresentation = HashSet<(ir::CellType, ir::Id)>;
const NODE_ID: &str = "NODE_ID";

/// The data structure used to represent sets of ids. This is used to represent
/// the `live`, `gen`, and `kill` sets.
#[derive(Default, Clone)]
pub struct Prop {
    map: HashMap<ir::CellType, HashSet<ir::Id>>,
}

/// Conversion to Prop from things that can be converted to HashSet<ir::Id>.
impl<T: Into<HashMap<ir::CellType, HashSet<ir::Id>>>> From<T> for Prop {
    fn from(t: T) -> Self {
        Prop { map: t.into() }
    }
}

/// Implement convenience math operators for Prop
impl BitOr<&Prop> for &Prop {
    type Output = Prop;
    fn bitor(self, rhs: &Prop) -> Self::Output {
        let mut map: HashMap<_, HashSet<_>> = self.map.clone();
        for (cell_type, cell_names) in &rhs.map {
            map.entry(cell_type.clone())
                .or_default()
                .extend(cell_names.clone());
        }
        Prop { map }
    }
}

impl Sub<&Prop> for &Prop {
    type Output = Prop;
    fn sub(self, rhs: &Prop) -> Self::Output {
        let mut map: HashMap<_, HashSet<_>> = self.map.clone();
        for (cell_type, cell_names) in &rhs.map {
            map.entry(cell_type.clone())
                .or_default()
                .retain(|name| !cell_names.clone().contains(name));
        }
        Prop { map }
    }
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
    ///   `(alive - kill) + gen`
    fn transfer(self, gen: &Prop, kill: &Prop) -> Self {
        &(&self - kill) | gen
    }

    /// Defines the data_flow transfer function. `(alive - kill) + gen`.
    /// However, this is for when gen and kill are sets, whereas self holds a map.
    fn transfer_set(
        self,
        gen: &LiveCellRepresentation,
        kill: &LiveCellRepresentation,
    ) -> Self {
        (self.sub_set(kill)).or_set(gen)
    }

    /// Add an element to Prop.
    fn insert(&mut self, (cell_type, cell_name): (ir::CellType, ir::Id)) {
        self.map.entry(cell_type).or_default().insert(cell_name);
    }

    fn or_set(&self, rhs: &LiveCellRepresentation) -> Self {
        let mut map: HashMap<_, HashSet<_>> = self.map.clone();
        for (cell_type, cell_name) in rhs {
            map.entry(cell_type.clone())
                .or_default()
                .insert(cell_name.clone());
        }
        Prop { map }
    }

    fn sub_set(&self, rhs: &LiveCellRepresentation) -> Self {
        let mut map: HashMap<_, HashSet<_>> = self.map.clone();
        for (cell_type, cell_name) in rhs {
            map.entry(cell_type.clone()).or_default().remove(cell_name);
        }
        Prop { map }
    }

    fn or_in_place(&mut self, rhs: Prop) {
        for (cell_type, cell_names) in rhs.map {
            self.map.entry(cell_type).or_default().extend(cell_names);
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
    /// Map from node (i.e., group enables or invokes) names
    /// to the components live inside them.
    live: HashMap<ir::Id, Prop>,
    /// Groups that have been identified as variable-like.
    /// Mapping from group name to the name of the register.
    variable_like: HashMap<ir::Id, Option<(ir::CellType, ir::Id)>>,
    /// Set of state shareable components (as type names)
    state_share: ShareSet,
    ///Set of shareable components (as type names)
    share: ShareSet,
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
    pub fn new(
        comp: &ir::Component,
        control: &ir::Control,
        state_share: ShareSet,
        share: ShareSet,
    ) -> Self {
        let mut ranges = LiveRangeAnalysis {
            state_share,
            share,
            ..Default::default()
        };

        build_live_ranges(
            control,
            Prop::default(),
            Prop::default(),
            Prop::default(),
            &mut ranges,
        );

        //adds (non-state) shareable cells as live in the group they're contained in
        //we already added (non-state) shareable cells as live in the invoke
        //they're contained in in build_live_ranges().
        comp.groups.iter().for_each(|group| {
            ranges.add_shareable_ranges(
                &group.borrow().assignments,
                group.borrow().name(),
            );
        });

        // Caleb: Right now we run remove-comb-groups before this is used so this code
        // doesn't do anything. Eventually, though, we want to be able to make
        // remove-comb-groups optional so I will keep this code.
        comp.comb_groups.iter().for_each(|group| {
            ranges.add_shareable_ranges(
                &group.borrow().assignments,
                group.borrow().name(),
            )
        });

        ranges
    }

    //For each cell used in assignments, adds it as part of the group_name's live range
    fn add_shareable_ranges(
        &mut self,
        assignments: &[ir::Assignment],
        group_name: &ir::Id,
    ) {
        let group_uses = ReadWriteSet::uses(assignments.iter())
            .filter(|cell| self.share.is_shareable_component(cell))
            .map(|cell| (cell.borrow().prototype.clone(), cell.clone_name()))
            .collect::<HashSet<_>>();
        match self.live.get_mut(group_name) {
            None => {
                unreachable!("Missing live range for {}. This might happen if a group is not used in the control program", group_name)
            }
            Some(prop) => *prop = prop.or_set(&group_uses),
        }
    }

    /// Returns a map from cell_type to map. maps each cell of type cell_type
    /// to the nodes (groups/invokes) at which cell is live.
    /// Essentially, this method allows us to go from a groups/invokes to cells mapping
    /// to a cells to groups/invokes mapping
    pub fn get_reverse(
        &mut self,
    ) -> HashMap<ir::CellType, HashMap<ir::Id, HashSet<&ir::Id>>> {
        let mut rev_map: HashMap<
            ir::CellType,
            HashMap<ir::Id, HashSet<&ir::Id>>,
        > = HashMap::new();
        for (group_name, prop) in &self.live {
            for (cell_type, cell_list) in &prop.map {
                let map = rev_map.entry(cell_type.clone()).or_default();
                for cell in cell_list {
                    map.entry(cell.clone()).or_default().insert(group_name);
                }
            }
        }
        rev_map
    }

    /// Given live_once_map, which maps control statement ids to maps of celltypes
    /// to cell_names, reorganizes the data by returning a map from
    /// celltypes to maps of cell_names to control statement ids.
    pub fn get_cell_to_control(
        live_once_map: HashMap<u64, HashMap<ir::CellType, HashSet<ir::Id>>>,
    ) -> HashMap<ir::CellType, HashMap<ir::Id, HashSet<u64>>> {
        let mut rev_map: HashMap<ir::CellType, HashMap<ir::Id, HashSet<u64>>> =
            HashMap::new();
        for (control_id, cell_type_map) in live_once_map {
            for (cell_type, cell_list) in cell_type_map {
                let cell_type_entry =
                    rev_map.entry(cell_type.clone()).or_default();
                for cell in cell_list {
                    cell_type_entry.entry(cell).or_default().insert(control_id);
                }
            }
        }
        rev_map
    }

    /// Updates live_once_map and par_thread_map.
    /// child_of_par indicates whether c is a direct child of a par block.
    /// is_in_par indecates whether c is nested within the par block, at any
    /// depth.
    /// live_once_map should only include control statements which are direct
    /// children of par blocks.
    /// if is_in_par is true, returns all of the cells live at some point within
    /// c. if it's false, behavior is unspecified.
    pub fn get_live_once_data(
        &self,
        live_once_map: &mut HashMap<
            u64,
            HashMap<ir::CellType, HashSet<ir::Id>>,
        >,
        par_thread_map: &mut HashMap<u64, u64>,
        c: &ir::Control,
        is_in_par: bool,
        child_of_par: bool,
    ) -> HashMap<ir::CellType, HashSet<ir::Id>> {
        match c {
            ir::Control::Empty(_) => HashMap::new(),
            ir::Control::Par(ir::Par { stmts, .. }) => {
                let parent_id = Self::get_guaranteed_id(c);
                let mut acc = HashMap::new();
                for stmt in stmts {
                    let live = self.get_live_once_data(
                        live_once_map,
                        par_thread_map,
                        stmt,
                        true,
                        true,
                    );
                    par_thread_map
                        .insert(Self::get_guaranteed_id(stmt), parent_id);
                    extend_hashmap(&mut acc, live);
                }
                if child_of_par {
                    live_once_map.insert(parent_id, acc.clone());
                }
                acc
            }
            ir::Control::Seq(ir::Seq { stmts, .. }) => {
                let mut acc = HashMap::new();
                for stmt in stmts {
                    let live = self.get_live_once_data(
                        live_once_map,
                        par_thread_map,
                        stmt,
                        is_in_par,
                        false,
                    );
                    if is_in_par {
                        extend_hashmap(&mut acc, live);
                    }
                }
                let id = Self::get_guaranteed_id(c);
                if child_of_par {
                    live_once_map.insert(id, acc.clone());
                }
                acc
            }
            ir::Control::If(ir::If {
                tbranch,
                fbranch,
                port,
                ..
            }) => {
                let mut tbranch = self.get_live_once_data(
                    live_once_map,
                    par_thread_map,
                    tbranch,
                    is_in_par,
                    false,
                );
                let fbranch = self.get_live_once_data(
                    live_once_map,
                    par_thread_map,
                    fbranch,
                    is_in_par,
                    false,
                );
                if is_in_par {
                    let id = Self::get_guaranteed_id(c);
                    extend_hashmap(&mut tbranch, fbranch);
                    if let Some((cell_type, cell_name)) =
                        LiveRangeAnalysis::port_to_cell_name(
                            port,
                            &self.state_share,
                        )
                    {
                        tbranch.entry(cell_type).or_default().insert(cell_name);
                    }
                    if child_of_par {
                        live_once_map.insert(id, tbranch.clone());
                    }
                    tbranch
                } else {
                    HashMap::new()
                }
            }
            ir::Control::While(ir::While { body, port, .. }) => {
                let mut body = self.get_live_once_data(
                    live_once_map,
                    par_thread_map,
                    body,
                    is_in_par,
                    false,
                );
                if is_in_par {
                    let id = Self::get_guaranteed_id(c);
                    if let Some((cell_type, cell_name)) =
                        LiveRangeAnalysis::port_to_cell_name(
                            port,
                            &self.state_share,
                        )
                    {
                        body.entry(cell_type).or_default().insert(cell_name);
                    }
                    if child_of_par {
                        live_once_map.insert(id, body.clone());
                    }
                    body
                } else {
                    HashMap::new()
                }
            }
            ir::Control::Enable(ir::Enable { group, .. }) => {
                if is_in_par {
                    let id = Self::get_guaranteed_id(c);
                    let live_set =
                        &self.live.get(&group.clone_name()).unwrap().map;
                    if child_of_par {
                        live_once_map.insert(id, live_set.clone());
                    }
                    return live_set.clone();
                }
                HashMap::new()
            }
            ir::Control::Invoke(ir::Invoke { comp, .. }) => {
                if is_in_par {
                    let id = Self::get_guaranteed_id(c);
                    let live_set =
                        &self.live.get(&comp.clone_name()).unwrap().map;
                    if child_of_par {
                        live_once_map.insert(id, live_set.clone());
                    }
                    return live_set.clone();
                }
                HashMap::new()
            }
        }
    }

    // Gets attribute s from c, panics otherwise. Should be used when you know
    // that c has attribute s. Potentially refactor (from domination map).
    fn get_guaranteed_id(c: &ir::Control) -> u64 {
        *c.get_attribute(NODE_ID).unwrap_or_else(||unreachable!(
            "called get_guaranteed_attribute, meaning we had to be sure it had the id"
        ))
    }

    /// Look up the set of things live at a node (i.e. group or invoke) definition.
    pub fn get(
        &self,
        node: &ir::Id,
    ) -> &HashMap<ir::CellType, HashSet<ir::Id>> {
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
    ) -> (LiveCellRepresentation, LiveCellRepresentation) {
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
    ) -> (LiveCellRepresentation, LiveCellRepresentation) {
        //The reads of the invoke include its inputs plus the cell itself, if the
        //outputs are not empty.
        let mut read_set: LiveCellRepresentation = invoke
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

        //The writes of the invoke include its outpus plus the cell itself, if the
        //inputs are not empty.
        let mut write_set: LiveCellRepresentation = invoke
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
}

/// Implements the parallel dataflow analysis that computes the liveness of every state shareable component
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
            let (reads, writes) = LiveRangeAnalysis::find_gen_kill_invoke(
                invoke,
                &lr.state_share,
            );
            // set the live set of this node to be the things live on the
            // output of this node plus the things written to in this invoke
            // plus all shareable components used
            let alive = alive.transfer_set(&reads, &writes);

            //get the shareable components used in the invoke stmt
            let (reads_share, writes_share) =
                LiveRangeAnalysis::find_gen_kill_invoke(invoke, &lr.share);
            let uses_share = &reads_share | &writes_share;

            let alive_writes = alive.or_set(&writes);
            lr.live.insert(
                invoke.comp.clone_name(),
                alive_writes.or_set(&uses_share),
            );
            (alive, gens.or_set(&reads), kills.or_set(&writes))
        }
        ir::Control::Enable(ir::Enable { group, .. }) => {
            // XXX(sam) no reason to compute this every time
            let (reads, writes) = lr.find_gen_kill_group(group);

            // compute transfer function
            let alive = alive.transfer_set(&reads, &writes);

            // set the live set of this node to be the things live on the
            // output of this node plus the things written to in this group
            lr.live.insert(group.clone_name(), alive.or_set(&writes));
            (alive, gens.or_set(&reads), kills.or_set(&writes))
        }
        ir::Control::Seq(ir::Seq { stmts, .. }) => stmts.iter().rev().fold(
            (alive, gens, kills),
            |(alive, gens, kills), e| {
                build_live_ranges(e, alive, gens, kills, lr)
            },
        ),
        ir::Control::If(ir::If {
            tbranch,
            fbranch,
            port,
            ..
        }) => {
            // compute each branch
            let (mut t_alive, mut t_gens, mut t_kills) = build_live_ranges(
                tbranch,
                alive.clone(),
                gens.clone(),
                kills.clone(),
                lr,
            );
            let (f_alive, f_gens, f_kills) =
                build_live_ranges(fbranch, alive, gens, kills, lr);

            // take union
            t_alive.or_in_place(f_alive);
            t_gens.or_in_place(f_gens);
            t_kills.or_in_place(f_kills);

            // feed to condition to compute
            if let Some(cell_info) =
                LiveRangeAnalysis::port_to_cell_name(port, &lr.state_share)
            {
                t_alive.insert(cell_info);
            }
            (t_alive, t_gens, t_kills)
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
                    |(mut acc_alive, mut acc_gen, mut acc_kill),
                     (alive, gen, kill)| {
                        (
                            // Doing in place operations saves time
                            {
                                acc_alive.or_in_place(alive);
                                acc_alive
                            },
                            {
                                acc_gen.or_in_place(gen);
                                acc_gen
                            },
                            {
                                acc_kill.or_in_place(kill);
                                acc_kill
                            },
                        )
                    },
                );

            let alive = alive.transfer(&gens, &kills);
            (alive, gens, kills)
        }
        ir::Control::While(ir::While { body, port, .. }) => {
            let (mut alive, gens, kills) =
                build_live_ranges(body, alive, gens, kills, lr);
            if let Some(cell) =
                LiveRangeAnalysis::port_to_cell_name(port, &lr.state_share)
            {
                alive.insert(cell)
            }
            build_live_ranges(body, alive, gens, kills, lr)
        }
    }
}

// given map, "extends" it to include data from rhs.
fn extend_hashmap(
    map: &mut HashMap<ir::CellType, HashSet<ir::Id>>,
    rhs: HashMap<ir::CellType, HashSet<ir::Id>>,
) {
    for (k, v) in rhs {
        map.entry(k).or_default().extend(v);
    }
}
