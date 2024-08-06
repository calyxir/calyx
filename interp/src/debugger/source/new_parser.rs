use super::structures::{GroupContents, NewSourceMap};
use crate::errors::InterpreterResult;
use pest_consume::{match_nodes, Error, Parser};
use std::collections::HashMap;
type ParseResult<T> = std::result::Result<T, Error<Rule>>;
type Node<'i> = pest_consume::Node<'i, Rule, ()>;

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("new_metadata.pest");

#[derive(Parser)]
#[grammar = "debugger/source/new_metadata.pest"]
pub struct MetadataParser;

#[pest_consume::parser]
impl MetadataParser {
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }
    pub fn num(input: Node) -> ParseResult<u64> {
        input
            .as_str()
            .parse::<u64>()
            .map_err(|_| input.error("Expected number"))
    }
    fn group_name(input: Node) -> ParseResult<String> {
        Ok(input.as_str().to_string())
    }

    fn path(input: Node) -> ParseResult<String> {
        Ok(input.as_str().to_string())
    }

    fn entry(input: Node) -> ParseResult<((String, String), GroupContents)> {
        Ok(match_nodes!(input.into_children();
            [group_name(comp), group_name(grp), path(p),num(st), num(end)] => ((comp, grp),GroupContents {path:p, start_line: st, end_line:end})
        ))
    }

    fn metadata(input: Node) -> ParseResult<NewSourceMap> {
        Ok(match_nodes!(input.into_children();
            [entry(e).., EOI(_)] => {
                let map: HashMap<(String, String), GroupContents> = e.collect();
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

// Meta is expected as the following format, this is an example for reg_seq.futil

// metadata #{
//    wr_reg0: /path/to/file 10
//    wr_reg1: /path/to/file 15
//   }#

#[cfg(test)]
#[test]
fn one_entry() {
    let entry = parse_metadata("hello.from: your/mom 6-9").unwrap();
    dbg!(&entry);
    let tup = entry.lookup(&(String::from("hello"), String::from("from")));
    assert_eq!(
        tup.unwrap().clone(),
        GroupContents {
            path: String::from("your/mom"),
            start_line: 6,
            end_line: 9
        }
    )
}

#[test]
fn mult_entries() {
    let entry = parse_metadata(
        "mom.wr_reg0: calyx/interp/test/control/reg_seq.futil 10-13,
        dad.wr_reg1: calyx/interp/test/control/reg_seq.futil 15-19",
    )
    .unwrap();
    let tup = entry.lookup(&(String::from("mom"), String::from("wr_reg0")));
    let tup2 = entry.lookup(&(String::from("dad"), String::from("wr_reg1")));
    assert_eq!(
        tup.unwrap().clone(),
        GroupContents {
            path: String::from("calyx/interp/test/control/reg_seq.futil"),
            start_line: 10,
            end_line: 13
        }
    );
    assert_eq!(
        tup2.unwrap().clone(),
        GroupContents {
            path: String::from("calyx/interp/test/control/reg_seq.futil"),
            start_line: 15,
            end_line: 19
        }
    )
}
