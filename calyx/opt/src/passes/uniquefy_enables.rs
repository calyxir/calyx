use std::{cmp, collections::BTreeMap, collections::BTreeSet};

use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{self as ir, Nothing};
use calyx_utils::{CalyxResult, OutputFile};
use serde::Serialize;

// Converts each dynamic and static enable to an enable of a unique group.
// Also (1) computes path descriptors for each unique enable group and par (outputted to `path_descriptor_json` if provided); and
// (2) statically assigns par thread ids to each unique enable group (outputted to `par_thread_json` if provided).
// Used by the profiler.

pub struct UniquefyEnables {
    path_descriptor_json: Option<OutputFile>,
    path_descriptor_infos: BTreeMap<String, PathDescriptorInfo>,
    par_thread_json: Option<OutputFile>,
    par_thread_info: BTreeMap<String, BTreeMap<String, u32>>,
}

impl Named for UniquefyEnables {
    fn name() -> &'static str {
        "uniquefy-enables"
    }

    fn description() -> &'static str {
        "Make all control (dynamic and static) enables unique."
    }

    fn opts() -> Vec<crate::traversal::PassOpt> {
        vec![
            PassOpt::new(
                "path-descriptor-json",
                "Write the path descriptor of each enable and par to a JSON file",
                ParseVal::OutStream(OutputFile::Null),
                PassOpt::parse_outstream,
            ),
            PassOpt::new(
                "par-thread-json",
                "Write an assigned thread ID of each enable to a JSON file",
                ParseVal::OutStream(OutputFile::Null),
                PassOpt::parse_outstream,
            ),
        ]
    }
}

/// Information to serialize for locating path descriptors
#[derive(Serialize)]
struct PathDescriptorInfo {
    pub enables: BTreeMap<String, String>,
    pub pars: BTreeSet<String>,
}

impl ConstructVisitor for UniquefyEnables {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(ctx);
        Ok(UniquefyEnables {
            path_descriptor_json: opts[&"path-descriptor-json"]
                .not_null_outstream(),
            path_descriptor_infos: BTreeMap::new(),
            par_thread_json: opts[&"par-thread-json"].not_null_outstream(),
            par_thread_info: BTreeMap::new(),
        })
    }

    fn clear_data(&mut self) {}
}

fn assign_par_threads_static(
    control: &ir::StaticControl,
    start_idx: u32,
    next_idx: u32,
    enable_to_track: &mut BTreeMap<String, u32>,
) -> u32 {
    match control {
        ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
            assign_par_threads_static(
                body,
                start_idx,
                next_idx,
                enable_to_track,
            )
        }
        ir::StaticControl::Enable(ir::StaticEnable { group, .. }) => {
            let group_name = group.borrow().name().to_string();
            enable_to_track.insert(group_name, start_idx);
            start_idx + 1
        }
        ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
            let mut idx = next_idx;
            for stmt in stmts {
                idx = assign_par_threads_static(
                    stmt,
                    idx,
                    idx + 1,
                    enable_to_track,
                );
            }
            idx
        }
        ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => {
            let mut new_next_idx = next_idx;
            for stmt in stmts {
                let potential_new_idx = assign_par_threads_static(
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
            let false_next_idx = assign_par_threads_static(
                tbranch,
                start_idx,
                next_idx,
                enable_to_track,
            );
            assign_par_threads_static(
                fbranch,
                start_idx,
                false_next_idx,
                enable_to_track,
            )
        }
        _ => next_idx,
    }
}

fn assign_par_threads(
    control: &ir::Control,
    start_idx: u32,
    next_idx: u32,
    enable_to_track: &mut BTreeMap<String, u32>,
) -> u32 {
    match control {
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            let mut new_next_idx = next_idx;
            for stmt in stmts {
                let potential_new_idx = assign_par_threads(
                    stmt,
                    start_idx,
                    new_next_idx,
                    enable_to_track,
                );
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
                idx = assign_par_threads(stmt, idx, idx + 1, enable_to_track);
            }
            idx
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            let false_next_idx = assign_par_threads(
                tbranch,
                start_idx,
                next_idx,
                enable_to_track,
            );
            assign_par_threads(
                fbranch,
                start_idx,
                false_next_idx,
                enable_to_track,
            )
        }
        ir::Control::While(ir::While { body, .. }) => {
            assign_par_threads(body, start_idx, next_idx, enable_to_track)
        }
        ir::Control::Repeat(ir::Repeat { body, .. }) => {
            assign_par_threads(body, start_idx, next_idx, enable_to_track)
        }
        ir::Control::Static(static_control) => assign_par_threads_static(
            static_control,
            start_idx,
            next_idx,
            enable_to_track,
        ),
        ir::Control::Invoke(_) => {
            panic!("compile-invoke should be run before unique-control!")
        }
        _ => next_idx,
    }
}

fn compute_path_descriptors_static(
    control: &ir::StaticControl,
    current_id: String,
    path_descriptor_info: &mut PathDescriptorInfo,
    parent_is_component: bool,
) {
    match control {
        ir::StaticControl::Repeat(_static_repeat) => todo!(),
        ir::StaticControl::Enable(ir::StaticEnable { group, .. }) => {
            let group_id = if parent_is_component {
                // edge case: the entire control is just one static enable
                format!("{}0", current_id)
            } else {
                current_id
            };
            let group_name = group.borrow().name();
            path_descriptor_info
                .enables
                .insert(group_name.to_string(), group_id);
        }
        ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
            let par_id = format!("{}-", current_id);
            for (acc, stmt) in stmts.iter().enumerate() {
                let stmt_id = format!("{}{}", par_id, acc);
                compute_path_descriptors_static(
                    stmt,
                    stmt_id,
                    path_descriptor_info,
                    false,
                );
            }
            path_descriptor_info.pars.insert(par_id);
        }
        ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => {
            for (acc, stmt) in stmts.iter().enumerate() {
                let stmt_id = format!("{}-{}", current_id, acc);
                compute_path_descriptors_static(
                    stmt,
                    stmt_id,
                    path_descriptor_info,
                    false,
                );
            }
        }
        ir::StaticControl::If(ir::StaticIf {
            tbranch, fbranch, ..
        }) => {
            // process true branch
            let true_id = format!("{}-t", current_id);
            compute_path_descriptors_static(
                tbranch,
                true_id,
                path_descriptor_info,
                false,
            );
            // process false branch
            let false_id = format!("{}-f", current_id);
            compute_path_descriptors_static(
                fbranch,
                false_id,
                path_descriptor_info,
                false,
            );
        }
        ir::StaticControl::Empty(_empty) => todo!(),
        ir::StaticControl::Invoke(_static_invoke) => todo!(),
    }
}

fn compute_path_descriptors(
    control: &ir::Control,
    current_id: String,
    path_descriptor_info: &mut PathDescriptorInfo,
    parent_is_component: bool,
) {
    match control {
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            for (acc, stmt) in stmts.iter().enumerate() {
                let stmt_id = format!("{}-{}", current_id, acc);
                compute_path_descriptors(
                    stmt,
                    stmt_id,
                    path_descriptor_info,
                    false,
                );
            }
        }
        ir::Control::Par(ir::Par { stmts, .. }) => {
            let par_id = format!("{}-", current_id);
            for (acc, stmt) in stmts.iter().enumerate() {
                let stmt_id = format!("{}{}", par_id, acc);
                compute_path_descriptors(
                    stmt,
                    stmt_id,
                    path_descriptor_info,
                    false,
                );
            }
            path_descriptor_info.pars.insert(par_id);
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            // process true branch
            let true_id = format!("{}-t", current_id);
            compute_path_descriptors(
                tbranch,
                true_id,
                path_descriptor_info,
                false,
            );
            // process false branch
            let false_id = format!("{}-f", current_id);
            compute_path_descriptors(
                fbranch,
                false_id,
                path_descriptor_info,
                false,
            );
        }
        ir::Control::While(ir::While { body, .. }) => {
            let body_id = format!("{}-b", current_id);
            compute_path_descriptors(
                body,
                body_id,
                path_descriptor_info,
                false,
            );
        }
        ir::Control::Enable(ir::Enable { group, .. }) => {
            let group_id = if parent_is_component {
                // edge case: the entire control is just one enable
                format!("{}0", current_id)
            } else {
                current_id
            };
            let group_name = group.borrow().name();
            path_descriptor_info
                .enables
                .insert(group_name.to_string(), group_id);
        }
        ir::Control::Repeat(ir::Repeat { .. }) => {
            todo!()
        }
        ir::Control::Static(static_control) => {
            compute_path_descriptors_static(
                static_control,
                current_id,
                path_descriptor_info,
                parent_is_component,
            );
        }
        ir::Control::Empty(_) => (),
        ir::Control::FSMEnable(_) => todo!(),
        ir::Control::Invoke(_) => {
            panic!("compile-invoke should be run before unique-control!")
        }
    }
}

impl Visitor for UniquefyEnables {
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
        // Compute path descriptors for each enable and par block in the component.
        let control = comp.control.borrow();
        let mut path_descriptor_info = PathDescriptorInfo {
            enables: BTreeMap::new(),
            pars: BTreeSet::new(),
        };
        compute_path_descriptors(
            &control,
            format!("{}.", comp.name),
            &mut path_descriptor_info,
            true,
        );
        self.path_descriptor_infos
            .insert(comp.name.to_string(), path_descriptor_info);
        // Compute par thread ids for each enable in the component.
        let mut enable_to_track: BTreeMap<String, u32> = BTreeMap::new();
        assign_par_threads(&control, 0, 1, &mut enable_to_track);
        self.par_thread_info
            .insert(comp.name.to_string(), enable_to_track);
        Ok(Action::Continue)
    }

    fn finish_context(&mut self, _ctx: &mut calyx_ir::Context) -> VisResult {
        // Write path descriptors to file if prompted.
        if let Some(json_out_file) = &mut self.path_descriptor_json {
            let _ = serde_json::to_writer_pretty(
                json_out_file.get_write(),
                &self.path_descriptor_infos,
            );
        }
        // Write par thread assignments to file if prompted.
        if let Some(json_out_file) = &mut self.par_thread_json {
            let _ = serde_json::to_writer_pretty(
                json_out_file.get_write(),
                &self.par_thread_info,
            );
        }
        Ok(Action::Continue)
    }
}
