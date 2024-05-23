//! Converts from an egglog AST to Calyx.

use crate::utils;
use calyx_backend::{Backend, CalyxEggBackend};
use calyx_ir::{self, Component, PrimitiveInfo};
use calyx_ir::{Assignment, RRC, WRC};
use egglog::{ast::Literal, match_term_app, EGraph, Term, TermDag};
use std::io::Write;
use std::os::macos::raw::stat;
use std::rc::Rc;
use std::{
    any::Any,
    clone,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::Path,
};
use tempfile::{tempfile, NamedTempFile};

// TODO(cgyurgyik): Eventually, we want to emit the entire Calyx program.
// This will require storing the parsed Calyx program, updating the control
// schedule à la egglog, and then determining whether the new control schedule
// introduced any new groups. If so, they should be built and added to the
// component. Finally, the new component should be emitted.
pub struct CalyxEgg<'a> {
    component: &'a calyx_ir::Component,
}

impl<'a> CalyxEgg<'a> {
    fn new(self, component: &'a calyx_ir::Component) -> CalyxEgg<'a> {
        CalyxEgg { component }
    }
}

pub fn round_trip(
    calyx_component: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Write the Calyx component to a temporary file.
    let mut temporary_file = tempfile::NamedTempFile::new()?;
    writeln!(temporary_file, "{}", calyx_component)?;

    // Set up path for workspace and library.
    let library_path =
        Path::new(option_env!("CALYX_PRIMITIVES_DIR").unwrap_or("../"));
    let ws = calyx_frontend::Workspace::construct(
        &Some(temporary_file.path().to_path_buf()),
        &library_path,
    )
    .unwrap();

    // The final output file. We snag the imports before trhe workspace is moved into the context.
    let mut f = tempfile::NamedTempFile::new()?;
    for import in ws.original_imports.iter() {
        writeln!(f, "import \"{}\";", import)?;
    }

    // Consume the Calyx program and produce an IR.
    let file = tempfile::NamedTempFile::new()?;
    let path = file.into_temp_path();
    let mut output = calyx_utils::OutputFile::File(path.to_path_buf());
    let context = calyx_ir::from_ast::ast_to_ir(ws).unwrap();
    let length: usize = context.components.len();
    if length > 1 {
        return Err(
            format!("expected 1 component, received: {}", length).into()
        );
    }

    let Some(component) = context.components.first() else {
        return Err(format!("no component successful in parsing").into());
    };

    // Retrieve the datatypes and rulesets.
    let mut egglog_program =
        utils::read_from(utils::RewriteRule::CalyxControl)?;
    // Emit and retrieve the Calyx program converted to egglog.
    CalyxEggBackend::emit(&context, &mut output).unwrap();
    egglog_program.push_str(&fs::read_to_string(path)?);
    // Retrieve the schedule for these rules (hard-coded, more or less).
    egglog_program
        .push_str(&utils::run_schedule(&[utils::RewriteRule::CalyxControl])?);

    // Parse and run the e-graph.
    let mut egraph = EGraph::default();
    egraph.parse_and_run_program(&egglog_program)?;

    let identifier: &str = component.name.id.into();

    let mut termdag = TermDag::default();
    let (sort, value) = egraph
        .eval_expr(&egglog::ast::Expr::Var((), identifier.into()))
        .unwrap();
    let (_, extracted) = egraph.extract(value, &mut termdag, &sort);

    let indent_level = 2;
    let control = program_from_egglog(indent_level + 2, extracted, &termdag)?;

    writeln!(f, "component {}() -> () {{", identifier)?;
    writeln!(f, "{}cells {{", " ".repeat(indent_level))?;
    for cell /*: &Rc<RefCell<Cell>>*/ in component.cells.iter() {
        let cell_reference = cell.borrow();
        calyx_ir::Printer::write_cell(
            &cell_reference,
            indent_level + 2,
            &mut f,
        )?;
    }
    writeln!(f, "{}}}", " ".repeat(indent_level))?;
    writeln!(f, "{}wires {{", " ".repeat(indent_level))?;
    for group in component.groups.iter() {
        let group_reference = group.borrow();
        calyx_ir::Printer::write_group(
            &group_reference,
            indent_level + 2,
            &mut f,
        )?;
        writeln!(f)?;
    }
    for static_group in component.static_groups.iter() {
        let group_reference = static_group.borrow();
        calyx_ir::Printer::write_static_group(
            &group_reference,
            indent_level + 2,
            &mut f,
        )?;
        writeln!(f)?;
    }
    for comb_group in component.comb_groups.iter() {
        let group_reference = comb_group.borrow();
        calyx_ir::Printer::write_comb_group(
            &group_reference,
            indent_level + 2,
            &mut f,
        )?;
        writeln!(f)?;
    }
    for wire in component.continuous_assignments.iter() {
        return Err(format!("wires not supported: {:?}", wire).into());
    }
    writeln!(f, "{}}}", " ".repeat(indent_level))?;
    writeln!(f, "{}control {{", " ".repeat(indent_level))?;
    write!(f, "{}", control)?;
    writeln!(f, "{}}}", " ".repeat(indent_level))?;
    writeln!(f, "}}")?;

    f.seek(SeekFrom::Start(0))?;
    let mut buf: String = String::new();
    f.read_to_string(&mut buf)?;
    Ok(buf)
}

pub struct EggToCalyx<'a> {
    termdag: &'a TermDag,
}

pub fn program_from_egglog(
    indent_level: usize,
    program: Term,
    termdag: &TermDag,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut temporary_file = tempfile::NamedTempFile::new()?;
    let mut handler = temporary_file.as_file_mut();

    let mut converter = EggToCalyx { termdag };
    converter.emit(&mut handler, indent_level, program)?;

    let mut buf = String::new();
    handler.seek(SeekFrom::Start(0))?;
    handler.read_to_string(&mut buf)?;
    Ok(buf)
}

impl<'a> EggToCalyx<'a> {
    fn emit(
        &mut self,
        f: &mut File,
        indent_level: usize,
        expr: Term,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match expr {
            Term::Lit(lit) => self.emit_literal(f, indent_level, lit),
            Term::App(..) => self.emit_app(f, indent_level, expr),
            Term::Var(..) => todo!("not implemented: Var, {:?}", expr),
        }
    }

    fn emit_literal(
        &mut self,
        f: &mut File,
        indent_level: usize,
        expr: Literal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match expr {
            Literal::Int(i) => {
                write!(f, "{}{}", " ".repeat(indent_level), i)?;
                Ok(())
            }
            Literal::String(s) => {
                write!(f, "{}{}", " ".repeat(indent_level), s)?;
                Ok(())
            }
            Literal::F64(..) | Literal::Bool(..) | Literal::Unit => {
                panic!("unexpected literal: {:?}", expr)
            }
        }
    }

    /// For now, this only produces the control schedule.
    fn emit_app(
        &mut self,
        f: &mut File,
        indent_level: usize,
        expr: Term,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match_term_app!(expr.clone();
        {
            ("Cell", [_]) => { Ok(()) }
            ("Group", [name, _]) => {
                // Just emit the name for the respective Enable.
                self.emit(f, indent_level, self.termdag.get(*name))
            }
            ("Enable", [group, attributes]) => {
                self.emit(f, indent_level, self.termdag.get(*attributes))?;
                self.emit(f, indent_level, self.termdag.get(*group))?;
                writeln!(f, ";")?;
                Ok(())

            }
            ("Attributes", [mapping]) => {
                self.emit(f,indent_level, self.termdag.get(*mapping))
            }
            ("Par", [attributes, list]) => {
                self.emit(f,indent_level, self.termdag.get(*attributes))?;
                writeln!(f,"{}par {{", " ".repeat(indent_level))?;
                self.emit(f, indent_level + 2, self.termdag.get(*list))?;
                writeln!(f, "{}}}", " ".repeat(indent_level))?;
                Ok(())
            }
            ("Seq", [attributes, list]) => {
                self.emit(f,indent_level, self.termdag.get(*attributes))?;
                writeln!(f,"seq {{")?;
                self.emit(f, indent_level + 2, self.termdag.get(*list))?;
                writeln!(f, "{}}}", " ".repeat(indent_level))?;
                Ok(())
            }
            ("Cons", [x, xs]) => {
                self.emit(f,indent_level, self.termdag.get(*x))?;
                self.emit(f,indent_level, self.termdag.get(*xs))
            }
            ("map-insert", [map, k, v]) => {
                write!(f,"@")?;
                self.emit(f, 0,self.termdag.get(*k))?;
                write!(f,"(")?;
                self.emit(f, 0, self.termdag.get(*v))?;
                write!(f,")")?;
                write!(f, " ")?;
                self.emit(f, 0, self.termdag.get(*map))?;
                Ok(())
            }
            ("map-empty", []) |
            ("Nil", []) => {
                Ok(())
            }
            (&_, _) => todo!("unexpected: {:?}", expr)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    // String equality without whitespace.
    fn assert_str_equal(a: &str, b: &str) {
        assert_eq!(
            a.split_ascii_whitespace().collect::<Vec<_>>(),
            b.split_ascii_whitespace().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_par_collapse() -> utils::Result {
        assert_str_equal(
            &round_trip(
                r#"
                import "primitives/core.futil";
                import "primitives/memories/comb.futil";

                component main() -> () {
                  cells {
                    @external(1) in = comb_mem_d1(32, 1, 1);
                    a = std_reg(32);
                    b = std_reg(32);
                  }
                  wires {
                    group A {
                      a.write_en = 1'b1;
                      in.addr0 = 1'b0;
                      a.in = in.read_data;
                    } 
                    group B {
                      b.write_en = 1'b1;
                      b.in = a.out;
                    }
                  }
                  control {
                    par { par { A; B; } }
                  }
                }
            "#,
            )?,
            r#"
            import "primitives/core.futil";
            import "primitives/memories/comb.futil";

            component main() -> () {
            cells {
              @external in = comb_mem_d1(32, 1, 1);
              a = std_reg(32);
              b = std_reg(32);
            }
            wires {
              group A {
                a.write_en = 1'd1;
                in.addr0 = 1'd0;
                a.in = in.read_data;
              }
              group B {
                b.write_en = 1'd1;
                b.in = a.out;
              }
            }
            control {
              par {
                A;
                B;
              }
            }
          }"#,
        );
        Ok(())
    }
}
