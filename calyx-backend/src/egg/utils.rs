use super::egg_to_calyx;
use egglog::{EGraph, Term, TermDag};
use main_error::MainError;
use std::{fmt, io};
use strum_macros::EnumIter;

#[derive(Debug, Clone, Copy, PartialEq, EnumIter)]
pub enum RewriteRule {
    Analysis = 0,
    FanOutReduction = 1,
    CollapseControl = 2,
    ParToSeq = 3,
    SplitSeq = 4,
    StaticCompaction = 5,
}

impl fmt::Display for RewriteRule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RewriteRule::Analysis => write!(f, "analysis"),
            RewriteRule::FanOutReduction => write!(f, "fan-out"),
            RewriteRule::CollapseControl => write!(f, "collapse-control"),
            RewriteRule::ParToSeq => write!(f, "par-to-seq"),
            RewriteRule::SplitSeq => write!(f, "split-seq"),
            RewriteRule::StaticCompaction => write!(f, "static-compaction"),
        }
    }
}

pub type Result = std::result::Result<(), MainError>; // xxx

pub fn extract_egglog(
    display: bool,
    identifier: &str,
    program: &String,
) -> (Term, TermDag) {
    let mut egraph = EGraph::default();
    egraph.parse_and_run_program(program).unwrap_or_else(|_| {
        panic!("failed to parse and run e-graph for program: {}", program)
    });

    if display {
        let serialized = egraph.serialize_for_graphviz(true, 100, 100);
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.into_temp_path().with_extension("svg");
        serialized.to_svg_file(path.clone()).unwrap();
        std::process::Command::new("open")
            .arg(path.to_str().unwrap())
            .output()
            .unwrap();
    }

    let mut termdag = TermDag::default();
    let (sort, value) = egraph
        .eval_expr(&egglog::ast::Expr::Var((), identifier.into()))
        .unwrap_or_else(|_| {
            panic!(
                "unexpected failure of e-graph extraction for component: {}. Original egglog program: {}",
                identifier, program
            )
        });
    let (_, extracted) = egraph.extract(value, &mut termdag, &sort);
    (extracted, termdag)
}

pub fn convert_component<F: io::Write>(
    component: &calyx_ir::Component,
    control: Term,
    termdag: &TermDag,
    f: &mut F,
) -> Result {
    let indent_level = 2;
    // TODO(cgyurgyik): How to extract the imports?
    let imports = [
        "import \"primitives/binary_operators.futil\";",
        "import \"primitives/core.futil\";",
        "import \"primitives/math.futil\";",
        "import \"primitives/memories/comb.futil\";",
        "import \"primitives/memories/seq.futil\";",
    ];
    writeln!(f, "{}", imports.join("\n"))?;
    writeln!(f)?;

    writeln!(f, "component {}() -> () {{", component.name.id)?;
    writeln!(f, "{}cells {{", " ".repeat(indent_level))?;
    for cell /*: &Rc<RefCell<Cell>>*/ in component.cells.iter() {
        let cell_reference = cell.borrow();
        calyx_ir::Printer::write_cell(
            &cell_reference,
            indent_level + 2,
            f,
        )?;
    }
    writeln!(f, "{}}}", " ".repeat(indent_level))?;
    writeln!(f, "{}wires {{", " ".repeat(indent_level))?;
    for group in component.groups.iter() {
        let group_reference = group.borrow();
        calyx_ir::Printer::write_group(&group_reference, indent_level + 2, f)?;
        writeln!(f)?;
    }
    for static_group in component.static_groups.iter() {
        let group_reference = static_group.borrow();
        calyx_ir::Printer::write_static_group(
            &group_reference,
            indent_level + 2,
            f,
        )?;
        writeln!(f)?;
    }
    for comb_group in component.comb_groups.iter() {
        let group_reference = comb_group.borrow();
        calyx_ir::Printer::write_comb_group(
            &group_reference,
            indent_level + 2,
            f,
        )?;
        writeln!(f)?;
    }
    if let Some(wire) = component.continuous_assignments.first() {
        return Err(format!("wires not supported: {:?}", wire).into());
    }
    writeln!(f, "{}}}", " ".repeat(indent_level))?;
    writeln!(f, "{}control {{", " ".repeat(indent_level))?;

    // Now, convert the control and emit it.
    let mut converter = egg_to_calyx::EggToCalyx { termdag };
    write!(f, "{}", converter.emit_string(indent_level + 2, control)?)?;

    writeln!(f, "{}}}", " ".repeat(indent_level))?;
    writeln!(f, "}}")?;
    Ok(())
}
