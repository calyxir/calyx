use crate::interp;
use crate::shared_env;

use btor2tools::Btor2Line;
use btor2tools::Btor2Parser;
use std::collections::HashMap;
use std::path::Path;

pub struct Btor2Program<'a> {
    parser: Btor2Parser,
    lines: Option<Vec<Btor2Line<'a>>>,
}

impl<'a> Default for Btor2Program<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Btor2Program<'a> {
    pub fn new() -> Self {
        Btor2Program {
            parser: Btor2Parser::new(),
            lines: None,
        }
    }

    pub fn load(&'a mut self, input_file: &str) -> Result<(), &str> {
        // Parse and store the btor2 file as Vec<Btor2Line>
        let input_path = Path::new(input_file);
        let btor2_lines_opt = self.parser.read_lines(input_path);
        match btor2_lines_opt {
            Err(e) => {
                eprintln!("{}", e);
                Err("Input file not found.")
            }
            Ok(btor2_lines) => {
                self.lines = Option::Some(btor2_lines.collect::<Vec<_>>());
                Ok(())
            }
        }
    }

    pub fn run(
        &'a self,
        inputs: HashMap<String, String>,
    ) -> Result<HashMap<String, String>, &str> {
        let btor2_lines = self.lines.as_ref().unwrap();
        let mut inputs_vec = Vec::new();
        for (name, val) in &inputs {
            inputs_vec.push(format!("{}={} ", name, val));
        }

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

        let mut s_env = shared_env::SharedEnvironment::new(node_sorts);

        // Parse inputs
        match interp::parse_inputs(&mut s_env, btor2_lines, &inputs_vec) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("{}", e);
                return Err("Inputs invalid.");
            }
        };

        // Main interpreter loop
        let result = interp::interpret(btor2_lines.iter(), &mut s_env);
        match result {
            Ok(()) => {}
            Err(e) => {
                eprintln!("{}", e);
                return Err("Runtime error in BTOR2 program.");
            }
        }

        let mut output_map = HashMap::new();

        btor2_lines.iter().for_each(|line| {
            if let btor2tools::Btor2Tag::Output = line.tag() {
                let output_name =
                    line.symbol().unwrap().to_string_lossy().into_owned();
                let src_node_idx = line.args()[0] as usize;
                let output_val = s_env.get(src_node_idx);

                output_map.insert(output_name, output_val.to_string());
            }
        });

        Ok(output_map)
    }
}
