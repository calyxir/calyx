use crate::analysis::ReachingDefinitionAnalysis;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, Builder, Cell, Group, LibrarySignatures, RRC};
use crate::utils::NameGenerator;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct RegisterUnsharing {}

impl Named for RegisterUnsharing {
    fn name() -> &'static str {
        "register-unsharing"
    }

    fn description() -> &'static str {
        "Split apart shared values into separate regsters"
    }
}

struct BookKeeper {
    analysis: ReachingDefinitionAnalysis,
    widths: HashMap<ir::Id, u64>,
    group_map: HashMap<ir::Id, RRC<Group>>,
    cell_map: HashMap<ir::Id, RRC<Cell>>,
}

impl BookKeeper {
    fn new(comp: &ir::Component) -> Self {
        let widths = comp
            .cells
            .iter()
            .filter_map(|c| {
                if let ir::CellType::Primitive { name, .. } =
                    &c.borrow().prototype
                {
                    if name == "std_reg" {
                        if let Some(in_port) = c.borrow().find("in") {
                            Some((
                                c.borrow().name.clone(),
                                in_port.borrow().width,
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        // (griffin) I'm sorry for the above.
        // There's probably a cleaner way to write this
        // TODO(griffin): fix?

        let analysis =
            ReachingDefinitionAnalysis::new(&comp, &comp.control.borrow());
        let group_map = HashMap::new();
        let cell_map = HashMap::new();

        Self {
            analysis,
            widths,
            group_map,
            cell_map,
        }
    }

    fn create_new_regs(
        &mut self,
        builder: &mut Builder,
    ) -> Vec<(ir::Id, ir::Id, Vec<ir::Id>)> {
        let overlap = self.analysis.calculate_overlap();

        let mut rename_list = vec![];

        for (name, sets) in &overlap {
            if sets.len() > 1 {
                for defs in &sets[1..] {
                    let new_name = builder
                        .add_primitive(
                            format!("unshr_{}", name),
                            "std_reg",
                            &[*self.widths.get(name).unwrap()],
                        )
                        .borrow()
                        .name
                        .clone();
                    rename_list.push((
                        new_name.clone(),
                        name.clone(),
                        defs.iter()
                            .map(|(_, groupname)| groupname.clone())
                            .collect(),
                    ));
                }
            }
        }
        rename_list
    }

    fn get_cell(&mut self, builder: &Builder, cell_name: &ir::Id) -> RRC<Cell> {
        if self.cell_map.contains_key(cell_name) {
            self.cell_map.get(cell_name).unwrap().clone()
        } else {
            let cell = builder.component.find_cell(&cell_name.clone()).unwrap();
            self.cell_map.insert(cell_name.clone(), cell.clone());
            cell
        }
    }

    fn get_group(
        &mut self,
        builder: &Builder,
        group_name: &ir::Id,
    ) -> RRC<Group> {
        if self.group_map.contains_key(group_name) {
            self.group_map.get(group_name).unwrap().clone()
        } else {
            let group =
                builder.component.find_group(group_name).unwrap().clone();
            self.group_map.insert(group_name.clone(), group.clone());
            group
        }
    }

    fn rename(
        &mut self,
        builder: &mut Builder,
        rename_list: &[(ir::Id, ir::Id, Vec<ir::Id>)],
    ) {
        let mut grp_map: HashMap<&ir::Id, Vec<(RRC<Cell>, RRC<Cell>)>> =
            HashMap::new();
        for (new_name, old_name, grouplist) in rename_list {
            for group in grouplist {
                grp_map.entry(group).or_default().push((
                    self.get_cell(builder, old_name),
                    self.get_cell(builder, new_name),
                ))
            }
        }

        for (grp, rename_cells) in grp_map {
            let group = self.get_group(builder, grp);
            let mut group_ref = group.borrow_mut();
            builder.rename_port_uses(&rename_cells, &mut group_ref.assignments)
        }
    }
}

impl Visitor for RegisterUnsharing {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _c: &LibrarySignatures,
    ) -> VisResult {
        let mut bookkeeper = BookKeeper::new(comp);
        let mut builder = Builder::from(comp, _c, false);

        let rename_list = bookkeeper.create_new_regs(&mut builder);

        bookkeeper.rename(&mut builder, &rename_list);
        // for (group, z) in &bookkeeper.analysis.reach {
        //     println!("Group {}", group);
        //     println!("  {:?}", z);
        // }

        // for (x, y) in &bookkeeper.analysis.calculate_overlap() {
        //     println!("Overlapping defns for {}", x);
        //     for def in y {
        //         println!("   {:?}\n", def);
        //     }
        // }
        Ok(Action::Stop)
    }
}
