use std::{
    collections::{HashMap, HashSet},
    ops::{BitOr, Sub},
    rc::Rc,
};

use crate::analysis::ReadWriteSet;
use crate::ir;

type GroupName = ir::Id;

#[derive(Clone, Debug)]
pub struct DefSet {
    set: HashSet<(ir::Id, GroupName)>,
}

impl DefSet {
    fn extend(&mut self, writes: HashSet<ir::Id>, grp: &GroupName) {
        for var in writes {
            self.set.insert((var, grp.clone()));
        }
    }

    fn new() -> Self {
        DefSet {
            set: HashSet::new(),
        }
    }

    fn kill_from_writeread(
        &self,
        writes: &HashSet<ir::Id>,
        reads: &HashSet<ir::Id>,
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

    fn kill_from_hashset(&self, killset: &HashSet<ir::Id>) -> Self {
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

type OverlapMap = HashMap<ir::Id, Vec<HashSet<(ir::Id, GroupName)>>>;

#[derive(Debug)]
pub struct ReachingDefinitionAnalysis {
    pub reach: HashMap<GroupName, DefSet>,
}

impl ReachingDefinitionAnalysis {
    pub fn new(_comp: &ir::Component, control: &ir::Control) -> Self {
        let initial_set = DefSet::new();
        let mut analysis = ReachingDefinitionAnalysis::empty();

        build_reaching_def(
            control,
            initial_set,
            KilledSet::new(),
            &mut analysis,
        );
        analysis
    }

    fn empty() -> Self {
        ReachingDefinitionAnalysis {
            reach: HashMap::new(),
        }
    }

    pub fn calculate_overlap(&self) -> OverlapMap {
        let mut overlap_map: HashMap<
            ir::Id,
            Vec<HashSet<(ir::Id, GroupName)>>,
        > = HashMap::new();
        for (grp, defset) in &self.reach {
            let mut group_overlaps: HashMap<
                ir::Id,
                HashSet<(ir::Id, GroupName)>,
            > = HashMap::new();

            for (defname, group_name) in &defset.set {
                let set = group_overlaps.entry(defname.clone()).or_default();
                set.insert((defname.clone(), group_name.clone()));
                set.insert((defname.clone(), grp.clone()));
            }

            for (defname, set) in group_overlaps {
                let overlap_vec = overlap_map.entry(defname).or_default();

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

type KilledSet = HashSet<ir::Id>;

fn build_reaching_def(
    c: &ir::Control,
    reach: DefSet,
    killed: KilledSet,
    rd: &mut ReachingDefinitionAnalysis,
) -> (DefSet, KilledSet) {
    match c {
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            stmts
                .iter()
                .fold((reach, killed), |(acc, killed), inner_c| {
                    build_reaching_def(inner_c, acc, killed, rd)
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
                .fold(DefSet::new(), |acc, element| &acc | &element);

            (par_exit_defs, &global_killed | &killed)
        }
        ir::Control::If(ir::If {
            tbranch,
            fbranch,
            cond,
            ..
        }) => {
            let fake_enable = ir::Control::Enable(ir::Enable {
                attributes: ir::Attributes::default(),
                group: Rc::clone(cond),
            });
            let (post_cond_def, post_cond_killed) =
                build_reaching_def(&fake_enable, reach, killed, rd);
            let (t_case_def, t_case_killed) = build_reaching_def(
                tbranch,
                post_cond_def.clone(),
                post_cond_killed.clone(),
                rd,
            );
            let (f_case_def, f_case_killed) = build_reaching_def(
                fbranch,
                post_cond_def,
                post_cond_killed,
                rd,
            );
            (&t_case_def | &f_case_def, &t_case_killed | &f_case_killed)
        }
        ir::Control::While(ir::While { cond, body, .. }) => {
            let fake_enable = ir::Control::Enable(ir::Enable {
                attributes: ir::Attributes::default(),
                group: Rc::clone(cond),
            });
            let (post_cond_def, post_cond_killed) =
                build_reaching_def(&fake_enable, reach, killed, rd);

            let (round_1_def, round_1_killed) =
                build_reaching_def(body, post_cond_def, post_cond_killed, rd);
            let (post_cond2_def, post_cond2_killed) = build_reaching_def(
                &fake_enable,
                round_1_def,
                round_1_killed,
                rd,
            );
            // Twice as nice?
            build_reaching_def(body, post_cond2_def, post_cond2_killed, rd)
        }
        ir::Control::Invoke(_) => {
            todo!()
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
                .collect::<HashSet<_>>();

            let read_set =
                ReadWriteSet::register_reads(&en.group.borrow().assignments)
                    .iter()
                    .map(|x| x.borrow().name.clone())
                    .collect::<HashSet<_>>();
            // only kill a def if the value is not read.
            let (mut cur_reach, killed) =
                reach.kill_from_writeread(&write_set, &read_set);
            cur_reach.extend(write_set, &en.group.borrow().name);

            rd.reach
                .insert(en.group.borrow().name.clone(), cur_reach.clone());

            (cur_reach, killed)
        }
        ir::Control::Empty(_) => (reach, killed),
    }
}
