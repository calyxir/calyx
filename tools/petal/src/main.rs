mod control;
mod design;
mod shared_cells;
mod visuals;

use anyhow::{Context, Ok, Result, anyhow};
use baa::{BitVecMutOps, BitVecValue};
use clap::Parser;
use indexmap::IndexSet;
use rustc_hash::FxHashMap;
use wellen::*;

use crate::design::{Design, Stack};
use crate::visuals::{compute_flame, write_flame};

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
    #[arg(long)]
    scaled_flame_out: Option<String>,
    #[arg(long)]
    flat_flame_out: Option<String>,
    #[arg(long, default_value_t = 100)]
    num_print_cycles: u64,
}

pub type Stacks = Vec<(u64, Vec<Stack>)>;

fn print_stacks(
    probe_values: &[usize],
    all_stacks: &Stacks,
    num_print_cycles: u64,
) {
    for (cycle, stacks) in probe_values
        .iter()
        .map(|v| &all_stacks[*v].1)
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

    // all probe signals we would need to track
    let signals = design.get_signals();

    let filter = wellen::stream::Filter::include_signals(&signals);

    let mut clock_previous = true;

    // populate probe_values on the clock's falling edge
    let clock_signal_ref = design.clk();
    let signal_bits = FxHashMap::from_iter(
        signals
            .iter()
            .enumerate()
            .map(|(idx, &signal)| (signal, idx as u32)),
    );

    // One bit vector for each cycle. Each index in the BitVecValue corresponds to a probe.
    // If it is active, the index will contain 1.
    let mut probe_values = BitVecValue::zero(signals.len() as u32);

    // record which unique probe_value we observe each cycle
    let mut unique_probe_values = IndexSet::<BitVecValue>::default();
    let mut probe_values_in_cycle = vec![];

    // compute the trace (stacks for each active cycle) from probe_values
    let mut stacks = vec![];

    wav.stream_time_steps(filter, |_time, values, changed| {
        let c: bool =
            values.get(&clock_signal_ref).unwrap().try_into().unwrap();
        if c && !clock_previous && !changed.is_empty() {
            for signal in changed {
                let probe_value: bool = values
                    .get(signal)
                    .unwrap()
                    .try_into()
                    .expect("Signal needs to be a bitvector!");
                let idx = signal_bits[signal];
                if probe_value {
                    probe_values.set_bit(idx);
                } else {
                    probe_values.clear_bit(idx);
                }
            }

            // have we seen this value before?
            let probe_values_idx = if let Some(idx) =
                unique_probe_values.get_index_of(&probe_values)
            {
                idx
            } else {
                let new_idx =
                    unique_probe_values.insert_full(probe_values.clone()).0;
                assert_eq!(new_idx, stacks.len());
                let local_stacks = design.compute_cycle_trace(&probe_values)?;
                stacks.push((0, local_stacks));
                new_idx
            };

            probe_values_in_cycle.push(probe_values_idx);
            stacks[probe_values_idx].0 += 1;
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
    println!("Number of clock ticks: {}", probe_values_in_cycle.len());

    print_stacks(&probe_values_in_cycle, &stacks, args.num_print_cycles);
    let flame_info = compute_flame(&stacks)?;
    write_flame(&flame_info, args.scaled_flame_out, args.flat_flame_out)?;

    Ok(())
}
