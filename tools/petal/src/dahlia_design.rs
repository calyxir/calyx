use crate::adls::{ComponentInfo, PosInfo};
use crate::design::{Design, GroupId, Stack};
use crate::timeline::CurrentlyActive;
use crate::visuals::{compute_flame, write_flames};
use anyhow::{Ok, Result};
use cranelift_entity::{PrimaryMap, entity_impl};
use rustc_hash::{FxHashMap, FxHashSet};
use std::fs::File;
use std::path::PathBuf;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct StatementId(u32);
entity_impl!(StatementId, "statement");

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct BlockId(u32);
entity_impl!(BlockId, "block");

struct Statement {
    line: String,
    line_num: u64,
    ancestors: Vec<BlockId>,
}
impl Statement {
    pub fn display_name(&self) -> String {
        format!("L{:04}: {}", self.line_num, self.line)
    }
}

struct Block {
    line: String,
    line_num: u64,
}

impl Block {
    pub fn display_name(&self) -> String {
        format!("BL{:04}: {}", self.line_num, self.line)
    }
}

enum InvokeTarget {
    Block(BlockId),
    Statement(StatementId),
}

/// (This structure follows what we have in `Design` for now)
pub struct DahliaDesign {
    blocks: PrimaryMap<BlockId, Block>,
    statements: PrimaryMap<StatementId, Statement>,
    groups_to_statement: FxHashMap<GroupId, StatementId>,
}

impl DahliaDesign {
    pub fn new(
        components: Vec<ComponentInfo>,
        parent_map_filename: Option<String>,
        d: &Design,
    ) -> Result<Self> {
        let mut out = Self {
            blocks: PrimaryMap::new(),
            statements: PrimaryMap::new(),
            groups_to_statement: FxHashMap::default(),
        };
        let group_names_to_ids = d.get_group_name_to_ids();
        let (parent_map, all_block_lines) =
            read_parent_map(parent_map_filename)?;
        let mut all_blocks: FxHashMap<u64, BlockId> = FxHashMap::default();
        // let AdlInfo { adl, components } = parse_adl_file(adl_filename)?;

        // For now, we assume that all Dahlia programs are single-function. This assert will
        // break when a program defies this assumption
        assert_eq!(components.len(), 1);
        let main_component = &components[0];
        assert_eq!(main_component.component, "main");
        let info_to_statement: FxHashMap<(u64, String), StatementId> =
            FxHashMap::default();
        for PosInfo {
            name,
            linenum,
            varname,
            ..
        } in main_component.groups.iter()
        {
            // let line_contents = varname.split(" {").next().unwrap().to_string();
            let line_contents = varname
                .split("{")
                .next()
                .unwrap()
                .split(";")
                .next()
                .unwrap()
                .to_string();
            let stmt_id = if let Some(id) =
                info_to_statement.get(&(*linenum, (varname.clone())))
            {
                *id
            } else {
                // we haven't seen this line yet; if it's a block, we will add the block in as well.
                if all_block_lines.contains(linenum) {
                    let b = Block {
                        line_num: *linenum,
                        line: line_contents.clone(),
                    };
                    all_blocks.insert(linenum.clone(), out.blocks.push(b));
                }
                // register the statement
                let s = Statement {
                    line_num: *linenum,
                    line: line_contents.clone(),
                    ancestors: vec![],
                };
                out.statements.push(s)
            };
            let group_id_set = &group_names_to_ids[name];
            // Start by assuming that each group will only have one enable in control?
            assert_eq!(group_id_set.len(), 1);
            let group_id = group_id_set.iter().next().unwrap();
            out.groups_to_statement.insert(*group_id, stmt_id);
        }
        // for each statement, construct the list of ancestors
        // I think we need to do this later because parent blocks may be added later than the child stmt
        for (_id, stmt) in out.statements.iter_mut() {
            if let Some(ancestor_line_nums) = parent_map.get(&stmt.line_num) {
                let ancestors: Vec<BlockId> =
                    ancestor_line_nums.iter().map(|a| all_blocks[a]).collect();
                stmt.ancestors = ancestors;
            }
        }
        Ok(out)
    }

    pub fn compute_dahlia_trace(
        &self,
        calyx_active: &CurrentlyActive,
    ) -> Result<Vec<Stack>> {
        let mut out: Vec<Stack> = vec![];
        for active_group in calyx_active.get_active_groups() {
            if let Some(s_id) = self.groups_to_statement.get(active_group) {
                let mut stack: Vec<String> = vec![];
                // the statement in question is active.
                let statement: &Statement = &self.statements[*s_id];
                for b_id in statement.ancestors.iter() {
                    let block: &Block = &self.blocks[*b_id];
                    stack.push(block.display_name())
                }
                stack.push(statement.display_name());
                out.push(stack);
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }
}

pub struct DahliaProfilingInfo {
    design: DahliaDesign,
    /// Different Calyx traces can map onto the same Dahlia trace,
    /// so we will go with the most simple option for now
    trace_info: FxHashMap<Vec<Stack>, u64>,
}

impl DahliaProfilingInfo {
    pub fn new(
        components: Vec<ComponentInfo>,
        parent_file: Option<String>,
        d: &Design,
    ) -> Result<Self> {
        let design = DahliaDesign::new(components, parent_file, d)?;
        Ok(Self {
            design,
            trace_info: Default::default(),
        })
    }

    pub fn process_cycle(
        &mut self,
        calyx_active: &CurrentlyActive,
    ) -> Result<()> {
        let stack: Vec<Stack> =
            self.design.compute_dahlia_trace(calyx_active)?;
        let curr_count = self.trace_info.entry(stack).or_insert(0);
        *curr_count += 1;
        Ok(())
    }

    pub fn output_flame(&self, out_dir: &str) -> Result<()> {
        let flame_input: Vec<(&u64, &Vec<Stack>)> =
            self.trace_info.iter().map(|(s, c)| (c, s)).collect();
        let flame = compute_flame(flame_input)?;
        let mut scaled_flame = PathBuf::from(out_dir);
        scaled_flame.push("dahlia-scaled-flame.folded");
        let mut flat_flame = PathBuf::from(out_dir);
        flat_flame.push("dahlia-flat-flame.folded");
        write_flames(&flame, Some(scaled_flame), Some(flat_flame))?;
        Ok(())
    }
}

fn read_parent_map(
    parent_map_opt: Option<String>,
) -> Result<(FxHashMap<u64, Vec<u64>>, FxHashSet<u64>)> {
    if let Some(parent_map_filename) = parent_map_opt {
        let parent_map_file = File::open(parent_map_filename)?;
        let parent_map_raw: FxHashMap<String, Vec<u64>> =
            serde_json::from_reader(parent_map_file)?;
        // reverse the parent map s.t.
        let mut all_blocks: FxHashSet<u64> = FxHashSet::default();
        let mut parent_map: FxHashMap<u64, Vec<u64>> = parent_map_raw
            .iter()
            .map(|(k, v)| {
                let k_num: u64 = k.parse().unwrap();
                all_blocks.extend(v.iter().cloned());
                let mut rev = v.clone();
                rev.reverse();
                (k_num, rev)
            })
            .collect();
        for block in all_blocks.iter() {
            let v = parent_map.get_mut(block).unwrap();
            v.push(block.clone());
        }
        Ok((parent_map, all_blocks))
    } else {
        println!("[Dahlia profiling] Parent map not given!!!");
        Ok((FxHashMap::default(), FxHashSet::default()))
    }
}
