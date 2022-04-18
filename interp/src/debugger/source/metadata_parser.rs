use std::collections::HashMap;

use pest_consume::{match_nodes, Error, Parser};

use crate::errors::InterpreterResult;

use super::structures::{NamedTag, SourceMap};

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

    fn id_string(input: Node) -> ParseResult<String> {
        Ok(input.as_str().to_string())
    }

    fn named_tag(input: Node) -> ParseResult<NamedTag> {
        Ok(match_nodes!(input.into_children();
            [num(n), id_string(s)] => (n,s).into()
        ))
    }

    fn tag(input: Node) -> ParseResult<NamedTag> {
        Ok(match_nodes!(input.into_children();
            [num(n)] => NamedTag::new_nameless(n),
            [named_tag(t)] => t,
        ))
    }

    fn escaped_newline(_input: Node) -> ParseResult<char> {
        Ok('\n')
    }

    fn string_char(input: Node) -> ParseResult<String> {
        Ok(input.as_str().to_string())
    }
    fn source_char(input: Node) -> ParseResult<String> {
        Ok(match_nodes!(input.into_children();
                [escaped_newline(e)] => e.to_string(),
                [string_char(s)] => s
        ))
    }

    fn inner_position_string(input: Node) -> ParseResult<String> {
        Ok(match_nodes!(input.into_children();
            [source_char(sc)..] => sc.collect()
        ))
    }

    fn position_string(input: Node) -> ParseResult<String> {
        Ok(match_nodes!(input.into_children();
            [inner_position_string(i)] => i
        ))
    }

    fn entry(input: Node) -> ParseResult<(NamedTag, String)> {
        Ok(match_nodes!(input.into_children();
            [tag(t), position_string(s)] => (t, s)
        ))
    }

    fn metadata(input: Node) -> ParseResult<SourceMap> {
        Ok(match_nodes!(input.into_children();
            [entry(e).., EOI(_)] => {
                let map: HashMap<NamedTag, String> = e.collect();
                map.into()
            }
        ))
    }
}

pub fn parse_metadata(input_str: &str) -> InterpreterResult<SourceMap> {
    let inputs = MetadataParser::parse(Rule::metadata, input_str)?;
    let input = inputs.single()?;
    Ok(MetadataParser::metadata(input)?)
}
