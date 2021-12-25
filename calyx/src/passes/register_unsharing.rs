use crate::analysis::reaching_defns::{
    GroupOrInvoke, ReachingDefinitionAnalysis,
};
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, Builder, Cell, CloneName, LibrarySignatures, RRC};
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
        "Split apart shared values into separate registers"
    }
}

type RewriteMap<T> = HashMap<T, HashMap<ir::Id, RRC<Cell>>>;

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
            .cells
            .iter()
            .filter_map(|c| {
                if let ir::CellType::Primitive { name, .. } =
                    &c.borrow().prototype
                {
                    if name == "std_reg" {
                        if let Some(in_port) = c.borrow().find("in") {
                            return Some((
                                c.clone_name(),
                                in_port.borrow().width,
                            ));
                        }
                    }
                }
                None
            })
            .collect();

        let analysis = ReachingDefinitionAnalysis::new(&comp.control.borrow());

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
                        grp_map.entry(group).or_default().insert(
                            old_name.clone(),
                            builder.component.find_cell(new_name).unwrap(),
                        );
                    }
                    GroupOrInvoke::Invoke(invoke) => {
                        invoke_map.entry(invoke.clone()).or_default().insert(
                            old_name.clone(),
                            builder.component.find_cell(new_name).unwrap(),
                        );
                    }
                }
            }
        }

        for (grp, rename_cells) in grp_map {
            let group = builder.component.find_group(grp).unwrap();
            let mut group_ref = group.borrow_mut();
            ir::Rewriter::rename_cell_uses(
                &rename_cells,
                &mut group_ref.assignments,
            )
        }

        self.invoke_map = invoke_map;
    }
}

impl Visitor for RegisterUnsharing {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        self.bookkeeper.replace(Bookkeeper::new(comp));
        let mut builder = Builder::new(comp, sigs);

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
        _comps: &[ir::Component],
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
                &self.bookkeeper.as_ref().unwrap().invoke_map.get(name);

            // only do rewrites if there is actually rewriting to do
            if let Some(rename_vec) = vec_array {
                ir::Rewriter::rewrite_invoke(
                    invoke,
                    rename_vec,
                    &HashMap::new(),
                );
            }
        }

        Ok(Action::Continue)
    }
}
