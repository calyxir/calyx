use crate::interp;
use crate::shared_env;

use crate::ir::Btor2InstrContents;
use crate::ir::{self, SortType};
use btor2tools::Btor2Line;
use btor2tools::Btor2Parser;
use std::collections::HashMap;
use std::path::Path;

use bitvec::prelude::*;

pub type BitString = BitVec<usize, Lsb0>;

fn slice_to_usize(slice: &BitSlice) -> usize {
    let mut ans: usize = 0;
    for i in 0..slice.len() {
        if slice[i] {
            ans += 1 << i;
        }
    }
    ans
}

// crappy thing that makes it work: no longer store lines, instead pass in reference to file path
pub struct Btor2Program<'a> {
    parser: Btor2Parser,
    path: &'a Path,
    // lines: Option<Vec<Btor2Line<'a>>>,
}

// impl Default for Btor2Program {
//     fn default() -> Self {
//         Self::new()
//     }
// }

impl<'a> Btor2Program<'a> {
    pub fn new(path: &'a str) -> Self {
        Btor2Program {
            parser: Btor2Parser::new(),
            path: Path::new(path),
        }
    }

    // pub fn load(&mut self, input_file: &str) -> Result<(), &str> {
    //     // Parse and store the btor2 file as Vec<Btor2Line>
    //     let input_path = Path::new(input_file);
    //     let btor2_lines_opt = self.parser.read_lines(input_path);
    //     match btor2_lines_opt {
    //         Err(e) => {
    //             eprintln!("{}", e);
    //             Err("Input file not found.")
    //         }
    //         Ok(btor2_lines) => {
    //             // self.lines = Option::Some(btor2_lines.collect::<Vec<_>>());
    //             Ok(())
    //         }
    //     }
    // }

    pub fn run(
        &mut self,
        inputs: HashMap<String, String>,
    ) -> Result<HashMap<String, usize>, &str> {
        let btor2_lines: Vec<Btor2Line<'_>> = self
            .parser
            .read_lines(self.path)
            .as_ref()
            .unwrap()
            .collect::<Vec<_>>();
        let mut inputs_vec = Vec::new();
        for (name, val) in &inputs {
            inputs_vec.push(format!("{}={} ", name, val));
        }

        let ir_lines = ir::convert_to_ir(btor2_lines);

        let node_sorts = ir_lines
            .iter()
            .map(|line| match line.contents {
                Btor2InstrContents::Sort
                | Btor2InstrContents::Output { .. } => 0,
                _ => match line.sort {
                    SortType::Bitvec { width } => width,
                    SortType::Array { .. } => 0, // TODO: handle arrays
                },
            })
            .collect::<Vec<_>>();

        let mut s_env = shared_env::SharedEnvironment::new(node_sorts);

        // Parse inputs
        match interp::parse_inputs(&mut s_env, &ir_lines, &inputs_vec) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("{}", e);
                return Err("Inputs invalid.");
            }
        };

        // Main interpreter loop
        let result = interp::interpret(ir_lines.iter(), &mut s_env);
        match result {
            Ok(()) => {}
            Err(e) => {
                eprintln!("{}", e);
                return Err("Runtime error in BTOR2 program.");
            }
        }

        let mut output_map = HashMap::new();

        ir_lines.iter().for_each(|line| {
            if let Btor2InstrContents::Output { name, arg1 } = &line.contents {
                let output_name = name.clone();
                let src_node_idx = *arg1;
                let output_val = s_env.get(src_node_idx);

                output_map.insert(output_name, slice_to_usize(output_val));
            }
        });

        Ok(output_map)
    }
}
