use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};

//use calyx_frontend::SetAttr::Pos;
use calyx_ir::GetAttributes;
use calyx_ir::source_info::{FileId, LineNum, SourceInfoTable};
use calyx_utils::WithPos;
use std::collections::HashMap;
use std::path::PathBuf;

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

    pub fn add_control_node<A: GetAttributes>(&mut self, node: &mut A) {
        let attr = node.get_mut_attributes();
        let temp = attr.copy_span();
        let (f, (line, _)) = temp.get_line_num();
        let fnum = self.file_ids.get(f).unwrap();
        let pos = self
            .src_table
            .push_position(*fnum, LineNum::new(line as u32));
        attr.insert_set(calyx_frontend::SetAttr::Pos, pos.value());
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
            let mut grp = rrcgrp.borrow_mut();
            let attr = &mut grp.attributes;
            let pos_data = attr.copy_span();
            let (f, span) = pos_data.get_line_num();
            let fid = self.file_ids.get(f).unwrap(); // this def should be in file_ids
            let pos = self
                .src_table
                .push_position(*fid, LineNum::new(span.0 as u32));
            // add tag to group attributes
            attr.insert_set(calyx_frontend::SetAttr::Pos, pos.value());
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

    // control nodes
    fn start_seq(
        &mut self,
        s: &mut calyx_ir::Seq,
        _comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        self.add_control_node(s);
        Ok(Action::Continue)
    }

    fn start_par(
        &mut self,
        s: &mut calyx_ir::Par,
        _comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        self.add_control_node(s);
        Ok(Action::Continue)
    }

    fn start_if(
        &mut self,
        s: &mut calyx_ir::If,
        _comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        self.add_control_node(s);
        Ok(Action::Continue)
    }

    fn start_while(
        &mut self,
        s: &mut calyx_ir::While,
        _comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        self.add_control_node(s);
        Ok(Action::Continue)
    }

    fn start_repeat(
        &mut self,
        s: &mut calyx_ir::Repeat,
        _comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        self.add_control_node(s);
        Ok(Action::Continue)
    }
}
