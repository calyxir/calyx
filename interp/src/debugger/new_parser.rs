use std::collections::HashMap;

use pest_consume::{match_nodes, Error, Parser};

use super::structures::{NamedTag, SourceMap};
use crate::errors::InterpreterResult;

type ParseResult<T> = std::result::Result<T, Error<Rule>>;
type Node<'i> = pest_consume::Node<'i, Rule, ()>;

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("metadata.pest");

#[derive(Parser)]
#[grammar = "debugger/source/metadata.pest"]
pub struct MetadataParser;

#[pest_consume::parser]
impl MetadataParser {
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn num(input: Node) -> ParseResult<u64> {
        input
            .as_str()
            .parse::<u64>()
            .map_err(|_| input.error("Expected non-negative number"))
    }
    fn group_name(input: Node) -> ParseResult<String> {
        input
            .as_str()
            .parse::<String>()
            .map_err(|_| input.error("Expected character"))
    }
    fn escaped_newline(_input: Node) -> ParseResult<char> {
        Ok('\n')
    }
    fn entry(input: Node) -> ParseResult<(String, i64)> {
        Ok(match_nodes!(input.into_children();
            [group_name(g), num(n)] => (g, n)
        ))
    }
    // Do we need to do all fields????????? Like Header
    fn metadata(input: Node) -> ParseResult<NewSourceMap> {
        Ok(match_nodes!(input.into_children();
            [entry(e).., EOI(_)] => {
                let map: HashMap<String, i64> = e.collect();
                map.into()
            }
        ))
    }
}
pub fn parse_metadata(input_str: &str) -> InterpreterResult<NewSourceMap> {
    let inputs = MetadataParser::parse(Rule::metadata, input_str)?;
    let input = inputs.single()?;
    Ok(MetadataParser::metadata(input)?)
}
