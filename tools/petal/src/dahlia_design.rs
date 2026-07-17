use crate::adls::{Adl, AdlInfo, PosInfo, parse_adl_file};
use crate::design::{Design, GroupId};
use cranelift_entity::{PrimaryMap, entity_impl};
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct StatementId(u32);
entity_impl!(StatementId, "statement");

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct BlockId(u32);
entity_impl!(BlockId, "block");

struct Statement {
    line: String,
    line_num: u32,
    ancestors: Vec<BlockId>,
}
impl Statement {
    pub fn display_name(&self) {
        println!("L{:03}: {}", self.line_num, self.line);
    }
}

struct Block {
    line: String,
    invokes: Vec<InvokeTarget>,
    line_num: u32,
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

impl DahliaDesign {
    pub fn new(adl_filename: &str, d: &Design) -> Self {
        let mut out = Self {
            blocks: PrimaryMap::new(),
            statements: PrimaryMap::new(),
            groups_to_statement: FxHashMap::default(),
        };
        let group_names_to_ids = d.get_group_name_to_ids();

        let AdlInfo { adl, components } = parse_adl_file(adl_filename);
        assert_eq!(adl, Adl::Dahlia);
        // For now, we assume that all Dahlia programs are single-function. This assert will
        // break when a program defies this assumption
        assert_eq!(components.len(), 1);
        let main_component = &components[0];
        assert_eq!(main_component.component, "main");
        let info_to_statement: FxHashMap<(usize, String), StatementId> =
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
                let s = Statement {
                    line_num: *linenum as u32,
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
        out
    }
}
