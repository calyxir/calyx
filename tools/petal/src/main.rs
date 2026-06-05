mod design;

use anyhow::{Context, Ok, Result};
use baa::{BitVecMutOps, BitVecValue};
use clap::Parser;
use wellen::*;

use crate::design::Design;

#[derive(Parser, Debug)]
#[command(name = "petal")]
#[command(author = "Ayaka Yorihiro <ayaka@cs.cornell.edu>")]
#[command(version)]
#[command(about = "Calyx profiler.", long_about = None)]
struct Args {
    #[arg(value_name = "WAV", index = 1)]
    filename: String,
}

fn read_bool(signal: SignalRef, values: &SignalMap<SignalValue>) -> bool {
    let value_ref: SignalValueRef = values.get(&signal).unwrap().into();
    if let SignalValueRef::BitVec(b) = value_ref {
        b.get_bit(0).as_ascii() == '1'
    } else {
        panic!("Clock needs to be a bitvector!");
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let opts = LoadOptions {
        multi_thread: true,
        remove_scopes_with_empty_name: false,
    };

    let mut wav = wellen::stream::read_from_file(&args.filename, &opts)
        .with_context(|| format!("Failed to load {}", args.filename))?;

    // static tree
    let design = Design::new(wav.hierarchy())?;

    // all probe signals we would need to track
    let signals = design.get_signals();

    let filter = wellen::stream::Filter::include_signals(&signals);

    let mut clock_previous = true;

    // One bit vector for each cycle. Each index in the BitVecValue corresponds to a probe.
    // If it is active, the index will contain 1.
    let mut probe_values: Vec<BitVecValue> = vec![];

    // populate probe_values on the clock's falling edge
    wav.stream_time_steps(filter, |time, values| {
        let c = read_bool(design.clk(), values);
        if c && !clock_previous {
            let mut value = BitVecValue::zero(signals.len() as u32);
            for (idx, &signal) in signals.iter().enumerate() {
                let probe_value = read_bool(signal, values);
                if probe_value {
                    value.set_bit(idx as u32);
                }
            }
            probe_values.push(value);
        }
        clock_previous = c;
    })
    .with_context(|| format!("Failed to stream"))?;
    println!("Number of clock ticks: {}", probe_values.len());

    // Compute the trace (stacks for each active cycle) from probe_values
    let mut cycle_count = -1;
    for (_, value) in probe_values.iter().enumerate() {
        // .take(15) // for debugging
        let stacks = design.compute(value)?;
        if !stacks.is_empty() {
            cycle_count += 1;
            println!("{cycle_count}");
            for stack in stacks {
                let stack_str = stack.join(", ");
                println!("	[{stack_str}]");
            }
        }
    }

    Ok(())
}
