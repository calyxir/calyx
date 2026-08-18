use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use argh::FromArgs;
use baa::{BitVecMutOps, BitVecValue};
use num_ir::short::TryFromShort;
use num_ir::typing::{TypeClass, TypeSpec};
use num_ir::vbfp::FixedDef;
use rand::SeedableRng;
use rand::prelude::*;

#[derive(Debug, thiserror::Error)]
enum DataGenErr {
    #[error("not fixed point")]
    NotFixed,
}

fn read_short_wrap(s: &str) -> Result<TypeSpec, String> {
    TypeSpec::read_short_t(s).map_err(|e| e.to_string())
}

#[derive(Debug, FromArgs)]
/// Generate random data according to parameters. Randomly-generated numbers are not sound for cryptography.
struct Opts {
    /// the type of the data to generate
    #[argh(positional, from_str_fn(read_short_wrap))]
    short_t: TypeSpec,

    /// the number of numbers to generate
    #[argh(positional)]
    amt: usize,

    /// destination path
    #[argh(option, short = 'o')]
    out_path: Option<PathBuf>,

    /// abs. bound of range of random values
    #[argh(option, long = "fixed-bound")]
    fixed_bound: Option<f64>,

    /// a seed for the RNG.
    #[argh(option, short = 'd')]
    seed: Option<u64>,

    /// separator between elements. defaults to "\n"
    #[argh(option, short = 's', long = "--sep")]
    #[argh(default = "String::from(\"\n\")")]
    sep: String,

    /// print output as hex strings, regardless of input type.
    #[argh(switch, short = 'x')]
    hex: bool,

    /// when -x is active, also pretty-print values to stderr.
    #[argh(switch, short = 'e')]
    use_stderr: bool,
}

fn main() -> Result<(), DataGenErr> {
    let opts: Opts = argh::from_env();

    let mut rng = if let Some(seed) = opts.seed {
        SmallRng::seed_from_u64(seed)
    } else {
        SmallRng::from_rng(&mut rand::rng())
    };
    let mut filebuf = if let Some(p) = opts.out_path {
        File::create(p).map(BufWriter::new).unwrap()
    } else {
        panic!("no output");
    };

    let mut bitvec_buf = BitVecValue::zero(opts.short_t.width as u32);
    let mut fd: Option<FixedDef> = None;
    if opts.fixed_bound.is_some() {
        let TypeClass::Fixed { exp_mag: e } = opts.short_t.class else {
            return Err(DataGenErr::NotFixed);
        };
        let fixed_equiv = num_ir::vbfp::FixedDef {
            total_size: opts.short_t.width,
            exp_mag: e,
            signed: opts.short_t.signed,
        };
        fd = Some(fixed_equiv)
    }
    for _ in 0..opts.amt {
        if let Some(ref d) = fd {
            bitvec_buf =
                d.rand_fixed_bounded(opts.fixed_bound.unwrap(), &mut rng);
        } else {
            bitvec_buf.randomize(&mut rng);
        }
        let print_s = if opts.hex {
            &opts
                .short_t
                .write_hexstring(&bitvec_buf, num_ir::typing::Endian::Little)
        } else {
            &opts
                .short_t
                .write_string(&bitvec_buf, num_ir::typing::Endian::Little)
        };
        use std::io::Write;
        write!(filebuf, "{}{}", print_s, opts.sep).unwrap();

        if opts.use_stderr && opts.hex {
            eprintln!(
                "{}",
                opts.short_t
                    .write_string(&bitvec_buf, num_ir::typing::Endian::Little)
            )
        }
    }
    Ok(())
}
