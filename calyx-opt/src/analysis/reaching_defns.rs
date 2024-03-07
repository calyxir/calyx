//! Calculate the reaching definitions in a control program.
use calyx_ir as ir;
use itertools::Itertools;
use std::cmp::Ordering;
use std::cmp::{Ord, PartialOrd};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    ops::BitOr,
};

use super::read_write_set::AssignmentAnalysis;

const INVOKE_PREFIX: &str = "__invoke_";

type GroupName = ir::Id;
type InvokeName = ir::Id;

/// A wrapper enum to distinguish between Ids that come from groups and ids that
/// were fabricated during the analysis for individual invoke statements. This
/// prevents attempting to look up the ids used for the invoke statements as
/// there will be no corresponding group.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum GroupOrInvoke {
    Group(GroupName),
    Invoke(InvokeName),
}

impl PartialOrd for GroupOrInvoke {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GroupOrInvoke {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (GroupOrInvoke::Group(a), GroupOrInvoke::Group(b))
            | (GroupOrInvoke::Invoke(a), GroupOrInvoke::Invoke(b)) => {
                ir::Id::cmp(a, b)
            }
            (GroupOrInvoke::Group(_), GroupOrInvoke::Invoke(_)) => {
                Ordering::Greater
            }
            (GroupOrInvoke::Invoke(_), GroupOrInvoke::Group(_)) => {
                Ordering::Less
            }
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<ir::Id> for GroupOrInvoke {
    fn into(self) -> ir::Id {
        match self {
            GroupOrInvoke::Group(id) | GroupOrInvoke::Invoke(id) => id,
        }
    }
}

#[derive(Debug, Default)]
pub struct MetadataMap {
    map: HashMap<*const ir::Invoke, ir::Id>,
    static_map: HashMap<*const ir::StaticInvoke, ir::Id>,
}

impl MetadataMap {
    fn attach_label(&mut self, invoke: &ir::Invoke, label: ir::Id) {
        self.map.insert(invoke as *const ir::Invoke, label);
    }

    fn attach_label_static(
        &mut self,
        invoke: &ir::StaticInvoke,
        label: ir::Id,
    ) {
        self.static_map
            .insert(invoke as *const ir::StaticInvoke, label);
    }

    pub fn fetch_label(&self, invoke: &ir::Invoke) -> Option<&ir::Id> {
        self.map.get(&(invoke as *const ir::Invoke))
    }

    pub fn fetch_label_static(
        &self,
        invoke: &ir::StaticInvoke,
    ) -> Option<&ir::Id> {
        self.static_map.get(&(invoke as *const ir::StaticInvoke))
    }
}
/// A datastructure used to represent a set of definitions/uses. These are
/// represented as pairs of (Id, GroupOrInvoke) where the Id is the identifier
/// being defined, and the second term represents the defining location (or use
/// location). In the case of a group, this location is just the group Id. In
/// the case of an invoke the "location" is a unique label assigned to each
/// invoke statement that beings with the INVOKE_PREFIX.
///
/// Defsets are constructed based on the assignments in a group and the ports in
/// an invoke. If a group writes to a register then it corresponds to a
/// definition (REGID, GROUPNAME). Similarly, this can be used to represent a
/// use of the register REGID in the very same group.
///
/// These structs are used both to determine what definitions reach a given
/// location and are also used to ensure that uses of a given definition (or
/// group of definitions are appropriately tied to any renaming that the
/// particular definition undergoes.
#[derive(Clone, Debug, Default)]
pub struct DefSet {
    set: BTreeSet<(ir::Id, GroupOrInvoke)>,
}

impl DefSet {
    fn extend(&mut self, writes: BTreeSet<ir::Id>, grp: GroupName) {
        for var in writes {
            self.set.insert((var, GroupOrInvoke::Group(grp)));
        }
    }

    fn kill_from_writeread(
        &self,
        writes: &BTreeSet<ir::Id>,
        reads: &BTreeSet<ir::Id>,
    ) -> (Self, KilledSet) {
        let mut killed = KilledSet::new();
        let def = DefSet {
            set: self
                .set
                .iter()
                .cloned()
                .filter_map(|(name, grp)| {
                    if !writes.contains(&name) || reads.contains(&name) {
                        Some((name, grp))
                    } else {
                        killed.insert(name);
                        None
                    }
                })
                .collect(),
        };
        (def, killed)
    }

    fn kill_from_hashset(&self, killset: &BTreeSet<ir::Id>) -> Self {
        DefSet {
            set: self
                .set
                .iter()
                .filter(|&(name, _)| !killset.contains(name))
                .cloned()
                .collect(),
        }
    }
}

impl BitOr<&DefSet> for &DefSet {
    type Output = DefSet;

    fn bitor(self, rhs: &DefSet) -> Self::Output {
        DefSet {
            set: &self.set | &rhs.set,
        }
    }
}

type OverlapMap = BTreeMap<ir::Id, Vec<BTreeSet<(ir::Id, GroupOrInvoke)>>>;

/// A struct used to compute a reaching definition analysis. The only field is a
/// map between [GroupOrInvoke] labels and the definitions that exit the given
/// group or the given Invoke node. This analysis is conservative and will only
/// kill a definition if the group MUST write the given register and does not
/// read it. If this is not the case old definitions will remain in the reaching
/// set as we cannot be certain that they have been killed.
///
/// Note that this analysis assumes that groups do not appear more than once
/// within the control structure and will provide inaccurate results if this
/// expectation is violated.
///
/// Like [super::LiveRangeAnalysis] par blocks are treated via a parallel CFG approach.
/// Concretely this means that after a par block executes any id that is killed
/// by one arm is killed and all defs introduced (but not killed) by any arm are
/// defined. Note that this assumes separate arms are not writing the same
/// register or reading a registe that is written by another arm.
#[derive(Debug, Default)]
pub struct ReachingDefinitionAnalysis {
    pub reach: BTreeMap<GroupOrInvoke, DefSet>,
    pub meta: MetadataMap,
}

impl ReachingDefinitionAnalysis {
    /// Constructs a reaching definition analysis for registers over the given
    /// control structure. Will include dummy "definitions" for invoke statements
    /// which can be ignored if one is not rewriting values
    /// **NOTE**: Assumes that each group appears at only one place in the control
    /// structure.
    pub fn new(control: &ir::Control) -> Self {
        let initial_set = DefSet::default();
        let mut analysis = ReachingDefinitionAnalysis::default();
        let mut counter: u64 = 0;

        build_reaching_def(
            control,
            initial_set,
            KilledSet::new(),
            &mut analysis,
            &mut counter,
        );
        analysis
    }

    /// Provides a map containing a vector of sets for each register. The sets
    /// within contain separate groupings of definitions for the given register.
    /// If the vector contains one set, then all the definitions for the given
    /// register name must have the same name.
    /// **NOTE:** Includes dummy "definitions" for continuous assignments and
    /// uses within groups and invoke statements. This is to ensure that all
    /// uses of a given register are rewriten with the appropriate name.
    pub fn calculate_overlap<'a, I, T: 'a>(
        &'a self,
        continuous_assignments: I,
    ) -> OverlapMap
    where
        I: Iterator<Item = &'a ir::Assignment<T>> + Clone + 'a,
    {
        let continuous_regs: Vec<ir::Id> = continuous_assignments
            .analysis()
            .cell_uses()
            .filter_map(|cell| {
                let cell_ref = cell.borrow();
                if let Some(name) = cell_ref.type_name() {
                    if name == "std_reg" {
                        return Some(cell_ref.name());
                    }
                }
                None
            })
            .collect();

        let mut overlap_map: BTreeMap<
            ir::Id,
            Vec<BTreeSet<(ir::Id, GroupOrInvoke)>>,
        > = BTreeMap::new();
        for (grp, defset) in &self.reach {
            let mut group_overlaps: BTreeMap<
                &ir::Id,
                BTreeSet<(ir::Id, GroupOrInvoke)>,
            > = BTreeMap::new();

            for (defname, group_name) in &defset.set {
                let set = group_overlaps.entry(defname).or_default();
                set.insert((*defname, group_name.clone()));
                set.insert((*defname, grp.clone()));
            }

            for name in &continuous_regs {
                let set = group_overlaps.entry(name).or_default();
                set.insert((
                    *name,
                    GroupOrInvoke::Group("__continuous".into()),
                ));
            }

            for (defname, set) in group_overlaps {
                let overlap_vec = overlap_map.entry(*defname).or_default();

                if overlap_vec.is_empty() {
                    overlap_vec.push(set)
                } else {
                    let mut no_overlap = vec![];
                    let mut overlap = vec![];

                    for entry in overlap_vec.drain(..) {
                        if set.is_disjoint(&entry) {
                            no_overlap.push(entry)
                        } else {
                            overlap.push(entry)
                        }
                    }

                    *overlap_vec = no_overlap;

                    if overlap.is_empty() {
                        overlap_vec.push(set);
                    } else {
                        overlap_vec.push(
                            overlap
                                .into_iter()
                                .fold(set, |acc, entry| &acc | &entry),
                        )
                    }
                }
            }
        }
        overlap_map
    }
}

type KilledSet = BTreeSet<ir::Id>;

fn remove_entries_defined_by(set: &mut KilledSet, defs: &DefSet) {
    let tmp_set: BTreeSet<_> = defs.set.iter().map(|(id, _)| id).collect();
    *set = std::mem::take(set)
        .into_iter()
        .filter(|x| !tmp_set.contains(x))
        .collect();
}

/// Returns the register cells whose out port is read anywhere in the given
/// assignments
fn register_reads<T>(assigns: &[ir::Assignment<T>]) -> BTreeSet<ir::Id> {
    assigns
        .iter()
        .analysis()
        .reads()
        .filter_map(|p| {
            let port = p.borrow();
            let ir::PortParent::Cell(cell_wref) = &port.parent else {
                unreachable!("Port not part of a cell");
            };
            // Skip this if the port is not an output
            if &port.name != "out" {
                return None;
            };
            let cr = cell_wref.upgrade();
            let cell = cr.borrow();
            if cell.is_primitive(Some("std_reg")) {
                Some(cr.borrow().name())
            } else {
                None
            }
        })
        .unique()
        .collect()
}

// handles `build_reaching_defns` for the enable/static_enables case.
// asgns are the assignments in the group (either static or dynamic)
fn handle_reaching_def_enables<T>(
    asgns: &[ir::Assignment<T>],
    reach: DefSet,
    rd: &mut ReachingDefinitionAnalysis,
    group_name: ir::Id,
) -> (DefSet, KilledSet) {
    let writes = asgns.iter().analysis().must_writes().cells();
    // for each write:
    // Killing all other reaching defns for that var
    // generating a new defn (Id, GROUP)
    let write_set = writes
        .filter(|x| match &x.borrow().prototype {
            ir::CellType::Primitive { name, .. } => name == "std_reg",
            _ => false,
        })
        .map(|x| x.borrow().name())
        .collect::<BTreeSet<_>>();

    let read_set = register_reads(asgns);

    // only kill a def if the value is not read.
    let (mut cur_reach, killed) =
        reach.kill_from_writeread(&write_set, &read_set);
    cur_reach.extend(write_set, group_name);

    rd.reach
        .insert(GroupOrInvoke::Group(group_name), cur_reach.clone());

    (cur_reach, killed)
}

fn build_reaching_def_static(
    sc: &ir::StaticControl,
    reach: DefSet,
    killed: KilledSet,
    rd: &mut ReachingDefinitionAnalysis,
    counter: &mut u64,
) -> (DefSet, KilledSet) {
    match sc {
        ir::StaticControl::Empty(_) => (reach, killed),
        ir::StaticControl::Enable(sen) => handle_reaching_def_enables(
            &sen.group.borrow().assignments,
            reach,
            rd,
            sen.group.borrow().name(),
        ),
        ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
            let (post_cond_def, post_cond_killed) = build_reaching_def_static(
                &ir::StaticControl::empty(),
                reach.clone(),
                killed,
                rd,
                counter,
            );

            let (round_1_def, mut round_1_killed) = build_reaching_def_static(
                body,
                post_cond_def,
                post_cond_killed,
                rd,
                counter,
            );

            remove_entries_defined_by(&mut round_1_killed, &reach);

            let (post_cond2_def, post_cond2_killed) = build_reaching_def(
                &ir::Control::empty(),
                &round_1_def | &reach,
                round_1_killed,
                rd,
                counter,
            );
            // Run the analysis a second time to get the fixed point of the
            // while loop using the defsets calculated during the first iteration
            let (final_def, mut final_kill) = build_reaching_def_static(
                body,
                post_cond2_def.clone(),
                post_cond2_killed,
                rd,
                counter,
            );

            remove_entries_defined_by(&mut final_kill, &post_cond2_def);

            (&final_def | &post_cond2_def, final_kill)
        }

        ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => stmts
            .iter()
            .fold((reach, killed), |(acc, killed), inner_c| {
                build_reaching_def_static(inner_c, acc, killed, rd, counter)
            }),
        ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
            let (defs, par_killed): (Vec<DefSet>, Vec<KilledSet>) = stmts
                .iter()
                .map(|ctrl| {
                    build_reaching_def_static(
                        ctrl,
                        reach.clone(),
                        KilledSet::new(),
                        rd,
                        counter,
                    )
                })
                .unzip();

            let global_killed = par_killed
                .iter()
                .fold(KilledSet::new(), |acc, set| &acc | set);

            let par_exit_defs = defs
                .iter()
                .zip(par_killed.iter())
                .map(|(defs, kills)| {
                    defs.kill_from_hashset(&(&global_killed - kills))
                })
                .fold(DefSet::default(), |acc, element| &acc | &element);
            (par_exit_defs, &global_killed | &killed)
        }
        ir::StaticControl::If(ir::StaticIf {
            tbranch, fbranch, ..
        }) => {
            let (post_cond_def, post_cond_killed) = build_reaching_def_static(
                &ir::StaticControl::empty(),
                reach,
                killed,
                rd,
                counter,
            );
            let (t_case_def, t_case_killed) = build_reaching_def_static(
                tbranch,
                post_cond_def.clone(),
                post_cond_killed.clone(),
                rd,
                counter,
            );
            let (f_case_def, f_case_killed) = build_reaching_def_static(
                fbranch,
                post_cond_def,
                post_cond_killed,
                rd,
                counter,
            );
            (&t_case_def | &f_case_def, &t_case_killed | &f_case_killed)
        }
        ir::StaticControl::Invoke(invoke) => {
            *counter += 1;

            let iterator = invoke
                .inputs
                .iter()
                .chain(invoke.outputs.iter())
                .filter_map(|(_, port)| {
                    if let ir::PortParent::Cell(wc) = &port.borrow().parent {
                        let rc = wc.upgrade();
                        let parent = rc.borrow();
                        if parent
                            .type_name()
                            .unwrap_or_else(|| ir::Id::from(""))
                            == "std_reg"
                        {
                            let name = format!("{}{}", INVOKE_PREFIX, counter);
                            rd.meta.attach_label_static(
                                invoke,
                                ir::Id::from(name.clone()),
                            );
                            return Some((
                                parent.name(),
                                GroupOrInvoke::Invoke(ir::Id::from(name)),
                            ));
                        }
                    }
                    None
                });

            let mut new_reach = reach;
            new_reach.set.extend(iterator);

            (new_reach, killed)
        }
    }
}

// Handles both `repeat` and `while` bodies when building reaching defs.
fn handle_repeat_while_body(
    body: &ir::Control,
    reach: DefSet,
    killed: KilledSet,
    rd: &mut ReachingDefinitionAnalysis,
    counter: &mut u64,
) -> (DefSet, KilledSet) {
    let (post_cond_def, post_cond_killed) = build_reaching_def(
        &ir::Control::empty(),
        reach.clone(),
        killed,
        rd,
        counter,
    );

    let (round_1_def, mut round_1_killed) =
        build_reaching_def(body, post_cond_def, post_cond_killed, rd, counter);

    remove_entries_defined_by(&mut round_1_killed, &reach);

    let (post_cond2_def, post_cond2_killed) = build_reaching_def(
        &ir::Control::empty(),
        &round_1_def | &reach,
        round_1_killed,
        rd,
        counter,
    );
    // Run the analysis a second time to get the fixed point of the
    // while loop using the defsets calculated during the first iteration
    let (final_def, mut final_kill) = build_reaching_def(
        body,
        post_cond2_def.clone(),
        post_cond2_killed,
        rd,
        counter,
    );

    remove_entries_defined_by(&mut final_kill, &post_cond2_def);

    (&final_def | &post_cond2_def, final_kill)
}

fn build_reaching_def(
    c: &ir::Control,
    reach: DefSet,
    killed: KilledSet,
    rd: &mut ReachingDefinitionAnalysis,
    counter: &mut u64,
) -> (DefSet, KilledSet) {
    match c {
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            stmts
                .iter()
                .fold((reach, killed), |(acc, killed), inner_c| {
                    build_reaching_def(inner_c, acc, killed, rd, counter)
                })
        }
        ir::Control::Par(ir::Par { stmts, .. }) => {
            let (defs, par_killed): (Vec<DefSet>, Vec<KilledSet>) = stmts
                .iter()
                .map(|ctrl| {
                    build_reaching_def(
                        ctrl,
                        reach.clone(),
                        KilledSet::new(),
                        rd,
                        counter,
                    )
                })
                .unzip();

            let global_killed = par_killed
                .iter()
                .fold(KilledSet::new(), |acc, set| &acc | set);

            let par_exit_defs = defs
                .iter()
                .zip(par_killed.iter())
                .map(|(defs, kills)| {
                    defs.kill_from_hashset(&(&global_killed - kills))
                })
                .fold(DefSet::default(), |acc, element| &acc | &element);
            (par_exit_defs, &global_killed | &killed)
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            let (post_cond_def, post_cond_killed) = build_reaching_def(
                &ir::Control::empty(),
                reach,
                killed,
                rd,
                counter,
            );
            let (t_case_def, t_case_killed) = build_reaching_def(
                tbranch,
                post_cond_def.clone(),
                post_cond_killed.clone(),
                rd,
                counter,
            );
            let (f_case_def, f_case_killed) = build_reaching_def(
                fbranch,
                post_cond_def,
                post_cond_killed,
                rd,
                counter,
            );
            (&t_case_def | &f_case_def, &t_case_killed | &f_case_killed)
        }
        ir::Control::While(ir::While { body, .. }) => {
            handle_repeat_while_body(body, reach, killed, rd, counter)
        }
        ir::Control::Invoke(invoke) => {
            *counter += 1;

            let iterator = invoke
                .inputs
                .iter()
                .chain(invoke.outputs.iter())
                .filter_map(|(_, port)| {
                    if let ir::PortParent::Cell(wc) = &port.borrow().parent {
                        let rc = wc.upgrade();
                        let parent = rc.borrow();
                        if parent
                            .type_name()
                            .unwrap_or_else(|| ir::Id::from(""))
                            == "std_reg"
                        {
                            let name = format!("{}{}", INVOKE_PREFIX, counter);
                            rd.meta.attach_label(
                                invoke,
                                ir::Id::from(name.clone()),
                            );
                            return Some((
                                parent.name(),
                                GroupOrInvoke::Invoke(ir::Id::from(name)),
                            ));
                        }
                    }
                    None
                });

            let mut new_reach = reach;
            new_reach.set.extend(iterator);

            (new_reach, killed)
        }
        ir::Control::Enable(en) => handle_reaching_def_enables(
            &en.group.borrow().assignments,
            reach,
            rd,
            en.group.borrow().name(),
        ),
        ir::Control::Empty(_) => (reach, killed),
        ir::Control::Repeat(ir::Repeat { body, .. }) => {
            handle_repeat_while_body(body, reach, killed, rd, counter)
        }
        ir::Control::Static(sc) => {
            build_reaching_def_static(sc, reach, killed, rd, counter)
        }
    }
}
