mod control;
mod design;
mod shared_cells;
mod visuals;

use anyhow::{Context, Ok, Result, anyhow};
use baa::{BitVecMutOps, BitVecValue};
use clap::Parser;
use indexmap::IndexMap;
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

pub type Stacks = IndexMap<BitVecValue, (u64, Vec<Stack>)>;

fn collect_stacks(
    design: &Design,
    probe_values: &[BitVecValue],
) -> Result<Stacks> {
    // Compute the trace (stacks for each active cycle) from probe_values
    let mut out = IndexMap::default();
    for value in probe_values {
        if let Some((count, _)) = out.get_mut(value) {
            *count += 1;
        } else {
            let stacks = design.compute_cycle_trace(value)?;
            out.insert(value.clone(), (1, stacks));
        };
    }
    Ok(out)
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

    // One bit vector for each cycle. Each index in the BitVecValue corresponds to a probe.
    // If it is active, the index will contain 1.
    let mut probe_values: Vec<BitVecValue> = vec![];

    // populate probe_values on the clock's falling edge
    let clock_signal_ref = design.clk();
    let signal_bits = FxHashMap::from_iter(
        signals
            .iter()
            .enumerate()
            .map(|(idx, &signal)| (signal, idx as u32)),
    );
    let mut value = BitVecValue::zero(signals.len() as u32);
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
                    value.set_bit(idx);
                } else {
                    value.clear_bit(idx);
                }
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

    let stacks = collect_stacks(&design, &probe_values)?;
    print_stacks(&probe_values, &stacks, args.num_print_cycles);
    let flame_info = compute_flame(&stacks)?;
    write_flame(&flame_info, args.scaled_flame_out, args.flat_flame_out)?;

    Ok(())
}
