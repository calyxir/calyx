use crate::analysis::GraphAnalysis;
use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures};
use calyx_utils::Error;
use std::collections::HashSet;

const READ_PORT: &str = "read_data";
const WRITE_PORT: &str = "write_data";

/// Pass to check common synthesis issues.
/// 1. If a memory is only read-from or written-to, synthesis tools will optimize it away. Add
///    @external attribute to the cell definition to make it an interface memory.
pub struct SynthesisPapercut {
    /// Names of memory primitives
    memories: HashSet<ir::Id>,
}

impl Default for SynthesisPapercut {
    fn default() -> Self {
        let memories =
            ["comb_mem_d1", "comb_mem_d2", "comb_mem_d3", "comb_mem_d4"]
                .iter()
                .map(|&mem| mem.into())
                .collect();
        SynthesisPapercut { memories }
    }
}

impl Named for SynthesisPapercut {
    fn name() -> &'static str {
        "synthesis-papercut"
    }

    fn description() -> &'static str {
        "Detect common problems when targeting synthesis backends"
    }
}

impl Visitor for SynthesisPapercut {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Get all the memory cells.
        let memory_cells = comp
            .cells
            .iter()
            .filter_map(|cell| {
                let cell = &cell.borrow();
                if let Some(ref parent) = cell.type_name() {
                    if self.memories.contains(parent) {
                        let has_external =
                            cell.get_attribute(ir::BoolAttr::External);
                        if has_external.is_none() {
                            return Some(cell.name());
                        }
                    }
                }
                None
            })
            .collect::<HashSet<_>>();

        // Early return if there are no memory cells.
        if memory_cells.is_empty() {
            return Ok(Action::Stop);
        }

        let has_mem_parent =
            |p: &ir::Port| memory_cells.contains(&p.get_parent_name());
        let analysis =
            GraphAnalysis::from(&*comp).edge_induced_subgraph(|p1, p2| {
                has_mem_parent(p1) || has_mem_parent(p2)
            });

        for mem in memory_cells {
            let cell = comp.find_cell(mem).unwrap();
            let read_port = cell.borrow().get(READ_PORT);
            if analysis.reads_from(&read_port.borrow()).next().is_none() {
                return Err(Error::papercut(
                    format!(
                        "Only writes performed on memory `{mem}'. Synthesis tools will remove this memory. Add @external to cell to turn this into an interface memory.",
                    ),
                ));
            }
            let write_port = cell.borrow().get(WRITE_PORT);
            if analysis.writes_to(&write_port.borrow()).next().is_none() {
                return Err(Error::papercut(
                    format!(
                        "Only reads performed on memory `{mem}'. Synthesis tools will remove this memory. Add @external to cell to turn this into an interface memory.",
                    ),
                ));
            }
        }
        Ok(Action::Stop)
    }
}
