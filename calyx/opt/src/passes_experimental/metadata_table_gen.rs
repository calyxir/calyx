use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::source_info::{FileId, LineNum, PositionId, SourceInfoTable};
use calyx_ir::Id;
use calyx_utils::WithPos;
use linked_hash_map::LinkedHashMap;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// Metadata stores a Map between each group name and data used in the metadata table (specified in PR #2022)
pub struct Metadata {
    //groups: LinkedHashMap<(Id, Id), ((usize, usize), PathBuf)>,
    src_table: SourceInfoTable,
    file_ids: HashMap<String, FileId>,
}

impl Metadata {
    /// Create an empty metadata table
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            src_table: SourceInfoTable::new_empty(),
            file_ids: HashMap::new(),
        }
    }

    // /// Add a new entry to the metadata table
    // fn add_entry(
    //     &mut self,
    //     comp_name: Id,
    //     name: Id,
    //     span: (usize, usize),
    //     path: PathBuf,
    // ) {
    //     let ins = self.groups.insert((comp_name, name), (span, path));
    //     if let Some(_v) = ins {
    //         panic!("Two of same group name found")
    //     }
    // }
}

// impl fmt::Display for Metadata {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let grps = &self.groups;

//         for ((comp, name), ((start, end), file)) in grps {
//             let file = file.to_str().unwrap();

//             writeln!(f, "{comp}.{name}: {file} {start}-{end}")?;
//         }

//         // display source info table

//         Ok(())
//     }
// }

impl Named for Metadata {
    fn name() -> &'static str {
        "metadata-table-generation"
    }
    fn description() -> &'static str {
        "generates metadata table for a file not containing one"
    }
}

impl ConstructVisitor for Metadata {
    fn from(_ctx: &calyx_ir::Context) -> calyx_utils::CalyxResult<Self>
    where
        Self: Sized,
    {
        Ok(Self::new())
    }

    fn clear_data(&mut self) {
        // preserve across components
        // hacky oops
    }
}
impl Visitor for Metadata {
    //iterate over all groups in all components and collect metadata
    // this just needs to create source table in Metadata
    // fn start_context(&mut self, ctx: &mut calyx_ir::Context) -> VisResult {
    //     if ctx.metadata.is_none() {
    //         let mut table = Metadata::new();
    //         for component in &ctx.components {
    //             let cmpt_iter = component.groups.into_iter();
    //             for rcc_grp in cmpt_iter {
    //                 let grp = rcc_grp.borrow_mut();
    //                 let pos_data = grp.attributes.copy_span();
    //                 let (file, span) = pos_data.get_line_num();
    //                 table.add_entry(
    //                     component.name,
    //                     grp.name(),
    //                     span,
    //                     PathBuf::from(file),
    //                 );
    //                 // hm may need to actually use the full name of the group
    //             }

    //             ctx.metadata = Some(table.to_string());
    //         }
    //         // to generate the table:
    //         let mut new_src_table: SourceInfoTable =
    //             match &ctx.source_info_table {
    //                 None => SourceInfoTable::new_empty(),
    //                 Some(s) => s.clone(),
    //             };

    //         // add this file to the table
    //         // longass train of calls that inlines what i do above to get file name from component span
    //         let temp0 = &ctx.components[0]
    //             .groups
    //             .into_iter()
    //             .next()
    //             .unwrap()
    //             .borrow_mut()
    //             .attributes
    //             .copy_span();
    //         let (file, _span) = temp0.get_line_num();
    //         let id = new_src_table.push_file(PathBuf::from(file));
    //         self.src_table = new_src_table;
    //         self.file_ids.insert(String::from(file), id);
    //         //update this -> table needs its own to_string
    //         // may need to add display method to source info table (source_info.rs)
    //     }
    //     Ok(Action::Stop)
    // }

    // this visits each component
    fn start(
        &mut self,
        comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        // implement visiting each component and adding the positions
        // how this works:
        // components have attributes !!
        // check if file exists for component definition
        let binding = comp.attributes.copy_span();
        let (file, _bs) = binding.get_line_num();

        // add file to source table (if not already in)
        if !self.file_ids.contains_key(file) {
            let id = self.src_table.push_file(PathBuf::from(file));
            self.file_ids.insert(String::from(file), id);
            dbg!(&self.src_table);
        }
        // visit all groups in component
        comp.groups.iter().for_each(|rrcgrp| {
            let grp = rrcgrp.borrow_mut();
            let pos_data = grp.attributes.copy_span();
            let (f, span) = pos_data.get_line_num();
            let fid = self.file_ids.get(f).unwrap(); // this def should be in file_ids
            let _temp = self
                .src_table
                .push_position(*fid, LineNum::new(span.0 as u32));
            //dbg!(&self.src_table);
        });
        //dbg!(&self.src_table);

        Ok(Action::Continue)
    }

    fn finish_context(&mut self, ctx: &mut calyx_ir::Context) -> VisResult {
        //dbg!(&self.src_table);
        ctx.source_info_table = Some(std::mem::take(&mut self.src_table));
        //dbg!(&ctx.source_info_table);
        Ok(Action::Continue)
    }

    // generic method helper

    // start seq
    fn start_seq(
        &mut self,
        s: &mut calyx_ir::Seq,
        comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let binding = comp.attributes.copy_span();
        let (file, _bs) = binding.get_line_num();

        // add file to source table (if not already in)
        if !self.file_ids.contains_key(file) {
            print!("no way")
        }
        let fnum = self.file_ids.get(file).unwrap();
        let line = s.attributes.copy_span().get_line_num();
        Ok(Action::Continue)
    }

    // start par

    // after groups are done: implement visit trait for each control node to add a position tag to it
    // get the line info from span and add position, add posId to node attributes (insert set)
}

// #[cfg(test)]
// mod tests {
//     use std::path::PathBuf;

//     use calyx_ir::Id;

//     use crate::passes_experimental::metadata_table_gen::Metadata;
//     #[test]
//     fn test_metadata() {
//         let mut data = Metadata::new();
//         let empt_string = data.to_string();
//         assert_eq!(empt_string, "");

//         let path = PathBuf::from("/temp/path/for/testing.futil");
//         data.add_entry(
//             Id::from("main"),
//             Id::from("group_1"),
//             (12, 16),
//             path.clone(),
//         );
//         data.add_entry(Id::from("main"), Id::from("group_2"), (23, 28), path);
//         let test_string = data.to_string();
//         assert_eq!(test_string, "main.group_1: /temp/path/for/testing.futil 12-16\nmain.group_2: /temp/path/for/testing.futil 23-28\n")
//     }
// }
