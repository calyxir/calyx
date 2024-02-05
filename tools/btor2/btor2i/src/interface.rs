use btor2i::cli;
use btor2i::error::InterpResult;
use btor2i::interp;
use btor2i::shared_env;
use btor2tools::Btor2Parser;
use clap::Parser;
use std::io;
use std::path::Path;
use std::time::Instant;
use tempfile::NamedTempFile;

fn loadProgram(input_file: Path) -> Result<Vec<Btor2Line>> {
  // Parse and store the btor2 file as Vec<Btor2Line>
  let mut parser = Btor2Parser::new();
  let btor2_lines_opt = parser.read_lines(&btor2_file);
  match btor2_lines_opt {
    None => return Err("Input file not found."),
    Some(btor2_lines) => return Ok(btor2_lines.collect::<Vec<_>>()),
  }
}

fn runProgramOnInputs(
  btor2_lines: Vec<Btor2Line>,
  inputs: HashMap<String, String>,
) -> Result<HashMap<String, String>> {
  let mut input_str = "";
  for (name, val) in &inputs {
    input_str.push_str(format!("{}={} ", name, val));
  }

  let node_sorts = btor2_lines
    .iter()
    .map(|line| match line.tag() {
      btor2tools::Btor2Tag::Sort | btor2tools::Btor2Tag::Output => 0,
      _ => match line.sort().content() {
        btor2tools::Btor2SortContent::Bitvec { width } => usize::try_from(width).unwrap(),
        btor2tools::Btor2SortContent::Array { .. } => 0, // TODO: handle arrays
      },
    })
    .collect::<Vec<_>>();

  let mut s_env = shared_env::SharedEnvironment::new(node_sorts);

  // Parse inputs
  match interp::parse_inputs(&mut s_env, &btor2_lines, input_str) {
    Ok(()) => {}
    Err(e) => {
      eprintln!("{}", e);
      return Err("Inputs invalid.");
    }
  };

  // Main interpreter loop
  interp::interpret(btor2_lines.iter(), &mut s_env)?;

  let mut output_map = HashMap::new();

  btor2_lines.iter().for_each(|line| {
    if let btor2tools::Btor2Tag::Output = line.tag() {
      let output_name = line.symbol().unwrap().to_string_lossy().into_owned();
      let src_node_idx = line.args()[0] as usize;
      let output_val = s_env.get(src_node_idx);

      output_map.insert(output_name, output_val);
    }
  });

  Ok(output_map);
}
