//! This module contains the data structures used by the debugger for source mapping
use std::{collections::HashMap, fs, path::PathBuf};

use calyx_ir::source_info::SourceInfoTable;

use crate::{errors::CiderResult, flatten::structures::context::Context};

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub struct NamedTag(u64, String);

impl NamedTag {
    pub fn new_nameless(tag: u64) -> Self {
        Self(tag, String::new())
    }
}

impl From<(u64, String)> for NamedTag {
    fn from(i: (u64, String)) -> Self {
        Self(i.0, i.1)
    }
}

/// GroupContents contains the file path of the group and the line numbers the
/// group is on.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupContents {
    pub path: String,
    pub start_line: u64,
    pub end_line: u64,
}
/// first item is group name and second is component name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupName {
    pub group: String,
    pub component: String,
}
/// impl struct with path and number
#[derive(Debug, Clone)]
/// NewSourceMap contains the group name as the key and the line it lies on with
///  as respect to its corresponding .futil file
pub struct NewSourceMap(HashMap<GroupName, GroupContents>);

impl NewSourceMap {
    pub fn generate_from_source_info(
        table: &SourceInfoTable,
        ctx: &Context,
    ) -> CiderResult<NewSourceMap> {
        let mut map: HashMap<GroupName, GroupContents> = HashMap::new();
        // iterate over groups
        for (g_idx, group) in ctx.primary.groups.iter() {
            // i'm assuming that multiple position tags would just reference the same place, so we just take the first
            let pos_data = table.lookup_position(
                *group
                    .pos
                    .as_ref()
                    .expect("group pos array is not")
                    .first()
                    .expect("group pos has no positions"),
            ); // unwrapping is fine the pos should be there afaik
            let path = table
                .lookup_file_path(pos_data.file)
                .to_str()
                .expect("file path is not valid UTF8")
                .to_string(); // check this is valid
            // get group name
            let grp_name = ctx.lookup_name(group.name());
            // get parent name
            let parent = ctx.get_component_from_group(g_idx);
            let parent_name = ctx.lookup_name(parent);
            let start_line = pos_data.line.into_inner().get() as u64;

            // i'm just assuming it has the end line, i need to make sure that's a valid assumption
            map.insert(
                GroupName {
                    group: grp_name.clone(),
                    component: parent_name.clone(),
                },
                GroupContents {
                    path,
                    start_line,
                    end_line: pos_data
                        .end_line
                        .map(|l| l.into_inner().get() as u64)
                        .unwrap_or(start_line), // if no end line then assumed to be one line
                },
            );
        }
        Ok(NewSourceMap(map))
    }
    /// look up group name, if not present, return None
    pub fn lookup(&self, key: &GroupName) -> Option<&GroupContents> {
        self.0.get(key)
    }

    pub fn lookup_line(&self, line_num: u64) -> Option<&GroupName> {
        self.0
            .iter()
            .find(|(_, v)| v.start_line == line_num)
            .map(|(k, _)| k)
    }
}

impl From<HashMap<GroupName, GroupContents>> for NewSourceMap {
    fn from(i: HashMap<GroupName, GroupContents>) -> Self {
        Self(i)
    }
}

pub struct SourceMap(HashMap<NamedTag, String>);

impl SourceMap {
    /// Lookup the source location for the given named tag. Tags for a specific
    /// named instance are looked for first, falling back to position tags with
    /// an empty name if nothing more specific is available
    pub fn lookup(&self, key: (u64, String)) -> Option<&String> {
        let key = key.into();

        self.0
            .get(&key)
            .or_else(|| self.0.get(&NamedTag(key.0, "".to_string())))
    }

    pub fn from_file(path: &Option<PathBuf>) -> CiderResult<Option<Self>> {
        if let Some(path) = path {
            let v = fs::read(path)?;
            let file_contents = std::str::from_utf8(&v)?;
            let map: Self =
                super::metadata_parser::parse_metadata(file_contents)?;
            Ok(Some(map))
        } else {
            Ok(None)
        }
    }

    pub fn from_string<S>(input: S) -> CiderResult<Self>
    where
        S: AsRef<str>,
    {
        super::metadata_parser::parse_metadata(input.as_ref())
    }
}

impl From<HashMap<NamedTag, String>> for SourceMap {
    fn from(i: HashMap<NamedTag, String>) -> Self {
        Self(i)
    }
}
