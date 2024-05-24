use egglog::{ast::Literal, match_term_app, Term, TermDag};
use std::fs::File;
use std::io::{Read, SeekFrom};
use std::io::{Seek, Write};

pub struct EggToCalyx<'a> {
    pub termdag: &'a TermDag,
}

impl<'a> EggToCalyx<'a> {
    pub fn emit_string(
        &mut self,
        indent_level: usize,
        expr: Term,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut temporary_file = tempfile::NamedTempFile::new()?;
        let handler = temporary_file.as_file_mut();

        self.emit(handler, indent_level, expr)?;
        let mut buf = " ".repeat(indent_level);
        handler.seek(SeekFrom::Start(0))?;
        handler.read_to_string(&mut buf)?;
        Ok(buf)
    }

    pub fn emit(
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
                self.emit(f, indent_level, self.termdag.get(*mapping))
            }
            ("Par", [attributes, list]) => {
                self.emit(f, indent_level, self.termdag.get(*attributes))?;
                writeln!(f,"{}par {{", " ".repeat(indent_level))?;
                self.emit(f, indent_level + 2, self.termdag.get(*list))?;
                writeln!(f, "{}}}", " ".repeat(indent_level))?;
                Ok(())
            }
            ("Seq", [attributes, list]) => {
                self.emit(f, indent_level, self.termdag.get(*attributes))?;
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
                // TODO(cgyurgyik): Fix printing; need to collect and then space evenly.
                write!(f,"@")?;
                self.emit(f, 0, self.termdag.get(*k))?;
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
