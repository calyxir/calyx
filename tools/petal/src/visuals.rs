use crate::design::Stack;
use anyhow::{Ok, Result};
use rustc_hash::FxHashMap;
use std::fs::{File};
use std::io::{BufWriter, Write};

pub struct FlameCount {
    scaled: f64,
    flat: f64,
}

pub fn compute_flame(
    cycle_trace: Vec<Stack>,
    out: &mut FxHashMap<String, FlameCount>,
) -> Result<()> {
    let mut normalizer = (1.0f64) / (cycle_trace.len() as f64);
    // attempt to mirror Python Petal's rounding to three decimal places.
    normalizer = (normalizer * 1000.0).round() / 1000.0;
    let mut stack_strings: Vec<String> =
        cycle_trace.iter().map(|stack| stack.join(";")).collect();
    stack_strings.sort();
    let mut acc = 0;
    for stack_string in stack_strings {
        let scaled = if acc == cycle_trace.len() - 1 {
            (1.0f64) - (normalizer * ((cycle_trace.iter().len() - 1) as f64))
        } else {
            normalizer
        };
        if let Some(curr) = out.get_mut(&stack_string) {
            curr.scaled += scaled;
            curr.flat += 1.0;
        } else {
            out.insert(
                stack_string,
                FlameCount {
                    scaled,
                    flat: 1.0,
                },
            );
        }
        acc += 1;
    }
    Ok(())
}

fn get_buffer(path_opt: Option<String>) -> Result<Option<BufWriter<File>>> {
    if let Some(path) = path_opt {
        let sf = File::create_new(path)?;
        Ok(Some(BufWriter::new(sf)))
    } else {
        Ok(None)
    }
}

pub fn write_flame(
    flame_info: FxHashMap<String, FlameCount>,
    scaled_flame_opt: Option<String>,
    folded_flame_opt: Option<String>,
) -> Result<()> {
    let scaled_buffer = get_buffer(scaled_flame_opt)?;
    if let Some(mut buffer) = scaled_buffer {
        for (s, f) in flame_info.iter() {
            // scaled flame graphs should be multiplied by 1000 to prevent the flame graph script
            // erroring out.
            writeln!(buffer, "{s} {:.1}", (f.scaled * 1000.0))?;
        }
    }
    let folded_buffer = get_buffer(folded_flame_opt)?;
    if let Some(mut buffer) = folded_buffer {
        for (s, f) in flame_info.iter() {
            writeln!(buffer, "{s} {}", f.flat)?;
        }
    }
    Ok(())
}
