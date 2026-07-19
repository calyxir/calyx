mod control;
mod design;
mod shared_cells;
mod timeline;

mod visuals;

mod adls;
mod dahlia_design;
mod statistics;

use crate::adls::{AdlInfo, AdlIntermediateInfo};
use crate::design::{Design, RegisterId, Stack};
use crate::statistics::Statistics;
use crate::timeline::{CalyxTimeline, CurrentlyActive};
use crate::visuals::flamegraph::write_calyx_flames;
use anyhow::{Context, Ok, Result, anyhow};
use baa::{BitVecMutOps, BitVecValue};
use clap::Parser;
use indexmap::IndexMap;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fs;
use wellen::*;

#[derive(Parser, Debug)]
#[command(name = "petal")]
#[command(author = "Ayaka Yorihiro <ayaka@cs.cornell.edu>")]
#[command(version)]
#[command(about = "Calyx profiler.", long_about = None)]
struct Args {
    #[arg(value_name = "WAV", index = 1)]
    filename: String,
    #[arg(value_name = "TDCC", index = 2)]
    tdcc_filename: String, // fsm.json
    #[arg(value_name = "PATH_DESC", index = 3)]
    path_descriptor_filename: String, // path-descriptor.json
    #[arg(value_name = "CTRL_POS", index = 4)]
    control_pos_filename: String, // ctrl-pos.json
    #[arg(value_name = "SHARED_CELLS", index = 5)]
    shared_cells: String, // shared-cells.json
    #[arg(value_name = "PAR_TRACKS", index = 6)]
    par_tracks_filename: String, // enable-par-track.json
    #[arg(value_name = "OUT_DIR", index = 7)]
    out_dir: String,
    #[arg(long)]
    scaled_flame_out: Option<String>,
    #[arg(long)]
    flat_flame_out: Option<String>,
    #[arg(long)]
    adl_file: Option<String>,
    #[arg(long)]
    dahlia_parent_map: Option<String>,
    #[arg(long, default_value_t = 100)]
    num_print_cycles: u64,
}

/// bool flag represents whether this cycle contained an active Group or Primitive leaf.
pub type Stacks =
    IndexMap<BitVecValue, (u64, Vec<Stack>, CurrentlyActive, bool)>;

fn collect_stacks(
    design: &Design,
    timeline: &mut CalyxTimeline,
    stats: &mut Statistics,
    probe_values: &[BitVecValue],
    register_value_diffs: &FxHashMap<u64, FxHashMap<RegisterId, u64>>,
    adl_info_opt: &mut Option<AdlIntermediateInfo>,
) -> Result<Stacks> {
    // Compute the trace (stacks for each active cycle) from probe_values
    let mut out: Stacks = IndexMap::default();
    let mut currently_active = CurrentlyActive::default();
    for (cycle_count, value) in probe_values.iter().enumerate() {
        let (active_this_cycle, gp_flag): (&mut CurrentlyActive, bool) =
            if let Some((count, _s, active, gp_flag)) = out.get_mut(value) {
                *count += 1;
                (active, *gp_flag)
            } else {
                let (stacks, active_this_cycle, group_or_primitive_leaf) =
                    design.compute_cycle_trace(value)?;
                out.insert(
                    value.clone(),
                    (
                        1,
                        stacks,
                        active_this_cycle.clone(),
                        group_or_primitive_leaf,
                    ),
                );
                (&mut active_this_cycle.clone(), group_or_primitive_leaf)
            };

        // get cell/control/group activity information
        let (started, ended) = currently_active.resolve(active_this_cycle)?;
        timeline.update(&started, &ended, cycle_count as u64)?;
        stats.update(
            &started,
            &ended,
            cycle_count as u64,
            gp_flag,
            register_value_diffs,
            active_this_cycle.get_active_cells(),
        );
        if let Some(adl_info) = adl_info_opt {
            adl_info.process_cycle(active_this_cycle)?;
        }
        currently_active = active_this_cycle.clone();
    }
    // close out timeline view/statistics by "ending" the contents of `currently_active`.
    let empty = CurrentlyActive::new();
    let total_cycles = probe_values.len() as u64;
    timeline.update(&empty, &currently_active, total_cycles)?;
    stats.close(&currently_active, total_cycles);

    Ok(out)
}

fn build_statistics(d: &Design) -> Result<Statistics> {
    let g = d.get_group_component_names();
    let c = d.get_cell_name_fsm_count();
    Ok(Statistics::new(g, c))
}

fn print_stacks(
    probe_values: &[BitVecValue],
    all_stacks: &Stacks,
    num_print_cycles: u64,
) {
    for (cycle, stacks) in probe_values
        .iter()
        .map(|v| &all_stacks[v].1)
        .filter(|s| !s.is_empty())
        .take(num_print_cycles as usize + 1)
        .enumerate()
    {
        println!("{cycle}");
        for stack in stacks {
            let stack_str = stack.join(", ");
            println!("	[{stack_str}]");
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    fs::create_dir_all(&args.out_dir)?;

    let ctrl_info = crate::control::ControlInfo::new(
        args.tdcc_filename,
        args.path_descriptor_filename,
        args.control_pos_filename,
    )?;

    let shared_cells =
        crate::shared_cells::SharedCellsInfo::new(args.shared_cells)?;

    let opts = LoadOptions {
        multi_thread: true,
        remove_scopes_with_empty_name: false,
    };

    let mut wav = wellen::stream::read_from_file(&args.filename, &opts)
        .with_context(|| format!("Failed to load {}", args.filename))?;

    // static tree
    let design = Design::new(wav.hierarchy(), ctrl_info, shared_cells)?;

    // construct information for the ADL, if this is an ADL program
    let mut adl_info = if let Some(adl_file) = args.adl_file {
        let a = AdlIntermediateInfo::new(
            &adl_file,
            args.dahlia_parent_map,
            &design,
        )?;
        Some(a)
    } else {
        None
    };

    // create tracks in the timeline
    let par_tracks = timeline::read_par_tracks(args.par_tracks_filename)?;
    let mut timeline = CalyxTimeline::new()?;
    design.build_timeline_tracks(&mut timeline, &par_tracks)?;
    let mut statistics = build_statistics(&design)?;

    // all probe signals we would need to track
    let signals = design.get_signals();
    let register_signals_map: FxHashMap<SignalRef, (SignalRef, RegisterId)> =
        design.get_register_signals_map();
    let mut signals_to_track = signals.clone();
    for (write_en_signal, (in_signal, _)) in register_signals_map.iter() {
        signals_to_track.push(*write_en_signal);
        signals_to_track.push(*in_signal);
    }
    let filter = wellen::stream::Filter::include_signals(&signals_to_track);
    let probe_signal_set: FxHashSet<SignalRef> =
        signals.iter().copied().collect();

    let mut clock_previous = true;

    // One bit vector for each cycle. Each index in the BitVecValue corresponds to a probe.
    // If it is active, the index will contain 1.
    let mut probe_values: Vec<BitVecValue> = vec![];

    // We want to track the value changes in each register.
    let mut register_value_diffs: FxHashMap<u64, FxHashMap<RegisterId, u64>> =
        FxHashMap::default();
    let mut acc: u64 = 0;

    // populate probe_values on the clock's rising edge
    let clock_signal_ref = design.clk();
    let (main_go_ref, main_done_ref) = design.main_probes();
    let signal_bits = FxHashMap::from_iter(
        signals
            .iter()
            .enumerate()
            .map(|(idx, &signal)| (signal, idx as u32)),
    );
    // Get all probes into a single bitvector
    let mut value = BitVecValue::zero(signals.len() as u32);
    wav.stream_time_steps(filter, |_time, values, changed| {
        let c: bool =
            values.get(&clock_signal_ref).unwrap().try_into().unwrap();
        let mut control_register_diffs = FxHashMap::default();
        if c && !clock_previous && !changed.is_empty() {
            let main_go: bool =
                values.get(&main_go_ref).unwrap().try_into().unwrap();
            let main_done: bool =
                values.get(&main_done_ref).unwrap().try_into().unwrap();
            if main_go && !main_done {
                // first process the register write_ens and ins
                for changed_write_en in changed
                    .iter()
                    .filter(|&s| register_signals_map.contains_key(s))
                {
                    let (in_signal, register_id) =
                        register_signals_map.get(changed_write_en).unwrap();
                    let register_new_value: u64 =
                        values.get(in_signal).unwrap().try_into().unwrap();
                    if values.get(changed_write_en).unwrap().try_into().unwrap()
                    {
                        // only add the register update when the write_en is up
                        control_register_diffs
                            .insert(*register_id, register_new_value);
                    }
                }
                for signal in
                    changed.iter().filter(|&s| probe_signal_set.contains(s))
                {
                    // normal probe values
                    let probe_value: bool = values
                        .get(signal)
                        .unwrap()
                        .try_into()
                        .expect("Signal needs to be a bitvector!");
                    let idx = signal_bits[signal];
                    if probe_value {
                        value.set_bit(idx);
                    } else {
                        value.clear_bit(idx);
                    }
                }
                if !control_register_diffs.is_empty() {
                    register_value_diffs.insert(acc, control_register_diffs);
                }
                probe_values.push(value.clone());
                acc += 1; // tracking clock ticks for control register diffs
            }
        }
        clock_previous = c;
        Ok(())
    })
    .map_err(|e| match e {
        stream::StreamError::Wellen(wellen_error) => {
            anyhow!(wellen_error)
        }
        stream::StreamError::Callback(e) => e,
    })?;
    println!("Number of cycles: {}", probe_values.len());

    let stacks = collect_stacks(
        &design,
        &mut timeline,
        &mut statistics,
        &probe_values,
        &register_value_diffs,
        &mut adl_info,
    )?;
    print_stacks(&probe_values, &stacks, args.num_print_cycles);
    write_calyx_flames(&stacks, args.scaled_flame_out, args.flat_flame_out)?;
    design.add_control_registers_to_timeline(
        &mut timeline,
        register_value_diffs,
    )?;

    timeline.output_timeline(&args.out_dir)?;
    statistics.output(&args.out_dir)?;

    if let Some(mut adl_data) = adl_info {
        adl_data.output_flame(&args.out_dir)?;
    }

    Ok(())
}
