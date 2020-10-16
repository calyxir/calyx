//! Parser for FuTIL libraries.
use super::ast as lib;
use crate::errors::{self, FutilResult, Span};
use crate::ir::{self, Direction};
use pest_consume::{match_nodes, Error, Parser};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

type ParseResult<T> = Result<T, Error<Rule>>;
type Node<'i> = pest_consume::Node<'i, Rule, Rc<String>>;

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("syntax.pest");

#[derive(Parser)]
#[grammar = "frontend/library/syntax.pest"]
pub struct LibraryParser;

impl LibraryParser {
    /// Parses a FuTIL library into an AST representation.
    pub fn parse_file(path: &PathBuf) -> FutilResult<lib::Library> {
        let content = &fs::read(path).map_err(|err| {
            errors::Error::InvalidFile(format!(
                "Failed to read {}: {}",
                path.to_string_lossy(),
                err.to_string()
            ))
        })?;
        let string_content = std::str::from_utf8(content)?;
        let inputs = LibraryParser::parse_with_userdata(
            Rule::file,
            string_content,
            Rc::new(string_content.to_string()),
        )?;
        let input = inputs.single()?;
        Ok(LibraryParser::file(input)?)
    }
}

#[pest_consume::parser]
impl LibraryParser {
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn char(input: Node) -> ParseResult<&str> {
        Ok(input.as_str())
    }

    fn string_lit(input: Node) -> ParseResult<String> {
        Ok(match_nodes!(
            input.into_children();
            [char(c)..] => c.collect::<Vec<_>>().join("")
        ))
    }

    fn identifier(input: Node) -> ParseResult<ir::Id> {
        Ok(ir::Id::new(
            input.as_str(),
            Some(Span::new(input.as_span(), Rc::clone(input.user_data()))),
        ))
    }

    fn bitwidth(input: Node) -> ParseResult<u64> {
        Ok(input.as_str().parse::<u64>().unwrap())
    }

    fn io_port(input: Node) -> ParseResult<lib::ParamPortdef> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(id), bitwidth(bw)] => lib::ParamPortdef {
                name: id,
                width: lib::Width::Const { value: bw },
                direction: Direction::Input,
            },
            [identifier(id), identifier(param)] => lib::ParamPortdef {
                name: id,
                width: lib::Width::Param { value: param },
                direction: Direction::Output,
            }
        ))
    }

    fn io_ports(input: Node) -> ParseResult<Vec<lib::ParamPortdef>> {
        Ok(match_nodes!(
            input.into_children();
            [io_port(p)..] => p.collect()))
    }

    fn params(input: Node) -> ParseResult<Vec<ir::Id>> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(id)..] => id.collect()
        ))
    }

    fn key_value(input: Node) -> ParseResult<(String, u64)> {
        Ok(match_nodes!(
            input.into_children();
            [string_lit(key), bitwidth(num)] => (key, num)
        ))
    }

    fn attributes(input: Node) -> ParseResult<HashMap<String, u64>> {
        Ok(match_nodes!(
            input.into_children();
            [key_value(kvs)..] => kvs.collect()
        ))
    }

    fn signature(input: Node) -> ParseResult<Vec<lib::ParamPortdef>> {
        Ok(match_nodes!(
            input.into_children();
            [io_ports(ins), io_ports(outs)] => {
                ins.into_iter().chain(outs.into_iter()).collect()
            },
            [io_ports(ins)] => {
                ins
            }
        ))
    }

    fn inner_wrap(input: Node) -> ParseResult<String> {
        // remove extra whitespace and indentation
        let mut FutilResult = String::new();
        // records the base indentation level
        let mut indent_level: Option<usize> = None;
        for line in input.as_str().lines() {
            // find the first non-empty line
            if !line.is_empty() && indent_level.is_none() {
                indent_level = line.find(|s| !char::is_whitespace(s));
            }

            // if we have already found indent level
            if indent_level.is_some() {
                FutilResult += indent_level
                    .map(|pre| {
                        if line.len() > pre {
                            line.split_at(pre).1
                        } else {
                            line
                        }
                    })
                    .unwrap_or(line)
                    .trim_end();
                FutilResult += "\n";
            }
        }
        Ok(FutilResult.trim_end().to_string())
    }

    fn block(input: Node) -> ParseResult<String> {
        Ok(match_nodes!(
            input.into_children();
            [inner_wrap(text)] => text
        ))
    }

    fn verilog_block(input: Node) -> ParseResult<lib::Verilog> {
        Ok(match_nodes!(
            input.into_children();
            [block(code)] => lib::Verilog { code }
        ))
    }

    fn implementation(input: Node) -> ParseResult<Vec<lib::Implementation>> {
        input
            .into_children()
            .map(|node| {
                Ok(match node.as_rule() {
                    Rule::verilog_block => lib::Implementation::Verilog {
                        data: Self::verilog_block(node)?,
                    },
                    _ => unreachable!(),
                })
            })
            .collect()
    }

    fn primitive(input: Node) -> ParseResult<lib::Primitive> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), attributes(attrs), params(p), signature(s), implementation(i)] => lib::Primitive {
                name,
                params: p,
                signature: s,
                attributes: attrs,
                implementation: i
            },
            [identifier(name), attributes(attrs), signature(s), implementation(i)] => lib::Primitive {
                name,
                params: vec![],
                signature: s,
                attributes: attrs,
                implementation: i
            },
            [identifier(name), params(p), signature(s), implementation(i)] => lib::Primitive {
                name,
                params: p,
                signature: s,
                attributes: HashMap::new(),
                implementation: i
            },
            [identifier(name), signature(s), implementation(i)] => lib::Primitive {
                name,
                params: vec![],
                signature: s,
                attributes: HashMap::new(),
                implementation: i
            }
        ))
    }

    fn file(input: Node) -> ParseResult<lib::Library> {
        Ok(match_nodes!(
            input.into_children();
            [primitive(p).., _] => lib::Library {
                primitives: p.collect()
            }
        ))
    }
}
