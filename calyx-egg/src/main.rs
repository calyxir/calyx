use egglog::EGraph;
use std::error::Error;
use std::fs;
use std::process::Command as _Command;

fn main() -> Result<(), Box<dyn Error>> {
    let mut egraph = EGraph::default();
    let path: Option<String> = std::env::args()
        .nth(1)
        .or(Some("calyx-egg/calyx.egg".to_string()));
    let input: String = fs::read_to_string(path.as_ref().unwrap())?;
    let program: Vec<egglog::ast::Command> =
        egraph.parse_program(&input).unwrap();

    for command in program.clone() {
        println!("{}", command);
    }
    egraph
        .run_program(program)
        .unwrap_or_else(|e| panic!("\n{}\n", e));

    // If `-d` is passed as the second argument, output the dot file.
    let binding: String = path.unwrap();
    let s: &str = binding.strip_suffix(".egg").unwrap();

    let _ = "-d".to_string();
    if let Some(_) = std::env::args().nth(2) {
        _Command::new("egglog")
            .arg(s.to_owned() + ".egg")
            .arg("--to-svg")
            .output()?;

        _Command::new("open").arg(s.to_owned() + ".svg").output()?;
    }

    Ok(())
}
