use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::Id;
use calyx_utils::WithPos;
use linked_hash_map::LinkedHashMap;
use std::fmt;
use std::path::PathBuf;

/// Metadata stores a Map between each group name and data used in the metadata table (specified in PR #2022)
#[derive(Default)]
pub struct Metadata {
    groups: LinkedHashMap<Id, ((usize, usize), PathBuf)>,
}

impl Metadata {
    /// Create an empty metadata table
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new entry to the metadata table
    fn add_entry(&mut self, name: Id, span: (usize, usize), path: PathBuf) {
        let ins = self.groups.insert(name, (span, path));
        if let Some(_v) = ins {
            panic!("Two of same group name found")
        }
    }
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let grps = &self.groups;

        for (name, ((start, end), file)) in grps {
            let file = file.to_str().unwrap();

            writeln!(f, "{name}: {file} {start}-{end}")?;
        }

        Ok(())
    }
}

impl Named for Metadata {
    fn name() -> &'static str {
        "metadata-table-generation"
    }
    fn description() -> &'static str {
        "generates metadata table for a file not containing one"
    }
}

impl Visitor for Metadata {
    //iterate over all groups in all components and collect metadata
    fn start_context(&mut self, ctx: &mut calyx_ir::Context) -> VisResult {
        if ctx.metadata.is_none() {
            let mut table = Metadata::new();
            for component in &ctx.components {
                let cmpt_iter = component.groups.into_iter();
                for rcc_grp in cmpt_iter {
                    let grp = rcc_grp.borrow_mut();
                    let pos_data = grp.attributes.copy_span();
                    let (file, span) = pos_data.get_line_num();
                    table.add_entry(grp.name(), span, PathBuf::from(file));
                }

                ctx.metadata = Some(table.to_string());
            }
        }
        Ok(Action::Stop)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use calyx_ir::Id;

    use crate::passes::metadata_table_gen::Metadata;
    #[test]
    fn test_metadata() {
        let mut data = Metadata::new();
        let empt_string = data.to_string();
        assert_eq!(empt_string, "");

        let path = PathBuf::from("/temp/path/for/testing.futil");
        data.add_entry(Id::from("group_1"), (12, 16), path.clone());
        data.add_entry(Id::from("group_2"), (23, 28), path);
        let test_string = data.to_string();
        assert_eq!(test_string, "group_1: /temp/path/for/testing.futil 12-16\ngroup_2: /temp/path/for/testing.futil 23-28\n")
    }
}
