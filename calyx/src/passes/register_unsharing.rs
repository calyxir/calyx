//! Pass to unshare registers by analyzing the live ranges of values stored
//! within them.
use crate::analysis::reaching_defns::{
    GroupOrInvoke, ReachingDefinitionAnalysis,
};
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, Builder, Cell, CloneName, LibrarySignatures, RRC};
use std::collections::HashMap;

/// Unsharing registers reduces the amount of multiplexers used in the final design, trading them
/// off for more memory.
///
/// A register use is said to be shared if it is used to store multiple, non-overlapping values in
/// it. Unsharing, then, is the process of identifying such usages of registers and generating
/// new registers to store non-overlapping values. For example, the following program:
///
/// ```
/// let x = 1;
/// x = x + 2;
/// x = x + 3
/// ```
///
/// Can be rewritten as:
/// ```
/// let x = 1;
/// let y = x + 2;
/// let z = y + 3;
/// ```
///
/// On the other hand, the following use of a register cannot be unshared:
/// ```
/// let x = 0;
/// for i in 0..10 {
///   x = x + 1;
/// }
/// ```
#[derive(Default)]
pub struct RegisterUnsharing {
    bookkeeper: Bookkeeper,
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
#[derive(Default)]
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

        Self {
            widths,
            analysis: ReachingDefinitionAnalysis::new(&comp.control.borrow()),
            invoke_map: HashMap::new(),
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
        comp: &mut ir::Component,
        rename_list: &[(ir::Id, ir::Id, Vec<GroupOrInvoke>)],
    ) {
        let mut grp_map: RewriteMap<&ir::Id> = HashMap::new();
        let mut invoke_map: RewriteMap<ir::Id> = HashMap::new();
        for (new_name, old_name, grouplist) in rename_list {
            for group_or_invoke in grouplist {
                let name = old_name.clone();
                let cell = comp.find_cell(new_name).unwrap();
                match group_or_invoke {
                    GroupOrInvoke::Group(group) => {
                        grp_map.entry(group).or_default().insert(name, cell);
                    }
                    GroupOrInvoke::Invoke(invoke) => {
                        invoke_map
                            .entry(invoke.clone())
                            .or_default()
                            .insert(name, cell);
                    }
                }
            }
        }

        for (grp, rename_cells) in grp_map {
            let group_ref = comp.find_group(grp).unwrap();
            let mut group = group_ref.borrow_mut();
            let empty_map = HashMap::new();
            let rewriter =
                ir::rewriter::PortRewrite::new(&rename_cells, &empty_map);
            group
                .assignments
                .iter_mut()
                .for_each(|assign| assign.for_each_port(|p| rewriter.get(p)));
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
        self.bookkeeper = Bookkeeper::new(comp);
        let mut builder = Builder::new(comp, sigs);
        // Build a rename list
        let rename_list = self.bookkeeper.create_new_regs(&mut builder);
        // Perform the structural renaming.
        self.bookkeeper.rename(comp, &rename_list);
        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        invoke: &mut ir::Invoke,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let book = &self.bookkeeper;

        if let Some(name) = book.analysis.meta.fetch_label(invoke) {
            // only do rewrites if there is actually rewriting to do
            if let Some(rename_vec) = book.invoke_map.get(name) {
                let empty_map = HashMap::new();
                let rewriter =
                    ir::rewriter::PortRewrite::new(rename_vec, &empty_map);
                ir::rewriter::Rewriter::rewrite_invoke(
                    invoke,
                    &rewriter,
                    &HashMap::new(),
                );
            }
        }

        Ok(Action::Continue)
    }
}
