mod design;

use design::{get_var, parse_probe_name};

use anyhow::{Context, Ok, Result};
use baa::BitVecMutOps;
use baa::BitVecOps;
use clap::Parser;
use rustc_hash::FxHashMap;
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

    let design = Design::new(wav.hierarchy())?;

    let signals = design.get_signals();

    let filter = wellen::stream::Filter::include_signals(&signals);

    let mut clock_previous = true;

    let mut probe_values: Vec<_> = vec![];

    wav.stream_time_steps(filter, |time, values| {
        let c = read_bool(design.clk(), values);
        if c && !clock_previous {
            let mut value = baa::BitVecValue::zero(signals.len() as u32);
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

    println!("probe value len: {}", probe_values.len());

    let mut cycle_count = -1;
    for (idx, value) in probe_values.iter().enumerate().take(15) {
        let stacks = design.compute(value)?;
        if !stacks.is_empty() {
            cycle_count += 1;
            println!("{cycle_count}");
            for stack in stacks {
                println!("{stack:?}");
            }
        }
    }

    Ok(())
}
