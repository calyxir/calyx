use crate::analysis::reaching_defns::{
    GroupOrInvoke, ReachingDefinitionAnalysis,
};
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, Builder, Cell, LibrarySignatures, RRC};
use std::{collections::HashMap, rc::Rc};

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
    invoke_map: RewriteMap<ir::Id>,
}

impl Bookkeeper {
    // Create a new bookkeeper from the component
    fn new(comp: &ir::Component) -> Self {
        // width map is needed to create new registers with the proper widths
        let widths = comp
            .iter_cells()
            .filter_map(|c| {
                if let ir::CellType::Primitive { name, .. } =
                    &c.borrow().prototype
                {
                    if name == "std_reg" {
                        if let Some(in_port) = c.borrow().find("in") {
                            return Some((
                                c.borrow().name().clone(),
                                in_port.borrow().width,
                            ));
                        }
                    }
                }
                None
            })
            .collect();

        let analysis =
            ReachingDefinitionAnalysis::new(&comp, &comp.control.borrow());

        let invoke_map = HashMap::new();

        Self {
            analysis,
            widths,
            invoke_map,
        }
    }
    /// This method takes the reaching definition analysis and uses it to
    /// determine the set of of overlapping definitions for each register.
    /// Registers may be split into X registers where X is the number of sets in
    /// the overlap calculation for that register.
    ///
    /// For registers with more than one set (i.e. those which have
    /// non-overlapping subsets of definitions) this method generates a new
    /// register name, creates the new register, and associates the new name and
    /// old name with a vector of location ids (group/invoke stmt). This tuple
    /// can then be used to rewrite the old name into the new name in the
    /// corresponding locations.
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
                        .name()
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
                            builder.component.find_cell(old_name).unwrap(),
                            builder.component.find_cell(new_name).unwrap(),
                        ))
                    }
                    GroupOrInvoke::Invoke(invoke) => {
                        invoke_map.entry(invoke.clone()).or_default().push((
                            builder.component.find_cell(old_name).unwrap(),
                            builder.component.find_cell(new_name).unwrap(),
                        ))
                    }
                }
            }
        }

        for (grp, rename_cells) in grp_map {
            let group = builder.component.find_group(grp).unwrap();
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
        if let Some(name) = self
            .bookkeeper
            .as_ref()
            .unwrap()
            .analysis
            .meta
            .fetch_label(invoke)
        {
            let vec_array =
                &self.bookkeeper.as_ref().unwrap().invoke_map.get(&name);

            // only do rewrites if there is actually rewriting to do
            if let Some(rename_vec) = vec_array {
                replace_invoke_ports(invoke, rename_vec);
            }
        }

        Ok(Action::Continue)
    }
}

fn replace_invoke_ports(
    invoke: &mut ir::Invoke,
    rewrites: &[(RRC<ir::Cell>, RRC<ir::Cell>)],
) {
    let parent_matches = |port: &RRC<ir::Port>, cell: &RRC<ir::Cell>| -> bool {
        if let ir::PortParent::Cell(cell_wref) = &port.borrow().parent {
            Rc::ptr_eq(&cell_wref.upgrade(), cell)
        } else {
            false
        }
    };

    let get_port =
        |port: &RRC<ir::Port>, cell: &RRC<ir::Cell>| -> RRC<ir::Port> {
            Rc::clone(&cell.borrow().get(&port.borrow().name))
        };

    for (_name, port) in
        invoke.inputs.iter_mut().chain(invoke.outputs.iter_mut())
    {
        if let Some((_old, new)) = rewrites
            .iter()
            .find(|&(cell, _)| parent_matches(port, cell))
        {
            *port = get_port(port, new)
        }
    }
}
