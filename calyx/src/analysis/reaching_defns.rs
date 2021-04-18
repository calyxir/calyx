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

    fn empty() -> Self {
        DefSet {
            set: HashSet::new(),
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

impl Sub<&HashSet<ir::Id>> for &DefSet {
    type Output = DefSet;

    fn sub(self, rhs: &HashSet<ir::Id>) -> Self::Output {
        DefSet {
            set: self
                .set
                .iter()
                .cloned()
                .filter(|(name, _)| !rhs.contains(name))
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct ReachingDefinitionAnalysis {
    pub reach: HashMap<GroupName, DefSet>,
}

impl ReachingDefinitionAnalysis {
    pub fn new(_comp: &ir::Component, control: &ir::Control) -> Self {
        let initial_set = DefSet::empty();
        let mut analysis = ReachingDefinitionAnalysis::empty();

        build_reaching_def(control, initial_set, &mut analysis);
        analysis
    }

    fn empty() -> Self {
        ReachingDefinitionAnalysis {
            reach: HashMap::new(),
        }
    }

    pub fn calculate_overlap(
        &self,
    ) -> HashMap<ir::Id, Vec<HashSet<(ir::Id, GroupName)>>> {
        let mut overlap_map: HashMap<
            ir::Id,
            Vec<HashSet<(ir::Id, GroupName)>>,
        > = HashMap::new();
        for defset in self.reach.values() {
            let mut group_overlaps: HashMap<
                ir::Id,
                HashSet<(ir::Id, GroupName)>,
            > = HashMap::new();

            for (defname, group_name) in &defset.set {
                let set = group_overlaps.entry(defname.clone()).or_default();
                set.insert((defname.clone(), group_name.clone()));
            }

            for (defname, set) in group_overlaps.drain() {
                let overlap_vec = overlap_map.entry(defname).or_default();
                if overlap_vec.is_empty() {
                    overlap_vec.push(set)
                } else {
                    let mut found = false;
                    for entry in overlap_vec.iter_mut() {
                        if !set.is_disjoint(entry) {
                            *entry = &set | entry;
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        overlap_vec.push(set);
                    }
                }
            }
        }
        overlap_map
    }
}

fn build_reaching_def(
    c: &ir::Control,
    reach: DefSet,
    rd: &mut ReachingDefinitionAnalysis,
) -> DefSet {
    match c {
        ir::Control::Seq(ir::Seq { stmts, .. }) => stmts
            .iter()
            .fold(reach, |acc, inner_c| build_reaching_def(inner_c, acc, rd)),
        ir::Control::Par(_) => {
            todo!()
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            let t_case = build_reaching_def(tbranch, reach.clone(), rd);
            let f_case = build_reaching_def(fbranch, reach, rd);
            &t_case | &f_case
        }
        ir::Control::While(_) => {
            todo!()
        }
        ir::Control::Invoke(_) => {
            todo!()
        }
        ir::Control::Enable(en) => {
            let writes =
                ReadWriteSet::write_set(&en.group.borrow().assignments);
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

            let mut cur_reach = &reach - &write_set;
            cur_reach.extend(write_set, &en.group.borrow().name);

            rd.reach
                .insert(en.group.borrow().name.clone(), cur_reach.clone());

            cur_reach
        }
        ir::Control::Empty(_) => reach,
    }
}
