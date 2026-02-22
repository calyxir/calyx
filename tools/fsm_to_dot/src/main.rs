use argh::FromArgs;
use calyx_frontend as frontend;
use calyx_ir::{self as ir, Printer};
use calyx_opt::pass_manager::{PassManager, PassResult};
use petgraph::dot::Dot;
use petgraph::graph::DiGraph;
use std::path::PathBuf;

#[derive(Debug, FromArgs)]
/// Convert Calyx FSMs to DOT format for visualization
struct Args {
    /// path to the Calyx file
    #[argh(positional)]
    file_path: PathBuf,

    /// library paths (default: current directory)
    #[argh(option, short = 'l')]
    lib_path: Vec<PathBuf>,

    /// name of the component to visualize (default: main)
    #[argh(option, short = 'c', default = "String::from(\"main\")")]
    component: String,

    /// output file path (default: stdout)
    #[argh(option, short = 'o')]
    output: Option<PathBuf>,
}

/// Export an FSM to DOT format
fn fsm_to_dot(fsm: &ir::FSM) -> String {
    let mut graph: DiGraph<String, String> = DiGraph::new();
    let num_states = fsm.assignments.len();

    // Create nodes for each state
    let nodes: Vec<_> = (0..num_states)
        .map(|i| graph.add_node(format!("S{}", i)))
        .collect();

    // Add edges based on transitions
    for (state, transition) in fsm.transitions.iter().enumerate() {
        match transition {
            ir::Transition::Unconditional(next_state) => {
                graph.add_edge(
                    nodes[state],
                    nodes[*next_state as usize],
                    "".to_string(),
                );
            }
            ir::Transition::Conditional(conds) => {
                for (guard, next_state) in conds {
                    let guard_str = Printer::guard_str(guard);
                    graph.add_edge(
                        nodes[state],
                        nodes[*next_state as usize],
                        guard_str,
                    );
                }
            }
        }
    }

    format!("{:?}", Dot::with_config(&graph, &[]))
}

/// Find all FSMs in a component
fn find_fsms(component: &ir::Component) -> Vec<(String, ir::RRC<ir::FSM>)> {
    component
        .fsms
        .iter()
        .map(|fsm_ref| {
            let fsm = fsm_ref.borrow();
            (fsm.name().to_string(), ir::RRC::clone(fsm_ref))
        })
        .collect()
}

fn main() -> PassResult<()> {
    let args: Args = argh::from_env();

    // Parse the Calyx program
    let lib_paths: Vec<_> = if args.lib_path.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.lib_path
    };

    let ws = frontend::Workspace::construct(&Some(args.file_path), &lib_paths)?;
    let mut ctx = ir::from_ast::ast_to_ir(
        ws,
        ir::from_ast::AstConversionConfig::default(),
    )?;

    // Run the debug pass of choice to test to generate FSMs
    let pm = PassManager::default_passes()?;
    pm.execute_plan(&mut ctx, &["med-fsm".to_string()], &[], &[], false)?;

    // Find the specified component
    let component = ctx
        .components
        .iter()
        .find(|c| c.name == args.component.as_str())
        .ok_or_else(|| {
            calyx_utils::Error::misc(format!(
                "Component '{}' not found",
                args.component
            ))
        })?;

    // Find all FSMs in the component
    let fsms = find_fsms(component);

    if fsms.is_empty() {
        eprintln!(
            "No FSMs found in component '{}'. Make sure to run fsm compilation passes.",
            args.component
        );
        eprintln!(
            "Hint: This tool works on programs with static control (static_seq, static_par, static_repeat, etc.)"
        );
        return Ok(());
    }

    // Generate DOT output for each FSM
    let mut output = String::new();
    for (name, fsm_ref) in fsms {
        let fsm = fsm_ref.borrow();
        output.push_str(&format!("// FSM: {}\n", name));
        output.push_str(&fsm_to_dot(&fsm));
        output.push_str("\n\n");
    }

    // Write output
    if let Some(output_path) = args.output {
        std::fs::write(output_path, output)?;
    } else {
        print!("{}", output);
    }

    Ok(())
}
