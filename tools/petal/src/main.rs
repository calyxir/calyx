mod control;
mod design;
mod shared_cells;
mod visuals;

use anyhow::{Context, Ok, Result, anyhow};
use baa::{BitVecMutOps, BitVecValue};
use clap::Parser;
use rustc_hash::FxHashMap;
use wellen::{stream::SignalValues, *};

use crate::design::Design;
use crate::visuals::{FlameCount, compute_flame, write_flame};

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
    num_print_cycles: i32,
}

/// Reads a boolean value from a signal.
fn read_bool(signal: SignalRef, values: &SignalValues) -> bool {
    let value_ref: SignalValueRef = values.get(&signal).unwrap();
    if let SignalValueRef::BitVec(b) = value_ref {
        b.get_bit(0).as_ascii() == '1'
    } else {
        panic!("Signal needs to be a bitvector!");
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
    wav.stream_time_steps(filter, |_time, values, _changed| {
        let c = read_bool(design.clk(), &values);
        if c && !clock_previous {
            let mut value = BitVecValue::zero(signals.len() as u32);
            for (idx, &signal) in signals.iter().enumerate() {
                let probe_value = read_bool(signal, &values);
                if probe_value {
                    value.set_bit(idx as u32);
                }
            }
            probe_values.push(value);
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

    // Compute the trace (stacks for each active cycle) from probe_values
    let mut cycle_count: i32 = -1;
    let mut flame_info: FxHashMap<String, FlameCount> = FxHashMap::default();
    for value in probe_values.iter() {
        let stacks = design.compute_cycle_trace(value)?;
        if !stacks.is_empty() {
            cycle_count += 1;
            if args.num_print_cycles >= cycle_count {
                println!("{cycle_count}");
                for stack in stacks.iter() {
                    let stack_str = stack.join(", ");
                    println!("	[{stack_str}]");
                }
            }
            compute_flame(stacks, &mut flame_info)?;
        }
    }

    write_flame(flame_info, args.scaled_flame_out, args.flat_flame_out)?;

    Ok(())
}
