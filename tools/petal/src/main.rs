mod control;
mod design;
mod shared_cells;
mod timeline;
mod visuals;

#[path = "perfetto.protos.rs"]
#[allow(clippy::all)]
#[rustfmt::skip]
mod perfetto_protos;

use crate::design::{Design, RegisterId, Stack};
use crate::timeline::{CurrentlyActive, Timeline};
use crate::visuals::{compute_flame, write_flame};
use anyhow::{Context, Ok, Result, anyhow};
use baa::{BitVecMutOps, BitVecValue};
use clap::Parser;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
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
    #[arg(long, default_value_t = 100)]
    num_print_cycles: u64,
}

pub type Stacks = IndexMap<BitVecValue, (u64, Vec<Stack>, CurrentlyActive)>;

fn collect_stacks(
    design: &Design,
    timeline: &mut Timeline,
    probe_values: &[BitVecValue],
) -> Result<(Stacks, u64, u64)> {
    // Compute the trace (stacks for each active cycle) from probe_values
    let mut out: Stacks = IndexMap::default();
    // NOTE: Can't just use enumerate because we want to skip the warmup cycles before main starts.
    // either we accumulate when main is active, or subtract the cycle where main started from the enumeration.
    let mut cycle_count = 0;
    let mut starting_cycle = 0;
    for value in probe_values.iter() {
        let active_this_cycle = if let Some((count, s, active)) =
            out.get_mut(value)
        {
            if !s.is_empty() {
                cycle_count += 1;
                if starting_cycle == 0 {
                    starting_cycle = cycle_count;
                }
            }
            *count += 1;
            active
        } else {
            let (stacks, active_this_cycle) =
                design.compute_cycle_trace(value)?;
            if !stacks.is_empty() {
                cycle_count += 1;
            }
            out.insert(value.clone(), (1, stacks, active_this_cycle.clone()));
            &mut active_this_cycle.clone()
        };
        timeline.update_timeline(active_this_cycle, cycle_count)?;
    }
    // close out the timeline view
    let end = CurrentlyActive::new();
    timeline.update_timeline(&end, cycle_count)?;
    Ok((out, starting_cycle, cycle_count))
}

fn add_cregisters_to_timeline(
    timeline: &mut Timeline,
    design: &Design,
    register_value_diffs: FxHashMap<u64, FxHashMap<RegisterId, u64>>,
    starting_cycle: u64,
    num_cycles: u64,
) -> Result<()> {
    design.add_cregisters_to_timeline(
        timeline,
        register_value_diffs,
        starting_cycle,
        num_cycles,
    )
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

    // create tracks in the timeline
    let par_tracks = timeline::read_par_tracks(args.par_tracks_filename)?;
    let mut timeline = Timeline::new()?;
    design.build_timeline_tracks(&mut timeline, &par_tracks)?;

    // all probe signals we would need to track
    let signals = design.get_signals();
    let register_signals_map: FxHashMap<SignalRef, RegisterId> =
        design.get_register_signals();
    let mut register_signals: Vec<SignalRef> =
        register_signals_map.keys().cloned().collect();
    let mut signals_to_track = signals.clone();
    signals_to_track
        .append(&mut register_signals_map.keys().cloned().collect());

    let filter = wellen::stream::Filter::include_signals(&signals_to_track);

    let mut clock_previous = true;

    // One bit vector for each cycle. Each index in the BitVecValue corresponds to a probe.
    // If it is active, the index will contain 1.
    let mut probe_values: Vec<BitVecValue> = vec![];

    let mut register_value_diffs: FxHashMap<u64, FxHashMap<RegisterId, u64>> =
        FxHashMap::default();
    let mut acc: u64 = 0;

    // populate probe_values on the clock's rising edge
    let clock_signal_ref = design.clk();
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
            for signal in changed {
                if let Some(register_id) = register_signals_map.get(&signal) {
                    let register_value: u64 =
                        values.get(signal).unwrap().try_into().unwrap();
                    control_register_diffs.insert(*register_id, register_value);
                } else {
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
            }
            acc += 1; // tracking clock ticks for control register diffs
            if !control_register_diffs.is_empty() {
                register_value_diffs.insert(acc, control_register_diffs);
            }
            probe_values.push(value.clone());
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
    println!("Number of clock ticks: {}", probe_values.len());

    let (stacks, starting_cycle, num_cycles) =
        collect_stacks(&design, &mut timeline, &probe_values)?;
    print_stacks(&probe_values, &stacks, args.num_print_cycles);
    let flame_info = compute_flame(&stacks)?;
    write_flame(&flame_info, args.scaled_flame_out, args.flat_flame_out)?;
    add_cregisters_to_timeline(
        &mut timeline,
        &design,
        register_value_diffs,
        starting_cycle,
        num_cycles,
    )?;

    timeline.output_timeline(&args.out_dir)?;

    Ok(())
}
