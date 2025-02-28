use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::source_info::{FileId, LineNum, PositionId, SourceInfoTable};
use calyx_ir::Id;
use calyx_utils::WithPos;
use linked_hash_map::LinkedHashMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// Metadata stores a Map between each group name and data used in the metadata table (specified in PR #2022)
#[derive(Default)]
pub struct Metadata {
    groups: LinkedHashMap<(Id, Id), ((usize, usize), PathBuf)>,
    src_table: SourceInfoTable,
}

impl Metadata {
    /// Create an empty metadata table
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new entry to the metadata table
    fn add_entry(
        &mut self,
        comp_name: Id,
        name: Id,
        span: (usize, usize),
        path: PathBuf,
    ) {
        let ins = self.groups.insert((comp_name, name), (span, path));
        if let Some(_v) = ins {
            panic!("Two of same group name found")
        }
    }
    /// Add a position entry
    fn add_position(&mut self, file: FileId, line: LineNum) {
        self.src_table.push_position(file, line);
    }
    /// Add a new file entry
    fn add_file(&mut self, path: PathBuf) {
        self.src_table.push_file(path);
    }
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let grps = &self.groups;

        for ((comp, name), ((start, end), file)) in grps {
            let file = file.to_str().unwrap();

            writeln!(f, "{comp}.{name}: {file} {start}-{end}")?;
        }

        // display source info table

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
                    table.add_entry(
                        component.name,
                        grp.name(),
                        span,
                        PathBuf::from(file),
                    );
                    // hm may need to actually use the full name of the group
                }

                ctx.metadata = Some(table.to_string());
            }
            // to generate the table:
            let mut src_table: SourceInfoTable = match &ctx.source_info_table {
                None => SourceInfoTable::new_empty(),
                Some(s) => s.clone(),
            };

            // add this file to the table
            // longass train of calls that inlines what i do above to get file name from component span
            let temp0 = &ctx.components[0]
                .groups
                .into_iter()
                .next()
                .unwrap()
                .borrow_mut()
                .attributes
                .copy_span();
            let (file, _span) = temp0.get_line_num();
            let id = src_table.push_file(PathBuf::from(file));
            // push all positions in the file to the pos table - use id

            //update this -> table needs its own to_string
            // may need to add display method to source info table (source_info.rs)
        }
        Ok(Action::Stop)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use calyx_ir::Id;

    use crate::passes_experimental::metadata_table_gen::Metadata;
    #[test]
    fn test_metadata() {
        let mut data = Metadata::new();
        let empt_string = data.to_string();
        assert_eq!(empt_string, "");

        let path = PathBuf::from("/temp/path/for/testing.futil");
        data.add_entry(
            Id::from("main"),
            Id::from("group_1"),
            (12, 16),
            path.clone(),
        );
        data.add_entry(Id::from("main"), Id::from("group_2"), (23, 28), path);
        let test_string = data.to_string();
        assert_eq!(test_string, "main.group_1: /temp/path/for/testing.futil 12-16\nmain.group_2: /temp/path/for/testing.futil 23-28\n")
    }
}
