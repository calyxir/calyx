use crate::analysis::reaching_defns::{
    GroupOrInvoke, ReachingDefinitionAnalysis,
};
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, Builder, Cell, Group, LibrarySignatures, RRC};
use std::collections::HashMap;

#[derive(Default)]
pub struct RegisterUnsharing {
    // This is an option because it cannot be initialized until the component is
    // seen
    bookkeeper: Option<Bookkeeper>,
}

impl Named for RegisterUnsharing {
    fn name() -> &'static str {
        "register-unsharing"
    }

    fn description() -> &'static str {
        "Split apart shared values into separate regsters"
    }
}

type RewriteMap<T> = HashMap<T, Vec<(RRC<Cell>, RRC<Cell>)>>;

// A struct for managing the overlapping analysis and rewrite information
struct Bookkeeper {
    analysis: ReachingDefinitionAnalysis,
    widths: HashMap<ir::Id, u64>,
    group_map: HashMap<ir::Id, RRC<Group>>,
    cell_map: HashMap<ir::Id, RRC<Cell>>,
    invoke_map: RewriteMap<ir::Id>,
}

impl Bookkeeper {
    // Create a new bookkeeper from the component
    fn new(comp: &ir::Component) -> Self {
        // width map is needed to create new registers with the proper widths
        let widths = comp
            .cells
            .iter()
            .filter_map(|c| {
                if let ir::CellType::Primitive { name, .. } =
                    &c.borrow().prototype
                {
                    if name == "std_reg" {
                        if let Some(in_port) = c.borrow().find("in") {
                            return Some((
                                c.borrow().name.clone(),
                                in_port.borrow().width,
                            ));
                        }
                    }
                }
                None
            })
            .collect();

        let analysis = ReachingDefinitionAnalysis::new(
            &comp,
            &mut comp.control.borrow_mut(),
        );

        // Used to amortize access to cells and groups that will be needed for
        // rewriting
        let group_map = HashMap::new();
        let cell_map = HashMap::new();
        let invoke_map = HashMap::new();

        Self {
            analysis,
            widths,
            group_map,
            cell_map,
            invoke_map,
        }
    }

    fn create_new_regs(
        &mut self,
        builder: &mut Builder,
    ) -> Vec<(ir::Id, ir::Id, Vec<GroupOrInvoke>)> {
        let overlap = self
            .analysis
            .calculate_overlap(&builder.component.continuous_assignments);

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
            let group = builder.component.find_group(group_name).unwrap();
            self.group_map.insert(group_name.clone(), group.clone());
            group
        }
    }

    fn rename(
        &mut self,
        builder: &mut Builder,
        rename_list: &[(ir::Id, ir::Id, Vec<GroupOrInvoke>)],
    ) {
        let mut grp_map: RewriteMap<&ir::Id> = HashMap::new();
        let mut invoke_map: RewriteMap<ir::Id> = HashMap::new();
        for (new_name, old_name, grouplist) in rename_list {
            for group_or_invoke in grouplist {
                match group_or_invoke {
                    GroupOrInvoke::Group(group) => {
                        grp_map.entry(group).or_default().push((
                            self.get_cell(builder, old_name),
                            self.get_cell(builder, new_name),
                        ))
                    }
                    GroupOrInvoke::Invoke(invoke) => {
                        invoke_map.entry(invoke.clone()).or_default().push((
                            self.get_cell(builder, old_name),
                            self.get_cell(builder, new_name),
                        ))
                    }
                }
            }
        }

        for (grp, rename_cells) in grp_map {
            let group = self.get_group(builder, grp);
            let mut group_ref = group.borrow_mut();
            builder.rename_port_uses(&rename_cells, &mut group_ref.assignments)
        }

        self.invoke_map = invoke_map;
    }
}

impl Visitor for RegisterUnsharing {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _c: &LibrarySignatures,
    ) -> VisResult {
        self.bookkeeper.replace(Bookkeeper::new(comp));
        let mut builder = Builder::from(comp, _c, false);

        let rename_list = self
            .bookkeeper
            .as_mut()
            .unwrap()
            .create_new_regs(&mut builder);

        self.bookkeeper
            .as_mut()
            .unwrap()
            .rename(&mut builder, &rename_list);

        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        invoke: &mut ir::Invoke,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        if let Some(name) =
            ReachingDefinitionAnalysis::extract_meta_name(invoke)
        {
            let vec_array =
                &self.bookkeeper.as_ref().unwrap().invoke_map.get(&name);

            // only do rewrites if there is actually rewriting to do
            if let Some(rename_vec) = vec_array {
                ReachingDefinitionAnalysis::replace_invoke_ports(
                    invoke, rename_vec,
                );
            }

            ReachingDefinitionAnalysis::clear_meta_name(invoke);
        }

        Ok(Action::Continue)
    }
}
