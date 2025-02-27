use super::{core::ParseNodes, ParsePath};

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

    fn separator(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn name(input: Node) -> ParseResult<String> {
        Ok(input.as_str().to_owned())
    }

    fn num(input: Node) -> ParseResult<u32> {
        input
            .as_str()
            .parse::<u32>()
            .map_err(|_| input.error("Expected non-negative number"))
    }

    fn branch(input: Node) -> ParseResult<bool> {
        let b = input.as_str();
        let result = b != "f";
        Ok(result)
    }

    fn clause(input: Node) -> ParseResult<ParseNodes> {
        Ok(match_nodes!(input.into_children();
            [separator(_), num(n)] => ParseNodes::Offset(n),
            [separator(_), body(_)] => ParseNodes::Body,
            [separator(_), branch(b)] => ParseNodes::If(b)
        ))
    }

    fn path(input: Node) -> ParseResult<ParsePath> {
        Ok(match_nodes!(input.into_children();
            [name(n), root(_), clause(c).., EOI(_)] => ParsePath::from_iter(c,n),
        ))
    }
}

// Parse the path
#[allow(dead_code)]
pub fn parse_path(input_str: &str) -> Result<ParsePath, Box<Error<Rule>>> {
    let entries = PathParser::parse(Rule::path, input_str)?;
    let entry = entries.single()?;

    PathParser::path(entry).map_err(Box::new)
}

#[cfg(test)]
#[test]
fn root() {
    let path = parse_path("32: .").unwrap();
    dbg!(path.get_path());
    assert_eq!(path.get_path(), Vec::new());
    assert_eq!(path.get_name(), "32");
}

#[test]
fn body() {
    let path = parse_path("0: .-b").unwrap();
    dbg!(path.get_path());
    assert_eq!(path.get_path(), vec![ParseNodes::Body]);
    assert_eq!(path.get_name(), "0");
}

#[test]
fn branch() {
    let path = parse_path("0: .-f").unwrap();
    dbg!(path.get_path());
    assert_eq!(path.get_path(), vec![ParseNodes::If(false)]);
    assert_eq!(path.get_name(), "0");
}

#[test]
fn offset() {
    let path = parse_path("0: .-0-1").unwrap();
    dbg!(path.get_path());
    assert_eq!(
        path.get_path(),
        vec![ParseNodes::Offset(0), ParseNodes::Offset(1)]
    );
    assert_eq!(path.get_name(), "0");
}

#[test]
fn multiple() {
    let path = parse_path("heLl.o123: .-0-1-b-t").unwrap();
    dbg!(path.get_path());
    assert_eq!(
        path.get_path(),
        vec![
            ParseNodes::Offset(0),
            ParseNodes::Offset(1),
            ParseNodes::Body,
            ParseNodes::If(true)
        ]
    );
    assert_eq!(path.get_name(), "heLl.o123");
}
