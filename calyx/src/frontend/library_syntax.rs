use crate::lang::ast;
use crate::lang::library::ast as lib;
use crate::{errors, errors::Result};
use pest_consume::{match_nodes, Error, Parser};
use std::fs;
use std::path::PathBuf;

type ParseResult<T> = std::result::Result<T, Error<Rule>>;
type Node<'i> = pest_consume::Node<'i, Rule, ()>;

const _GRAMMAR: &str = include_str!("library_syntax.pest");

#[derive(Parser)]
#[grammar = "frontend/library_syntax.pest"]
pub struct LibraryParser;

impl LibraryParser {
    pub fn from_file(path: &PathBuf) -> Result<lib::Library> {
        let content = &fs::read(path).unwrap();
        let string_content = std::str::from_utf8(content).unwrap();
        let inputs = LibraryParser::parse(Rule::file, string_content)?;
        let input = inputs.single()?;
        Ok(LibraryParser::file(input)?)
    }
}

#[pest_consume::parser]
impl LibraryParser {
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn identifier(input: Node) -> ParseResult<ast::Id> {
        Ok(input.as_str().into())
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

    fn block(input: Node) -> ParseResult<String> {
        Ok(input.into_pair().as_str().to_string())
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
            [identifier(name), params(p), signature(s), implementation(i)] => lib::Primitive {
                name,
                params: p,
                signature: s,
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
