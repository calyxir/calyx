use super::core::ParsePath;

use pest_consume::{match_nodes, Error, Parser};

type ParseResult<T> = std::result::Result<T, Error<Rule>>;
type Node<'i> = pest_consume::Node<'i, Rule, ()>;

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("path_parser.pest");

#[derive(Parser)]
#[grammar = "debugger/commands/path_parser.pest"]

pub struct PathParser;

#[pest_consume::parser]
impl PathParser {
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn root(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn body(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn num(input: Node) -> ParseResult<u32> {
        input
            .as_str()
            .parse::<u32>()
            .map_err(|_| input.error("Expected non-negative number"))
    }

    fn clause(input: Node) -> ParseResult<ParsePath> {
        Ok(match_nodes!(input.into_children();
            [num(n)] => ParsePath::Offset(n),
            [body(_)] => ParsePath::Body,
            [] => ParsePath::Separator,
        ))
    }

    fn path(input: Node) -> ParseResult<ParsePath> {
        Ok(match_nodes!(input.into_children();
            [root(_), EOI(_)] => ParsePath::Root,
            [clause(c).., EOI(_)] => c,
            [EOI(_)] => ParsePath::End,
        ))
    }
}

// Parse the path
pub fn parse_path(input_str: &str) -> Result<Vec<ParsePath>, Error<Rule>> {
    let mut path_vec = vec![];
    let entries = PathParser::parse(Rule::path, input_str)?;
    let entry = entries.single()?;

    path_vec.extend(PathParser::path(entry));

    Ok(path_vec)
}
