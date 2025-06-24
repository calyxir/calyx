use std::{cmp, collections::HashMap};

use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{self as ir, Nothing};
use calyx_utils::{CalyxResult, OutputFile};
use serde::Serialize;

// Used by the profiler.

pub struct UniqueControl {
    path_descriptor_json: Option<OutputFile>,
    path_descriptor_infos: HashMap<String, PathDescriptorInfo>,
    par_thread_json: Option<OutputFile>,
    par_thread_info: HashMap<String, HashMap<String, u32>>,
}

impl Named for UniqueControl {
    fn name() -> &'static str {
        "unique-control"
    }

    fn description() -> &'static str {
        "Make all control enables unique by adding a wrapper group"
    }

    fn opts() -> Vec<crate::traversal::PassOpt> {
        vec![
            PassOpt::new(
                "path-descriptor-json",
                "Write the path descriptor of each group to a JSON file",
                ParseVal::OutStream(OutputFile::Null),
                PassOpt::parse_outstream,
            ),
            PassOpt::new(
                "par-thread-json",
                "Write the path descriptor of each group to a JSON file",
                ParseVal::OutStream(OutputFile::Null),
                PassOpt::parse_outstream,
            ),
        ]
    }
}

/// Information to serialize for profiling purposes
#[derive(Serialize)]
struct PathDescriptorInfo {
    pub enables: HashMap<String, String>,
    pub pars: HashMap<String, usize>,
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
            par_thread_json: opts[&"par-thread-json"].not_null_outstream(),
            par_thread_info: HashMap::new(),
        })
    }

    fn clear_data(&mut self) {}
}

fn par_track_static(
    control: &ir::StaticControl,
    start_idx: u32,
    next_idx: u32,
    enable_to_track: &mut HashMap<String, u32>,
) -> u32 {
    match control {
        ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
            par_track_static(&body, start_idx, next_idx, enable_to_track)
        }
        ir::StaticControl::Enable(ir::StaticEnable { group, .. }) => {
            let group_name = group.borrow().name().to_string();
            enable_to_track.insert(group_name, start_idx);
            start_idx + 1
        }
        ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
            let mut idx = next_idx;
            for stmt in stmts {
                idx = par_track_static(stmt, idx, idx + 1, enable_to_track);
            }
            idx
        }
        ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => {
            let mut new_next_idx = next_idx;
            for stmt in stmts {
                let potential_new_idx = par_track_static(
                    stmt,
                    start_idx,
                    new_next_idx,
                    enable_to_track,
                );
                new_next_idx = cmp::max(new_next_idx, potential_new_idx)
            }
            new_next_idx
        }
        ir::StaticControl::If(ir::StaticIf {
            tbranch, fbranch, ..
        }) => {
            let false_next_idx = par_track_static(
                &tbranch,
                start_idx,
                next_idx,
                enable_to_track,
            );
            par_track_static(
                &fbranch,
                start_idx,
                false_next_idx,
                enable_to_track,
            )
        }
        _ => next_idx,
    }
}

fn par_track(
    control: &ir::Control,
    start_idx: u32,
    next_idx: u32,
    enable_to_track: &mut HashMap<String, u32>,
) -> u32 {
    match control {
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            let mut new_next_idx = next_idx;
            for stmt in stmts {
                let potential_new_idx =
                    par_track(stmt, start_idx, new_next_idx, enable_to_track);
                new_next_idx = cmp::max(new_next_idx, potential_new_idx)
            }
            new_next_idx
        }
        ir::Control::Enable(enable) => {
            let group_name = enable.group.borrow().name().to_string();
            enable_to_track.insert(group_name, start_idx);
            start_idx + 1
        }
        ir::Control::Par(ir::Par { stmts, .. }) => {
            let mut idx = next_idx;
            for stmt in stmts {
                idx = par_track(stmt, idx, idx + 1, enable_to_track);
            }
            idx
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            let false_next_idx =
                par_track(&tbranch, start_idx, next_idx, enable_to_track);
            par_track(&fbranch, start_idx, false_next_idx, enable_to_track)
        }
        ir::Control::While(ir::While { body, .. }) => {
            par_track(&body, start_idx, next_idx, enable_to_track)
        }
        ir::Control::Repeat(ir::Repeat { body, .. }) => {
            par_track(&body, start_idx, next_idx, enable_to_track)
        }
        ir::Control::Static(static_control) => par_track_static(
            static_control,
            start_idx,
            next_idx,
            enable_to_track,
        ),
        _ => next_idx,
    }
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
            path_descriptor_info.pars.insert(par_id, par.stmts.len());
        }
        ir::Control::If(iff) => {
            // process true branch
            let true_id = format!("{}-t", current_id);
            label_control_enables(
                &iff.tbranch,
                true_id,
                path_descriptor_info,
                false,
            );
            // process false branch
            let false_id = format!("{}-f", current_id);
            label_control_enables(
                &iff.fbranch,
                false_id,
                path_descriptor_info,
                false,
            );
        }
        ir::Control::While(ir::While { body, .. }) => {
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
        let unique_group_name: String = format!("{}UG", group_name);
        // create an unique-ified version of the group
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

    fn static_enable(
        &mut self,
        s: &mut calyx_ir::StaticEnable,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let group_name = s.group.borrow().name();
        // UG stands for "unique group". This is to separate these names from the original group names
        let unique_group_name = format!("{}UG", group_name);
        // create an unique-ified version of the group
        let mut builder = ir::Builder::new(comp, sigs);
        let unique_group = builder.add_static_group(
            unique_group_name,
            s.group.borrow().get_latency(),
        );
        // Since we don't need to worry about setting the `done` signal, the assignments of unique_group are
        // a straight copy of the original group's assignments
        unique_group.borrow_mut().assignments =
            s.group.borrow().assignments.clone();

        Ok(Action::Change(Box::new(ir::Control::static_enable(
            unique_group,
        ))))
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
            pars: HashMap::new(),
        };
        label_control_enables(
            &control,
            format!("{}.", comp.name.to_string()),
            &mut path_descriptor_info,
            true,
        );
        self.path_descriptor_infos
            .insert(comp.name.to_string(), path_descriptor_info);
        let mut enable_to_track: HashMap<String, u32> = HashMap::new();
        par_track(&control, 0, 1, &mut enable_to_track);
        self.par_thread_info
            .insert(comp.name.to_string(), enable_to_track);
        Ok(Action::Continue)
    }

    fn finish_context(&mut self, _ctx: &mut calyx_ir::Context) -> VisResult {
        if let Some(json_out_file) = &mut self.path_descriptor_json {
            let _ = serde_json::to_writer_pretty(
                json_out_file.get_write(),
                &self.path_descriptor_infos,
            );
        }
        if let Some(json_out_file) = &mut self.par_thread_json {
            let _ = serde_json::to_writer_pretty(
                json_out_file.get_write(),
                &self.par_thread_info,
            );
        }
        Ok(Action::Continue)
    }
}
