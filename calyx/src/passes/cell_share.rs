use crate::errors::CalyxResult;
use crate::ir::rewriter;
use crate::{
    analysis::{
        GraphColoring, LiveRangeAnalysis, ReadWriteSet, ShareSet,
        StaticParTiming,
    },
    ir::{
        self,
        traversal::Named,
        traversal::{Action, ConstructVisitor, VisResult, Visitor},
        CloneName,
    },
};
use itertools::Itertools;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;

// function to turn cell types to string when we are building the json for
// share_freqs
fn cell_type_to_string(cell_type: &ir::CellType) -> String {
    match cell_type {
        ir::CellType::Primitive {
            name,
            param_binding,
            ..
        } => {
            let param_str = param_binding
                .iter()
                .map(|(id, val)| {
                    let mut id_str = id.to_string();
                    id_str.push('_');
                    id_str.push_str(&val.to_string());
                    id_str
                })
                .join("_");
            let mut name_str = name.to_string();
            name_str.push('_');
            name_str.push_str(&param_str);
            name_str
        }
        ir::CellType::Component { name } => name.to_string(),
        ir::CellType::ThisComponent => "ThisComponent".to_string(),
        ir::CellType::Constant { val, width } => {
            let mut s = "Const_".to_string();
            s.push_str(&val.to_string());
            s.push('_');
            s.push_str(&width.to_string());
            s
        }
    }
}

/// Given a [LiveRangeAnalysis] that specifies the "share" and "state_share" cells
/// alive at each group, minimizes the cells used for each component.
///
/// This works by constructing an interference graph for each alive "state_share" cell.
/// If two cells are ever alive at the same time, then there is an edge
/// between them in the interference graph. Additionally, if two cells
/// are different prototypes, then there is an edge between them.
///
/// A greedy graph coloring algorithm on the interference graph
/// is used to assign each cell a name.
///
/// By default, this pass will share a given cell as many times as possible. However,
/// by passing a command line argument, we can limit the number of times a given
/// cell is reused. The rationale behind this option is that, in certain cases,
/// if you share a given component too much, the logic to determine when that
/// component should be activated ends up being more expensive than just using
/// a separate component. To pass this command line argument, you give three numbers:
/// The number of times a given combinational component can be shared, the number
/// of times a given register can be shared, and the number of times all other
/// components can be shared. Generally we would want settings such that 1 < 2 < 3,
/// since a given share of a 3) would save more hardware than a share of a 2), and
/// a share of a 2) would save more hardware than a share of a 1).
/// The exact command line syntax to use: if we had a file, "x.futil" and ran:
/// `cargo run x.futil -x cell-share:bounds=2,4,8`, then we would only share a
/// given combinational component at most twice, a given register at most 4 times,
/// and all other components at most 8 times. If you wanted to do something with
/// fud then run `fud e ... -s futil.flags " -x cell-share:bounds=2,4,8"`. Finally
/// if you do not want to bound the sharing for a particular cell type,
/// you can pass -1 as a bound. So for example if you passed
/// `-x cell-share:bounds=2,-1,3` this means that you will always share registers.
/// Note: *The no spaces are important.*
/// Also, if you pass the following flag: `-x cell-share:print-share-freqs=file-name`
/// this pass will write a json to `file-name`. If want to print into stdout
/// then just set the file-name to be "stdout" (you don't need the quotes
/// when you actually pass in the argument, so run `-x cell-share:print-share-freqs=stdout`),
/// and if you want to print to stderr then just set the file-name to be "stderr".
/// The json will map an integer (say n) to the number of cells in the new design (i.e.,
/// the design after sharing has been performed) that were shared
/// exactly n times. So the key n = 2 will be mapped to the number of cells in the
/// new design that are shared exactly twice.
///
/// This pass only renames uses of cells. [crate::passes::DeadCellRemoval] should be run after this
/// to actually remove the definitions.
pub struct CellShare {
    live: LiveRangeAnalysis,
    rewrites: HashMap<ir::Id, ir::RRC<ir::Cell>>,
    /// Set of state shareable components (as type names)
    state_shareable: ShareSet,

    /// Set of shareable components (as type names)
    shareable: ShareSet,

    /// Cell active in continuous assignments, or ref cells (we want to ignore both)
    cont_ref_cells: HashSet<ir::Id>,

    /// The number of times a given class of cell can be shared. bounds should be
    /// length 3 to hold the 3 classes: comb cells, registers, and everything else
    bounds: Vec<Option<i64>>,

    /// maps the ids of groups to a set of tuples (i,j), the clock cycles (relative
    /// to the start of the par) that group is live
    par_timing_map: StaticParTiming,
    print_par_timing: bool,

    /// Maps cell types to the corresponding pdf. Each pdf is a hashmap which maps
    /// the number of times a given cell name reused (i.e., shared) to the
    /// number of cells that have been shared that many times times.
    share_freqs: HashMap<ir::Id, HashMap<ir::CellType, HashMap<i64, i64>>>,
    /// whether or not to print the share_freqs
    print_share_freqs: Option<String>,
}

impl Named for CellShare {
    fn name() -> &'static str {
        "cell-share"
    }
    fn description() -> &'static str {
        "use the fewest possible cells"
    }
}

impl ConstructVisitor for CellShare {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let state_shareable = ShareSet::from_context::<true>(ctx);
        let shareable = ShareSet::from_context::<false>(ctx);
        let (print_share_freqs, bounds, print_par_timing) =
            Self::parse_args(ctx);

        Ok(CellShare {
            live: LiveRangeAnalysis::default(),
            rewrites: HashMap::new(),
            cont_ref_cells: HashSet::new(),
            state_shareable,
            shareable,
            bounds,
            par_timing_map: StaticParTiming::default(),
            print_par_timing,
            share_freqs: HashMap::new(),
            print_share_freqs,
        })
    }

    fn clear_data(&mut self) {
        self.rewrites = HashMap::new();
        self.live = LiveRangeAnalysis::default();
        self.cont_ref_cells = HashSet::new();
    }
}

impl CellShare {
    fn initialize(
        &mut self,
        comp: &ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) {
        //add cont cells
        self.cont_ref_cells =
            ReadWriteSet::uses(comp.continuous_assignments.iter())
                .map(|cr| cr.borrow().clone_name())
                .collect();
        //add ref cells
        self.cont_ref_cells.extend(
            comp.cells
                .iter()
                .filter(|cell| cell.borrow().is_reference())
                .map(|cell| cell.borrow().clone_name()),
        );

        // TODO(rachit): Pass cont_ref_cells to LiveRangeAnalysis so that it ignores unneccessary
        // cells.
        self.live = LiveRangeAnalysis::new(
            &mut comp.control.borrow_mut(),
            self.state_shareable.clone(),
            self.shareable.clone(),
        );

        self.par_timing_map = StaticParTiming::new(
            &mut comp.control.borrow_mut(),
            comp.name,
            &self.live,
        );
        if self.print_par_timing {
            println!("{:?}", self.par_timing_map);
        }
    }

    fn cell_filter(&self, cell: &ir::Cell) -> bool {
        // Cells used in continuous assignments cannot be shared, nor can ref cells.
        if self.cont_ref_cells.contains(&cell.name()) {
            return false;
        }
        if let Some(ref name) = cell.type_name() {
            self.state_shareable.contains(name) || self.shareable.contains(name)
        } else {
            false
        }
    }

    // given a ctx, gets the bounds and the file to write the sharing frequencies
    // to. For example, if "-x cell-share:bounds=2,3,4"
    // is passed in the cmd line, we should return [2,3,4]. If no such argument
    // is given, return the default, which is [None, None, None]
    fn parse_args(ctx: &ir::Context) -> (Option<String>, Vec<Option<i64>>, bool)
    where
        Self: Named,
    {
        let n = Self::name();

        // getting the given opts for -x cell-share:__
        let given_opts: HashSet<_> = ctx
            .extra_opts
            .iter()
            .filter_map(|opt| {
                let mut splits = opt.split(':');
                if splits.next() == Some(n) {
                    splits.next()
                } else {
                    None
                }
            })
            .collect();

        let print_par_timing =
            given_opts.iter().any(|arg| *arg == "print_par_timing");

        // searching for "-x cell-share:bounds=x,y,z" and getting back "x,y,z"
        let bounds_arg = given_opts.iter().find_map(|arg| {
            let split: Vec<&str> = arg.split('=').collect();
            if let Some(str) = split.first() {
                if str == &"bounds" && split.len() == 2 {
                    return Some(split[1]);
                }
            }
            None
        });

        // searching for "-x cell-share:print-share-freqs=file_name" and getting Some(file_name) back
        let print_pdf_arg = given_opts.iter().find_map(|arg| {
            let split: Vec<&str> = arg.split('=').collect();
            if let Some(str) = split.first() {
                if str == &"print-share-freqs" && split.len() == 2 {
                    return Some(split[1].to_string());
                }
            }
            None
        });

        let mut bounds = Vec::new();
        let mut set_default = false;

        // if bounds_arg = "x,y,z", set bounds to [Some(x),Some(y),Some(z)]
        // a -1 argument means no bound since that means we always want to share
        if let Some(s) = bounds_arg {
            bounds = s
                .split(',')
                .map(|s| {
                    let val = s.parse::<i64>().unwrap_or(-2);
                    if val == -1 {
                        None
                    } else {
                        Some(val)
                    }
                })
                .collect();
        } else {
            set_default = true;
        }

        if bounds.len() != 3 || bounds.contains(&Some(-2)) {
            set_default = true;
        }

        if set_default {
            // could possibly put vec![x,y,z] where x,y, and z are deliberately
            // chosen numbers here instead
            (print_pdf_arg, vec![None, None, None], print_par_timing)
        } else {
            (print_pdf_arg, bounds, print_par_timing)
        }
    }

    // prints the json if self.print_share_freqs is not None
    fn print_share_json(&self) {
        if let Some(file) = &self.print_share_freqs {
            let printable_share_freqs: HashMap<String, HashMap<String, _>> =
                self.share_freqs
                    .iter()
                    .map(|(id, freq_map)| {
                        (
                            id.to_string(),
                            freq_map
                                .iter()
                                .map(|(cell_type, pdf)| {
                                    (cell_type_to_string(cell_type), pdf)
                                })
                                .collect(),
                        )
                    })
                    .collect();
            let json_share_freqs: Value = json!(printable_share_freqs);
            if file == "stdout" {
                println!("{json_share_freqs}");
            } else if file == "stderr" {
                eprintln!("{json_share_freqs}");
                std::process::exit(1);
            } else {
                fs::write(file, format!("{}", json_share_freqs))
                    .expect("unable to write file");
            }
        }
    }
}

/// The algorithm that runs is:
///  - instantiate conflict graph using all component cells that satisfy `cell_filter`
///  - use [ScheduleConflicts] to find groups/invokes that run in parallel with each other
///  - for each tuple combination of cells that return true on cell_filter(), c1 and c2
///  - first determine if their live ranges overlap. If so, then insert a conflict between
///  c1 and c2
///  - if c1 and c2 don't have overlapping live ranges, check if c1 and c2 are ever
///  live at within the same par block, and they are live at different children
///  of the par block, then add a conflict.
///  - perform graph coloring using `self.ordering` to define the order of the greedy coloring
///  - use coloring to rewrite group assignments, continuous assignments, and conditional ports.
impl Visitor for CellShare {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        self.initialize(comp, sigs);

        let cells = comp.cells.iter().filter(|c| self.cell_filter(&c.borrow()));

        // Mapping from cell type to names of all cells of that type.
        let mut cells_by_type: HashMap<ir::CellType, Vec<ir::Id>> =
            HashMap::new();
        for cell in cells {
            cells_by_type
                .entry(cell.borrow().prototype.clone())
                .or_default()
                .push(cell.clone_name());
        }

        // Maps cell type to conflict graph (will be used to perform coloring)
        let mut graphs_by_type: HashMap<ir::CellType, GraphColoring<ir::Id>> =
            cells_by_type
                .clone()
                .into_iter()
                .map(|(key, cell_names)| {
                    (key, GraphColoring::from(cell_names.into_iter()))
                })
                .collect();

        // We assume unique ids have already been computed by LiveRangeAnalysis

        // live_once_map maps celltypes to maps that map cells to control statements
        // in which the cell was live for at least one group/invoke. Furthermore,
        // only control statements that are direct children of par blocks
        // are included in this map.
        let mut live_once_map = HashMap::new();
        // Maps every control statement that is a direct child of a par block to
        // its parent par block. (maps id number to id number)
        let mut par_thread_map = HashMap::new();
        // Maps celltype to map that maps cells to groups/invokes in which the cell is live.
        let mut live_cell_map = HashMap::new();
        // build live_once_map and par_thread_map
        self.live.get_live_control_data(
            &mut live_once_map,
            &mut par_thread_map,
            &mut live_cell_map,
            &HashSet::new(),
            &comp.control.borrow(),
        );

        // Adding the conflicts
        for (cell_type, cells) in &cells_by_type {
            // Run remove_dead_cells before this cell-share pass.
            let g = graphs_by_type.get_mut(cell_type).unwrap();

            // mapping from cell names to the enables/invokes in which it is live
            let cell_to_nodes =
                live_cell_map.entry(cell_type.clone()).or_default();
            // mapping of cell names to the control statements in which it is live
            // at least once. Only control statements that are direct children of
            // par blocks are included
            let cell_to_control =
                live_once_map.entry(cell_type.clone()).or_default();
            for (a, b) in cells.iter().tuple_combinations() {
                // checking if live ranges overlap
                // nodes (groups/invokes) in which a is live
                if let Some(live_a) = cell_to_nodes.get(a) {
                    if let Some(live_b) = cell_to_nodes.get(b) {
                        if !live_a.is_disjoint(live_b) {
                            g.insert_conflict(a, b);
                            continue;
                        }
                    }
                }
                // checking if b is live at any groups/invokes running in parallel
                // to groups/invokes live at a
                // get the children of pars in which a was alive "at least once"
                if let Some(live_once_a) = cell_to_control.get(a) {
                    // get the children of pars in which b was alive "at least once"
                    if let Some(live_once_b) = cell_to_control.get(b) {
                        'outer: for live_a in live_once_a {
                            for live_b in live_once_b {
                                // a and b are live within the same par block but not within
                                // the same child thread, then insert conflict.
                                let parent_a =
                                    par_thread_map.get(live_a).unwrap();
                                let parent_b =
                                    par_thread_map.get(live_b).unwrap();
                                if live_a != live_b && parent_a == parent_b {
                                    if self.par_timing_map.liveness_overlaps(
                                        parent_a, live_a, live_b, a, b,
                                    ) {
                                        g.insert_conflict(a, b);
                                        break 'outer;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // perform graph coloring to rename the cells
        let mut coloring: rewriter::RewriteMap<ir::Cell> = HashMap::new();
        let mut comp_share_freqs: HashMap<ir::CellType, HashMap<i64, i64>> =
            HashMap::new();
        for (cell_type, mut graph) in graphs_by_type {
            // getting bound, based on self.bounds and cell_type
            let bound = {
                if let Some(ref name) = cell_type.get_name() {
                    let comb_bound = self.bounds.get(0).unwrap_or(&None);
                    let reg_bound = self.bounds.get(1).unwrap_or(&None);
                    let other_bound = self.bounds.get(2).unwrap_or(&None);
                    if self.shareable.contains(name) {
                        comb_bound
                    } else if name == "std_reg" {
                        reg_bound
                    } else {
                        other_bound
                    }
                } else {
                    &None
                }
            };
            if graph.has_nodes() {
                coloring.extend(
                    graph
                        .color_greedy(*bound)
                        .iter()
                        .map(|(a, b)| (*a, comp.find_cell(*b).unwrap())),
                );
                // only generate share-freqs if we're going to use them.
                if self.print_share_freqs.is_some() {
                    // must accumulate sharing numbers for share_freqs
                    comp_share_freqs.insert(cell_type, graph.get_share_freqs());
                }
            }
        }

        // add the sharing freqs for the component we just analyzed
        if self.print_share_freqs.is_some() {
            // must accumulate sharing numbers for share_freqs
            self.share_freqs.insert(comp.name, comp_share_freqs);
            // print share freqs json if self.print_share_freqs is not none
            self.print_share_json();
        }

        // Rewrite assignments using the coloring generated.
        let empty_map: ir::rewriter::PortRewriteMap = HashMap::new();
        let rewriter = ir::Rewriter::new(&coloring, &empty_map);
        comp.for_each_assignment(|assign| {
            assign.for_each_port(|port| rewriter.get(port));
        });

        // Rewrite control uses of ports
        rewriter.rewrite_control(
            &mut comp.control.borrow_mut(),
            &HashMap::new(),
            &HashMap::new(),
        );

        Ok(Action::Stop)
    }
}
