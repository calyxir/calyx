use std::collections::{HashMap, HashSet};

use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{self as ir, Nothing, While};
use calyx_utils::{CalyxResult, OutputFile};
use serde::Serialize;

/// Adds probe wires to each group (includes static groups and comb groups) to detect when a group is active.
/// Used by the profiler.
pub struct UniqueControl {
    path_descriptor_json: Option<OutputFile>,
    path_descriptor_infos: HashMap<String, PathDescriptorInfo>,
}

impl Named for UniqueControl {
    fn name() -> &'static str {
        "unique-control"
    }

    fn description() -> &'static str {
        "Make all control enables unique by adding a wrapper group"
    }

    fn opts() -> Vec<crate::traversal::PassOpt> {
        vec![PassOpt::new(
            "path-descriptor-json",
            "Write the path descriptor of each group to a JSON file",
            ParseVal::OutStream(OutputFile::Null),
            PassOpt::parse_outstream,
        )]
    }
}

/// Information to serialize for profiling purposes
#[derive(Serialize)]
struct PathDescriptorInfo {
    pub enables: HashMap<String, String>,
    pub pars: HashSet<String>,
}

impl ConstructVisitor for UniqueControl {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(ctx);
        Ok(UniqueControl {
            path_descriptor_json: opts[&"path-descriptor-json"]
                .not_null_outstream(),
            path_descriptor_infos: HashMap::new(),
        })
    }

    fn clear_data(&mut self) {}
}

fn label_control_enables(
    control: &ir::Control,
    current_id: String,
    path_descriptor_info: &mut PathDescriptorInfo,
    parent_is_component: bool,
) -> () {
    match control {
        ir::Control::Seq(seq) => {
            let mut acc = 0;
            for stmt in &seq.stmts {
                let stmt_id = format!("{}-{}", current_id, acc);
                label_control_enables(
                    stmt,
                    stmt_id,
                    path_descriptor_info,
                    false,
                );
                acc += 1;
            }
        }
        ir::Control::Par(par) => {
            let mut acc = 0;
            let par_id = format!("{}-", current_id);
            for stmt in &par.stmts {
                let stmt_id = format!("{}{}", par_id, acc);
                label_control_enables(
                    stmt,
                    stmt_id,
                    path_descriptor_info,
                    false,
                );
                acc += 1;
            }
            path_descriptor_info.pars.insert(par_id);
        }
        ir::Control::If(iff) => {
            // process true branch
            let true_id = format!("{}t", current_id);
            label_control_enables(
                &iff.tbranch,
                true_id,
                path_descriptor_info,
                false,
            );
            // process false branch
            let false_id = format!("{}f", current_id);
            label_control_enables(
                &iff.fbranch,
                false_id,
                path_descriptor_info,
                false,
            );
        }
        ir::Control::While(While { body, .. }) => {
            let body_id = format!("{}-b", current_id);
            label_control_enables(&body, body_id, path_descriptor_info, false);
        }
        ir::Control::Enable(enable) => {
            let group_id = if parent_is_component {
                // edge case: the entire control is just one enable
                format!("{}0", current_id)
            } else {
                current_id
            };
            let group_name = enable.group.borrow().name();
            path_descriptor_info
                .enables
                .insert(group_name.to_string(), group_id);
        }
        _ => {}
    }
}

impl Visitor for UniqueControl {
    fn enable(
        &mut self,
        s: &mut calyx_ir::Enable,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let group_name = s.group.borrow().name();
        // UG stands for "unique group". This is to separate these names from the original group names
        let unique_group_name = format!("{}UG", group_name);
        // create a wrapper group
        let mut builder = ir::Builder::new(comp, sigs);
        let unique_group = builder.add_group(unique_group_name);
        // let unique_group_assignments = s.group.borrow().assignments.clone();
        let mut unique_group_assignments: Vec<calyx_ir::Assignment<Nothing>> =
            Vec::new();
        for asgn in s.group.borrow().assignments.iter() {
            if asgn.dst.borrow().get_parent_name() == group_name
                && asgn.dst.borrow().name == "done"
            {
                // done needs to be reassigned
                let new_done_asgn = builder.build_assignment(
                    unique_group.borrow().get("done"),
                    asgn.src.clone(),
                    *asgn.guard.clone(),
                );
                unique_group_assignments.push(new_done_asgn);
            } else {
                unique_group_assignments.push(asgn.clone());
            }
        }
        unique_group
            .borrow_mut()
            .assignments
            .append(&mut unique_group_assignments);
        Ok(Action::Change(Box::new(ir::Control::enable(unique_group))))
    }

    fn finish(
        &mut self,
        comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let control = comp.control.borrow();
        let mut path_descriptor_info = PathDescriptorInfo {
            enables: HashMap::new(),
            pars: HashSet::new(),
        };
        label_control_enables(
            &control,
            format!("{}.", comp.name.to_string()),
            &mut path_descriptor_info,
            true,
        );
        self.path_descriptor_infos
            .insert(comp.name.to_string(), path_descriptor_info);
        Ok(Action::Continue)
    }

    fn finish_context(&mut self, _ctx: &mut calyx_ir::Context) -> VisResult {
        // iterate through and record info about who is what's parent
        if let Some(json_out_file) = &mut self.path_descriptor_json {
            let _ = serde_json::to_writer_pretty(
                json_out_file.get_write(),
                &self.path_descriptor_infos,
            );
        }
        Ok(Action::Continue)
    }
}
