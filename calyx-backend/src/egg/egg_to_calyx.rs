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
        let mut buf = String::new();
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
                write!(f, "{}", " ".repeat(indent_level))?;
                self.emit(f, indent_level, self.termdag.get(*attributes))?;
                self.emit(f, 0, self.termdag.get(*group))?;
                writeln!(f, ";")?;
                Ok(())

            }
            ("Attributes", [mapping]) => {
                let mut mapping = self.termdag.get(*mapping);
                'outer: loop {
                    match_term_app!(mapping; {
                        ("map-insert", [map, k, v]) => {
                            write!(f,"@")?;
                            self.emit(f, 0, self.termdag.get(*k))?;
                            if let Term::Lit(Literal::Int(n)) = self.termdag.get(*v) {
                                if n > 1 {
                                    write!(f,"(")?;
                                    self.emit(f, 0, self.termdag.get(*v))?;
                                    write!(f,")")?;
                                }
                            }
                            write!(f," ")?;
                            mapping = self.termdag.get(*map);
                            continue;
                        }
                        ("map-empty", []) => {
                            break 'outer;
                        }
                        (&_, _) => todo!("unexpected: {:?}", expr)
                    });
                }
                Ok(())
            }
            ("Par", [attributes, list]) => {
                write!(f,"{}", " ".repeat(indent_level))?;
                self.emit(f, 0, self.termdag.get(*attributes))?;
                writeln!(f,"par {{", )?;
                self.emit(f, indent_level + 2, self.termdag.get(*list))?;
                writeln!(f, "{}}}", " ".repeat(indent_level))?;
                Ok(())
            }
            ("Seq", [attributes, list]) => {
                write!(f,"{}", " ".repeat(indent_level))?;
                self.emit(f, 0, self.termdag.get(*attributes))?;
                writeln!(f,"seq {{")?;
                self.emit(f, indent_level + 2, self.termdag.get(*list))?;
                writeln!(f, "{}}}", " ".repeat(indent_level))?;
                Ok(())
            }
            ("Repeat", [attributes, n, body]) => {
                write!(f,"{}", " ".repeat(indent_level))?;
                self.emit(f, 0, self.termdag.get(*attributes))?;
                write!(f, "repeat ")?;
                self.emit(f, 0, self.termdag.get(*n))?;
                writeln!(f, " {{")?;
                self.emit(f, indent_level + 2, self.termdag.get(*body))?;
                writeln!(f, "{}}}", " ".repeat(indent_level))?;
                Ok(())
            }
            ("Cons", [x, xs]) => {
                self.emit(f,indent_level, self.termdag.get(*x))?;
                self.emit(f,indent_level, self.termdag.get(*xs))
            }
            ("Nil", []) => {
                Ok(())
            }
            (&_, _) => todo!("unexpected: {:?}", expr)
        })
    }
}
