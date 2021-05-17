use std::cmp::Ordering;
use std::cmp::{Ord, PartialOrd};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::BitOr,
    rc::Rc,
};

use crate::analysis::ReadWriteSet;
use crate::ir::{self, RRC};

pub const INVOKE_PREFIX: &str = "__invoke_";

type GroupName = ir::Id;
type InvokeName = ir::Id;

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
            (GroupOrInvoke::Group(a), GroupOrInvoke::Group(b)) => {
                ir::Id::cmp(a, b)
            }
            (GroupOrInvoke::Group(_), GroupOrInvoke::Invoke(_)) => {
                Ordering::Greater
            }
            (GroupOrInvoke::Invoke(_), GroupOrInvoke::Group(_)) => {
                Ordering::Less
            }
            (GroupOrInvoke::Invoke(a), GroupOrInvoke::Invoke(b)) => {
                ir::Id::cmp(a, b)
            }
        }
    }
}

impl Into<ir::Id> for GroupOrInvoke {
    fn into(self) -> ir::Id {
        match self {
            GroupOrInvoke::Group(id) | GroupOrInvoke::Invoke(id) => id,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DefSet {
    set: BTreeSet<(ir::Id, GroupOrInvoke)>,
}

impl DefSet {
    fn extend(&mut self, writes: BTreeSet<ir::Id>, grp: &GroupName) {
        for var in writes {
            self.set.insert((var, GroupOrInvoke::Group(grp.clone())));
        }
    }

    fn new() -> Self {
        DefSet {
            set: BTreeSet::new(),
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
                .cloned()
                .filter(|(name, _)| !killset.contains(name))
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

#[derive(Debug)]
pub struct ReachingDefinitionAnalysis {
    pub reach: BTreeMap<GroupOrInvoke, DefSet>,
}

impl ReachingDefinitionAnalysis {
    pub fn new(_comp: &ir::Component, control: &mut ir::Control) -> Self {
        let initial_set = DefSet::new();
        let mut analysis = ReachingDefinitionAnalysis::empty();
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

    fn empty() -> Self {
        ReachingDefinitionAnalysis {
            reach: BTreeMap::new(),
        }
    }

    pub fn calculate_overlap(
        &self,
        continuous_assignments: &[ir::Assignment],
    ) -> OverlapMap {
        let continuous_regs: Vec<RRC<ir::Cell>> =
            ReadWriteSet::uses(continuous_assignments)
                .into_iter()
                .filter(|cell| {
                    let cell_ref = cell.borrow();
                    if let Some(name) = cell_ref.type_name() {
                        name == "std_reg"
                    } else {
                        false
                    }
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
                set.insert((defname.clone(), group_name.clone()));
                set.insert((defname.clone(), grp.clone()));

                for name in &continuous_regs {
                    set.insert((
                        name.clone().borrow().name.clone(),
                        grp.clone(),
                    ));
                }
            }

            for (defname, set) in group_overlaps {
                let overlap_vec =
                    overlap_map.entry(defname.clone()).or_default();

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

fn build_reaching_def(
    c: &mut ir::Control,
    reach: DefSet,
    killed: KilledSet,
    rd: &mut ReachingDefinitionAnalysis,
    counter: &mut u64,
) -> (DefSet, KilledSet) {
    match c {
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            stmts
                .iter_mut()
                .fold((reach, killed), |(acc, killed), inner_c| {
                    build_reaching_def(inner_c, acc, killed, rd, counter)
                })
        }
        ir::Control::Par(ir::Par { stmts, .. }) => {
            let (defs, par_killed): (Vec<DefSet>, Vec<KilledSet>) = stmts
                .iter_mut()
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

            dbg!(&par_killed);
            let global_killed = par_killed
                .iter()
                .fold(KilledSet::new(), |acc, set| &acc | set);

            let par_exit_defs = defs
                .iter()
                .zip(par_killed.iter())
                .map(|(defs, kills)| {
                    dbg!(defs, &global_killed - kills);
                    let new = defs.kill_from_hashset(&(&global_killed - kills));
                    dbg!(&new);
                    new
                })
                .fold(DefSet::new(), |acc, element| &acc | &element);
            dbg!(&par_exit_defs);
            (par_exit_defs, &global_killed | &killed)
        }
        ir::Control::If(ir::If {
            tbranch,
            fbranch,
            cond,
            ..
        }) => {
            let mut fake_enable = ir::Control::Enable(ir::Enable {
                attributes: ir::Attributes::default(),
                group: Rc::clone(cond),
            });
            let (post_cond_def, post_cond_killed) = build_reaching_def(
                &mut fake_enable,
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
        ir::Control::While(ir::While { cond, body, .. }) => {
            let mut fake_enable = ir::Control::Enable(ir::Enable {
                attributes: ir::Attributes::default(),
                group: Rc::clone(cond),
            });
            let (post_cond_def, post_cond_killed) = build_reaching_def(
                &mut fake_enable,
                reach,
                killed,
                rd,
                counter,
            );

            let (round_1_def, round_1_killed) = build_reaching_def(
                body,
                post_cond_def,
                post_cond_killed,
                rd,
                counter,
            );
            let (post_cond2_def, post_cond2_killed) = build_reaching_def(
                &mut fake_enable,
                round_1_def,
                round_1_killed,
                rd,
                counter,
            );
            // Twice as nice?
            build_reaching_def(
                body,
                post_cond2_def,
                post_cond2_killed,
                rd,
                counter,
            )
        }
        ir::Control::Invoke(invoke) => {
            *counter += 1;
            let inputs: Vec<(ir::Id, RRC<ir::Port>)> =
                invoke.inputs.drain(..).collect();
            let outputs: Vec<(ir::Id, RRC<ir::Port>)> =
                invoke.outputs.drain(..).collect();

            let iterator =
                inputs.iter().chain(outputs.iter()).filter_map(|(_, port)| {
                    if let ir::PortParent::Cell(wc) = &port.borrow().parent {
                        let rc = wc.upgrade();
                        let parent = rc.borrow();
                        if parent.type_name().unwrap_or(&ir::Id::from(""))
                            == "std_reg"
                        {
                            let name = format!("{}{}", INVOKE_PREFIX, counter);
                            invoke.attributes.insert(INVOKE_PREFIX, *counter);
                            Some((
                                parent.name.clone(),
                                GroupOrInvoke::Invoke(ir::Id::from(name)),
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });

            let mut new_reach = reach;
            new_reach.set.extend(iterator);

            invoke.inputs = inputs;
            invoke.outputs = outputs;

            (new_reach, killed)
        }
        ir::Control::Enable(en) => {
            let writes =
                ReadWriteSet::must_write_set(&en.group.borrow().assignments);
            // for each write:
            // Killing all other reaching defns for that var
            // generating a new defn (Id, GROUP)
            let write_set = writes
                .iter()
                .filter(|&x| match &x.borrow().prototype {
                    ir::CellType::Primitive { name, .. } => name == "std_reg",
                    _ => false,
                })
                .map(|x| x.borrow().name.clone())
                .collect::<BTreeSet<_>>();

            let read_set =
                ReadWriteSet::register_reads(&en.group.borrow().assignments)
                    .iter()
                    .map(|x| x.borrow().name.clone())
                    .collect::<BTreeSet<_>>();
            // only kill a def if the value is not read.
            let (mut cur_reach, killed) =
                reach.kill_from_writeread(&write_set, &read_set);
            cur_reach.extend(write_set, &en.group.borrow().name);

            rd.reach.insert(
                GroupOrInvoke::Group(en.group.borrow().name.clone()),
                cur_reach.clone(),
            );

            (cur_reach, killed)
        }
        ir::Control::Empty(_) => (reach, killed),
    }
}
