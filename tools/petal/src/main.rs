mod hierarchy;

use hierarchy::{get_var, parse_probe_name};

use anyhow::{Context, Ok, Result};
use baa::BitVecMutOps;
use baa::BitVecOps;
use clap::Parser;
use rustc_hash::FxHashMap;
use wellen::*;

use crate::hierarchy::Design;

#[derive(Parser, Debug)]
#[command(name = "petal")]
#[command(author = "Ayaka Yorihiro <ayaka@cs.cornell.edu>")]
#[command(version)]
#[command(about = "Calyx profiler.", long_about = None)]
struct Args {
    #[arg(value_name = "WAV", index = 1)]
    filename: String,
}

#[derive(Debug)]
struct Vars {
    clk: SignalRef,
    main_go: SignalRef,
    main_done: SignalRef,
    probes: Vec<VarRef>,
}

impl Vars {
    fn get_signal_refs(&self, h: &Hierarchy) -> Vec<SignalRef> {
        let mut signals: Vec<_> =
            self.probes.iter().map(|&v| h[v].signal_ref()).collect();

        signals.push(self.clk);
        signals.push(self.main_go);
        signals.push(self.main_done);

        signals.sort();
        signals.dedup();

        signals
    }
}

struct ControlTree {
    main_go_idx: u32,
    main_done_idx: u32,
    main_scope: ScopeRef,
    to_index: FxHashMap<SignalRef, u32>,
    h: Hierarchy,
}

impl ControlTree {
    fn new(
        main_go_idx: u32,
        main_done_idx: u32,
        probe_signals: &[SignalRef],
        h: Hierarchy,
    ) -> Result<Self> {
        let main_scope = h
            .lookup_scope(&[&"toplevel", &"main"])
            .with_context(|| format!("Failed to find main scope"))?;
        let to_index = FxHashMap::from_iter(
            probe_signals
                .iter()
                .enumerate()
                .map(|(idx, &signal)| (signal, idx as u32)),
        );
        Ok(Self {
            main_go_idx,
            main_done_idx,
            main_scope,
            to_index,
            h,
        })
    }

    fn compute(&self, values: &baa::BitVecValue) -> Result<Vec<Vec<String>>> {
        let h = &self.h;
        let mut out = vec![];
        if (values.is_bit_set(self.main_go_idx)) {
            let mut stack = vec!["main".to_string()];
            for scope in h[self.main_scope].scopes(h) {
                let name = h[scope].name(h);
                if name.ends_with("_probe") {
                    let out = get_var(h, &h[scope], "out")?;
                    let probe_value =
                        values.is_bit_set(self.to_index[&h[out].signal_ref()]);
                    if probe_value {
                        let probe_name = parse_probe_name(name)?;
                        println!("Probe {probe_name:?} is active.")
                    }
                }
            }
        }
        Ok(out)
    }
}

fn find_probes(h: &Hierarchy) -> Result<Vars> {
    let main = h
        .lookup_scope(&[&"toplevel", &"main"])
        .with_context(|| format!("Failed to find main scope"))?;
    let clk = get_var(h, &h[main], "clk")?;
    let main_go = get_var(h, &h[main], "go")?;
    let main_done = get_var(h, &h[main], "done")?;
    let probes: Result<Vec<_>> = h
        .all_scopes()
        .filter(|s| s.name(h).ends_with("_probe"))
        .map(|s| get_var(h, s, "out"))
        .collect();
    let probes = probes?;

    println!("Found {} probes", probes.len());
    Ok(Vars {
        clk: h[clk].signal_ref(),
        main_go: h[main_go].signal_ref(),
        main_done: h[main_done].signal_ref(),
        probes,
    })
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

    let vars = find_probes(wav.hierarchy())?;
    let signals = vars.get_signal_refs(wav.hierarchy());

    let filter = wellen::stream::Filter::include_signals(&signals);

    let mut clock_previous = true;
    let probe_signals: Vec<_> = vars
        .probes
        .iter()
        .map(|&v| wav.hierarchy()[v].signal_ref())
        .collect();

    let main_go_idx = probe_signals.len() as u32;
    let main_done_idx = probe_signals.len() as u32 + 1;
    let mut probe_values: Vec<_> = vec![];

    wav.stream_time_steps(filter, |time, values| {
        let c = read_bool(vars.clk, values);
        if c && !clock_previous {
            let mut value =
                baa::BitVecValue::zero(probe_signals.len() as u32 + 2);
            for (idx, &signal) in probe_signals.iter().enumerate() {
                let probe_value = read_bool(signal, values);
                if probe_value {
                    value.set_bit(idx as u32);
                }
            }
            // accounting for main_go and main_done
            if read_bool(vars.main_go, values) {
                value.set_bit(main_go_idx);
            }
            if read_bool(vars.main_done, values) {
                value.set_bit(main_done_idx);
            }
            probe_values.push(value);
        }
        clock_previous = c;
    })
    .with_context(|| format!("Failed to stream"))?;

    println!("probe value len: {}", probe_values.len());

    let tree = ControlTree::new(
        main_go_idx,
        main_done_idx,
        &probe_signals,
        wav.hierarchy().clone(),
    )?;
    for (idx, value) in probe_values.iter().enumerate().take(15) {
        println!("{idx} {:?}", tree.compute(value));
    }

    Ok(())
}
