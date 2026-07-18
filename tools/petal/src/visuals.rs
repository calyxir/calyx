use crate::Stacks;
use crate::design::Stack;
use anyhow::{Ok, Result};
use indexmap::IndexMap;
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// Represents the flame graph values (how much for a single stack).
pub struct FlameCount {
    scaled: f64,
    flat: f64,
}

pub fn compute_flame_calyx(
    stacks: &Stacks,
) -> Result<Vec<(String, FlameCount)>> {
    let filtered: Vec<(&u64, &Vec<Stack>)> = stacks
        .values()
        .filter(|(_, s, _, _)| !s.is_empty())
        .map(|(c, s, _, _)| (c, s))
        .collect();
    compute_flame(filtered)
}

/// Update the flame graph count given the trace for a single cycle.
pub fn compute_flame(
    counts_and_stacks: Vec<(&u64, &Vec<Stack>)>,
) -> Result<Vec<(String, FlameCount)>> {
    let mut out =
        IndexMap::<String, FlameCount>::with_capacity(counts_and_stacks.len());
    // now we only look at the stacks for each unique value
    for (count, stacks) in counts_and_stacks {
        let factor = *count as f64;
        let mut stack_strings: Vec<String> =
            stacks.iter().map(|stack| stack.join(";")).collect();
        stack_strings.sort();

        // scale by the number of stacks
        let num_stacks = stack_strings.len();
        let mut normalizer = (1.0f64) / (num_stacks as f64);
        // attempt to mirror Python Petal's rounding to three decimal places.
        normalizer = (normalizer * 1000.0).round() / 1000.0;

        let last_idx = num_stacks - 1;
        for (idx, stack_string) in stack_strings.into_iter().enumerate() {
            let scaled = if idx == last_idx {
                // ensure that everything sums up to one
                1.0 - normalizer * last_idx as f64
            } else {
                normalizer
            };
            if let Some(curr) = out.get_mut(&stack_string) {
                curr.scaled += scaled * factor;
                curr.flat += factor;
            } else {
                out.insert(
                    stack_string,
                    FlameCount {
                        scaled: scaled * factor,
                        flat: factor,
                    },
                );
            }
        }
    }

    Ok(out.into_iter().collect())
}

// /// Helper function to write_flame() that returns a BufWriter for a flame graph if requested.
// fn get_buffer(path_opt: Option<String>) -> Result<Option<BufWriter<File>>> {
//     if let Some(path_str) = path_opt {
//         let path = Path::new(&path_str);
//         if let Some(d) = path.parent() {
//             fs::create_dir_all(d)?;
//         }
//         let sf = File::create(path)?;
//         Ok(Some(BufWriter::new(sf)))
//     } else {
//         Ok(None)
//     }
// }

fn write_flame(
    flame_info: &[(String, FlameCount)],
    path: &PathBuf,
    scaled: bool,
) -> Result<()> {
    let path = Path::new(path);
    if let Some(d) = path.parent() {
        fs::create_dir_all(d)?;
    }
    let sf = File::create(path)?;
    let mut buffer = BufWriter::new(sf);
    // NOTE: we don't sort keys internally!
    for (s, f) in flame_info {
        if scaled {
            writeln!(buffer, "{s} {:.1}", f.scaled * 1000.0)?;
        } else {
            writeln!(buffer, "{s} {}", f.flat)?;
        }
    }
    Ok(())
}

/// Writes a scaled/flattened flame graph to scaled_flame_opt/folded_flame_opt if requested.
pub fn write_flames(
    flame_info: &[(String, FlameCount)],
    scaled_flame_opt: Option<PathBuf>,
    folded_flame_opt: Option<PathBuf>,
) -> Result<()> {
    if let Some(scaled_flame) = scaled_flame_opt {
        write_flame(&flame_info, &scaled_flame, true)?;
    }

    if let Some(folded_flame) = folded_flame_opt {
        write_flame(&flame_info, &folded_flame, false)?;
    }

    // let scaled_buffer = get_buffer(scaled_flame_opt)?;
    // // sort keys to get deterministic output.
    // if let Some(mut buffer) = scaled_buffer {
    //     for (s, f) in flame_info {
    //         writeln!(buffer, "{s} {:.1}", f.scaled * 1000.0)?;
    //     }
    // }
    // let folded_buffer = get_buffer(folded_flame_opt)?;
    // if let Some(mut buffer) = folded_buffer {
    //     for (s, f) in flame_info {
    //         writeln!(buffer, "{s} {}", f.flat)?;
    //     }
    // }
    Ok(())
}

pub fn write_calyx_flames(
    stacks: &Stacks,
    scaled_flame_opt: Option<String>,
    folded_flame_opt: Option<String>,
) -> Result<()> {
    let flame_info: Vec<(String, FlameCount)> = compute_flame_calyx(stacks)?;
    let s = scaled_flame_opt.map(|s| PathBuf::from(s));
    let f = folded_flame_opt.map(|f| PathBuf::from(f));
    write_flames(&flame_info, s, f)
}
