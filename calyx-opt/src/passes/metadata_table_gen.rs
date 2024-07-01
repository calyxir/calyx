use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::Id;
use linked_hash_map::LinkedHashMap;
use std::path::PathBuf;

/// visit each group and collect necessary data (name and line num)
/// construct metadata table and append to file
struct Metadata {
    groups: LinkedHashMap<Id, (u32, PathBuf)>,
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
    pub fn to_string(&self) -> String {
        let grps = &self.groups;
        let mut text = String::from("metadata #{\n");
        for (name, (line_num, file)) in grps {
            let name = name.to_string();
            let file = file.to_str();
            let file = match file {
                None => "x",
                Some(f) => f,
            };
            let line = format!("    {name}: {file} {line_num}\n");
            text.push_str(line.as_str());
        }
        text.push_str("}#");
        text
    }
    /// Add a new entry to the metadata table
    pub fn add_entry(
        &mut self,
        name: Id,
        line: u32,
        path: PathBuf,
    ) -> &mut Self {
        let ins = self.groups.insert(name, (line, path));
        match ins {
            None => self,
            Some(v) => {
                println!("Found two of same group name");
                self
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use calyx_ir::Id;

    use crate::passes::metadata_table_gen::Metadata;
    #[test]
    fn test_metadata_string() {
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
