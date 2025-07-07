use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};

//use calyx_frontend::SetAttr::Pos;
use calyx_ir::GetAttributes;
use calyx_ir::source_info::{FileId, LineNum, SourceInfoTable};
use calyx_utils::WithPos;
use std::collections::HashMap;
use std::path::PathBuf;

/// Metadata creates and stores the source info table for the currently running program
pub struct Metadata {
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
}

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
    fn start_context(&mut self, ctx: &mut calyx_ir::Context) -> VisResult {
        if let Some(x) = std::mem::take(&mut ctx.source_info_table) {
            self.src_table = x;
        }
        Ok(Action::Continue)
    }

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
        let (file, (line, _)) = binding.get_line_num();

        // add file to source table (if not already in)
        if !self.file_ids.contains_key(file) {
            let id = self.src_table.push_file(PathBuf::from(file));
            self.file_ids.insert(String::from(file), id);
        }

        // add source position of the component itself
        let component_file_id = self.file_ids.get(file).unwrap();
        let component_pos = self
            .src_table
            .push_position(*component_file_id, LineNum::new(line as u32));
        comp.attributes
            .insert_set(calyx_frontend::SetAttr::Pos, component_pos.value());

        // visit all groups in component
        for rrcgrp in comp.groups.iter() {
            let mut grp = rrcgrp.borrow_mut();
            let attr = &mut grp.attributes;
            let pos_data = attr.copy_span();
            let (f, (line_start, _line_end)) = pos_data.get_line_num();
            let fid = self.file_ids.get(f).unwrap(); // this def should be in file_ids
            let pos = self
                .src_table
                .push_position(*fid, LineNum::new(line_start as u32));
            // add tag to group attributes
            attr.insert_set(calyx_frontend::SetAttr::Pos, pos.value());
        }
        Ok(Action::Continue)
    }

    fn finish_context(&mut self, ctx: &mut calyx_ir::Context) -> VisResult {
        ctx.source_info_table = Some(std::mem::take(&mut self.src_table));
        Ok(Action::Continue)
    }

    fn enable(
        &mut self,
        s: &mut calyx_ir::Enable,
        _comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        self.add_control_node(s);
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
