pub mod bvec;
pub mod cli;
pub mod error;
pub mod interp;
pub mod shared_env;

use btor2tools::Btor2Parser;
use clap::Parser;
use error::InterpResult;
use std::io;
use std::path::Path;
use std::time::Instant;
use tempfile::NamedTempFile;

fn main() -> InterpResult<()> {
    let start = Instant::now();
    let args = cli::CLI::parse();

    let btor2_file = match args.file {
        None => {
            // If no file is provided, we assume stdin
            let mut tmp = NamedTempFile::new().unwrap();
            io::copy(&mut io::stdin(), &mut tmp).unwrap();
            tmp.path().to_path_buf()
        }
        Some(input_file_path) => {
            Path::new(input_file_path.as_str()).to_path_buf()
        }
    };

    // Parse and store the btor2 file as Vec<Btor2Line>
    let mut parser = Btor2Parser::new();
    let btor2_lines =
        parser.read_lines(&btor2_file).unwrap().collect::<Vec<_>>();

    // take the btor2lines and convert them into normal lines

    for _ in 0..args.num_repeat {
        // Collect node sorts
        let node_sorts = btor2_lines
            .iter()
            .map(|line| match line.tag() {
                btor2tools::Btor2Tag::Sort | btor2tools::Btor2Tag::Output => 0,
                _ => match line.sort().content() {
                    btor2tools::Btor2SortContent::Bitvec { width } => {
                        usize::try_from(width).unwrap()
                    }
                    btor2tools::Btor2SortContent::Array { .. } => 0, // TODO: handle arrays
                },
            })
            .collect::<Vec<_>>();

        // Init environment
        // let mut env = interp::Environment::new(btor2_lines.len() + 1);
        let mut s_env = shared_env::SharedEnvironment::new(node_sorts);

        // Parse inputs
        match interp::parse_inputs(&mut s_env, &btor2_lines, &args.inputs) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };

        // Main interpreter loop
        interp::interpret(btor2_lines.iter(), &mut s_env)?;

        // Print result of execution
        if !args.profile {
            println!("{}", s_env);

            // Extract outputs
            btor2_lines.iter().for_each(|line| {
                if let btor2tools::Btor2Tag::Output = line.tag() {
                    let output_name =
                        line.symbol().unwrap().to_string_lossy().into_owned();
                    let src_node_idx = line.args()[0] as usize;
                    let output_val = s_env.get(src_node_idx);

                    println!("{}: {}", output_name, output_val);
                }
            });
        }
    }

    // print to stderr the time it took to run
    let duration = start.elapsed();
    eprintln!("Time elapsed: {} µs", duration.as_micros());

    Ok(())
}
