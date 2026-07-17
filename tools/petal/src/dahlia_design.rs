use crate::adls::{Adl, AdlInfo, PosInfo, parse_adl_file};
use crate::design::{Design, GroupId};
use anyhow::{Ok, Result};
use cranelift_entity::{PrimaryMap, entity_impl};
use rustc_hash::{FxHashMap, FxHashSet};
use std::fs::File;

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
    pub fn display_name(&self) {
        println!("L{:03}: {}", self.line_num, self.line);
    }
}

struct Block {
    line: String,
    line_num: u64,
}

impl Block {
    pub fn display_name(&self) {
        println!("BL{:03}: {}", self.line_num, self.line);
    }
}

enum InvokeTarget {
    Block(BlockId),
    Statement(StatementId),
}

/// (This structure follows what we have in `Design` for now)
struct DahliaDesign {
    blocks: PrimaryMap<BlockId, Block>,
    statements: PrimaryMap<StatementId, Statement>,
    groups_to_statement: FxHashMap<GroupId, StatementId>,
}

fn read_parent_map(
    parent_map_filename: &str,
) -> Result<(FxHashMap<u64, Vec<u64>>, FxHashSet<u64>)> {
    let parent_map_file = File::open(parent_map_filename)?;
    let parent_map_raw: FxHashMap<String, Vec<u64>> =
        serde_json::from_reader(parent_map_file)?;
    // reverse the parent map s.t.
    let mut all_blocks: FxHashSet<u64> = FxHashSet::default();
    let parent_map: FxHashMap<u64, Vec<u64>> = parent_map_raw
        .iter()
        .map(|(k, v)| {
            all_blocks.extend(v.iter().cloned());
            let mut rev = v.clone();
            rev.reverse();
            (k.parse().unwrap(), rev)
        })
        .collect();
    Ok((parent_map, all_blocks))
}

impl DahliaDesign {
    pub fn new(
        adl_filename: &str,
        parent_map_filename: &str,
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
        let AdlInfo { adl, components } = parse_adl_file(adl_filename)?;

        assert_eq!(adl, Adl::Dahlia);
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
            let stmt_id = if let Some(id) =
                info_to_statement.get(&(*linenum, (varname.clone())))
            {
                *id
            } else {
                // we haven't seen this line yet; if it's a block, we will add the block in as well.
                if all_block_lines.contains(linenum) {
                    let b = Block {
                        line_num: *linenum,
                        line: varname.clone(),
                    };
                    all_blocks.insert(linenum.clone(), out.blocks.push(b));
                }
                // register the statement
                let s = Statement {
                    line_num: *linenum,
                    line: varname.clone(),
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
        // for each statement, construct the ancestors vector
        Ok(out)
    }
}
