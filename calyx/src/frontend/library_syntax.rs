use crate::errors::{self, Result, Span};
use crate::lang::ast;
use crate::lang::library::ast as lib;
use pest_consume::{match_nodes, Error, Parser};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

type ParseResult<T> = std::result::Result<T, Error<Rule>>;
type Node<'i> = pest_consume::Node<'i, Rule, Rc<String>>;

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("library_syntax.pest");

#[derive(Parser)]
#[grammar = "frontend/library_syntax.pest"]
pub struct LibraryParser;

impl LibraryParser {
    pub fn parse_file(path: &PathBuf) -> Result<lib::Library> {
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

    fn identifier(input: Node) -> ParseResult<ast::Id> {
        Ok(ast::Id::new(
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
                width: lib::Width::Const { value: bw }
            },
            [identifier(id), identifier(param)] => lib::ParamPortdef {
                name: id,
                width: lib::Width::Param { value: param }
            }
        ))
    }

    fn io_ports(input: Node) -> ParseResult<Vec<lib::ParamPortdef>> {
        Ok(match_nodes!(
            input.into_children();
            [io_port(p)..] => p.collect()))
    }

    fn params(input: Node) -> ParseResult<Vec<ast::Id>> {
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

    fn signature(input: Node) -> ParseResult<lib::ParamSignature> {
        Ok(match_nodes!(
            input.into_children();
            [io_ports(ins), io_ports(outs)] => lib::ParamSignature {
                inputs: ins,
                outputs: outs
            },
            [io_ports(ins)] => lib::ParamSignature {
                inputs: ins,
                outputs: vec![]
            }
        ))
    }

    fn inner_wrap(input: Node) -> ParseResult<String> {
        // remove extra whitespace and indentation
        let mut result = String::new();
        // records the base indentation level
        let mut indent_level: Option<usize> = None;
        for line in input.as_str().lines() {
            // find the first non-empty line
            if !line.is_empty() && indent_level.is_none() {
                indent_level = line.find(|s| !char::is_whitespace(s));
            }

            // if we have already found indent level
            if indent_level.is_some() {
                result += indent_level
                    .map(|pre| {
                        if line.len() > pre {
                            line.split_at(pre).1
                        } else {
                            line
                        }
                    })
                    .unwrap_or(line)
                    .trim_end();
                result += "\n";
            }
        }
        Ok(result.trim_end().to_string())
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
