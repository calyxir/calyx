//! Converts from an egglog AST to Calyx.

use std::{
    any::Any,
    borrow::Borrow,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
};

use crate::utils;
use calyx_backend::CalyxEggBackend;
use calyx_ir;
use egglog::{ast::Literal, match_term_app, Term, TermDag};
use std::io::Write;
use tempfile::{tempfile, NamedTempFile};

pub struct EggToCalyx<'a> {
    termdag: &'a TermDag,
}

pub fn program_from_egglog(
    program: Term,
    termdag: &TermDag,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut temporary_file = tempfile::NamedTempFile::new()?;
    let mut handler = temporary_file.as_file_mut();

    let mut converter = EggToCalyx { termdag };
    let indent_level: usize = 0;
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
            Term::Lit(lit) => self.emit_literal(f, lit),
            Term::App(..) => self.emit_app(f, indent_level, expr),
            Term::Var(..) => todo!("not implemented: Var, {:?}", expr),
        }
    }

    fn emit_literal(
        &mut self,
        f: &mut File,
        expr: Literal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match expr {
            Literal::Int(i) => {
                write!(f, "{}", i)?;
                Ok(())
            }
            Literal::String(s) => {
                write!(f, "{}", s)?;
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
        match_term_app!(expr.clone(); {
            // Avoid indentation for these "dead" Terms.
            ("map-empty", []) |
            ("Nil", []) => {
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            (&_, _) => {
                Ok(write!(f, "{}", " ".repeat(indent_level))?)
            }
        })?;

        match_term_app!(expr.clone();
        {
            ("Cell", [_]) => { Ok(()) }
            ("Group", [name, _]) => {
                // Just emit the name for the respective Enable.
                self.emit(f, indent_level, self.termdag.get(*name))
            }
            ("Enable", [group, attributes]) => {
                self.emit(f, indent_level, self.termdag.get(*attributes))?;
                self.emit(f, /*indent_level=*/0, self.termdag.get(*group))?;
                writeln!(f, ";")?;
                Ok(())

            }
            ("Attributes", [mapping]) => {
                self.emit(f,indent_level, self.termdag.get(*mapping))
            }
            ("Seq", [attributes, list]) => {
                self.emit(f,indent_level, self.termdag.get(*attributes))?;
                writeln!(f,"seq {{")?;
                self.emit(f, indent_level + 1, self.termdag.get(*list))?;
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

    #[test]
    fn test_simple() -> utils::Result {
        // TODO(cgyurgyik): Incomplete.

        let program = utils::calyx_string_to_egglog_string(
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
                seq { seq { A; B; } }
              }
            }
        "#,
        )?;
        let mut egraph = egglog::EGraph::default();
        egraph.parse_and_run_program(&program)?;

        let identifier: &str = "egg-main";

        let mut termdag = TermDag::default();
        let (sort, value) = egraph
            .eval_expr(&egglog::ast::Expr::Var((), identifier.into()))
            .unwrap();
        let (_, extracted) = egraph.extract(value, &mut termdag, &sort);
        println!("\n{}\n", termdag.to_string(&extracted));

        let S: String = program_from_egglog(extracted, &termdag)?;
        println!("{}", S);

        Ok(())
        // converter.emit(expr)
    }
}
