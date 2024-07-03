use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::Id;
use calyx_utils::WithPos;
use linked_hash_map::LinkedHashMap;
use std::fmt;
use std::path::PathBuf;

/// Metadata stores a Map between each group name and data used in the metadata table (specified in PR #2022)
#[derive(Default)]
pub struct Metadata {
    groups: LinkedHashMap<Id, (usize, PathBuf)>,
}

impl Metadata {
    /// Create an empty metadata table
    pub fn new() -> Self {
        let table = Metadata {
            groups: LinkedHashMap::new(),
        };
        table
    }
    /// Return this metadata table as a properly formatted string (see #2022 in git PRs)
    // fn to_string(&self) -> String {
    //     let grps = &self.groups;
    //     let mut text = String::new();
    //     for (name, (line_num, file)) in grps {
    //         let name = name.to_string();
    //         let file = file.to_str();
    //         let file = match file {
    //             None => "x",
    //             Some(f) => f,
    //         };
    //         let line = format!("    {name}: {file} {line_num}\n");
    //         text.push_str(line.as_str());
    //     }
    //     //text.push_str("}#");
    //     text
    // }
    /// Add a new entry to the metadata table
    fn add_entry(&mut self, name: Id, line: usize, path: PathBuf) -> &mut Self {
        let ins = self.groups.insert(name, (line, path));
        match ins {
            None => self,
            Some(_v) => {
                println!("Found two of same group name");
                self
            }
        }
    }
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let grps = &self.groups;
        let mut text = String::new();

        for (name, (line_num, file)) in grps {
            let file = file.to_str();
            let file = match file {
                None => "x",
                Some(f) => f,
            };
            text.push_str(format!("    {name}: {file} {line_num}\n").as_str());
        }

        write!(f, "{}", text)

        //this seems dumb
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
        match &ctx.metadata {
            None => {
                let mut table = Metadata::new();
                for component in &mut ctx.components {
                    let cmpt_iter = component.groups.into_iter();
                    for rcc_grp in cmpt_iter {
                        let grp = rcc_grp.borrow_mut();
                        let pos_data = grp.attributes.copy_span();
                        let (file, line_num) = pos_data.get_line_num();
                        table.add_entry(
                            grp.name(),
                            line_num,
                            PathBuf::from(file),
                        );
                    }
                }
                ctx.metadata = Some(table.to_string());
                Ok(Action::Stop)
            }
            Some(_data) => Ok(Action::Stop),
        }
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
        println!("empty metadata string: \n{empt_string}");

        let path = PathBuf::from("/temp/path/for/testing.futil");
        data.add_entry(Id::from("group_1"), 12, path.clone());
        data.add_entry(Id::from("group_2"), 23, path);
        let test_string = data.to_string();
        println!("added 2 metadata string:\n{test_string}")
    }
}
