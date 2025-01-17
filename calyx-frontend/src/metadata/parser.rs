use std::path::PathBuf;

use pest_consume::{match_nodes, Error, Parser};

use super::table::{FileId, LineNum, MetadataTable, PositionId};

type ParseResult<T> = Result<T, Error<Rule>>;
type Node<'i> = pest_consume::Node<'i, Rule, ()>;

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("metadata.pest");

#[derive(Parser)]
#[grammar = "metadata/metadata.pest"]
pub struct MetadataParser;

#[pest_consume::parser]
impl MetadataParser {
    fn num(input: Node) -> ParseResult<u32> {
        input
            .as_str()
            .parse::<u32>()
            .map_err(|_| input.error("Identifying numbers must be u32"))
    }

    fn quote(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn path_text(input: Node) -> ParseResult<PathBuf> {
        Ok(PathBuf::from(input.as_str()))
    }

    fn path(input: Node) -> ParseResult<PathBuf> {
        Ok(match_nodes!(input.into_children();
                [quote(_), path_text(p), quote(_)] => p
        ))
    }

    fn file_entry(input: Node) -> ParseResult<(FileId, PathBuf)> {
        Ok(match_nodes!(input.into_children();
            [num(n), path(p)] => (FileId::new(n), p)
        ))
    }

    fn file_header(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn file_table(input: Node) -> ParseResult<Vec<(FileId, PathBuf)>> {
        Ok(match_nodes!(input.into_children();
            [file_header(_), file_entry(e)..] => e.collect()))
    }

    fn position_header(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn position_entry(
        input: Node,
    ) -> ParseResult<(PositionId, FileId, LineNum)> {
        Ok(match_nodes!(input.into_children();
            [num(pos_num), num(file_num), num(line_no)] => (PositionId::new(pos_num), FileId::new(file_num), LineNum::new(line_no))
        ))
    }

    fn position_table(
        input: Node,
    ) -> ParseResult<Vec<(PositionId, FileId, LineNum)>> {
        Ok(match_nodes!(input.into_children();
                [position_header(_), position_entry(e)..] => e.collect()))
    }

    fn metadata_table(input: Node) -> ParseResult<MetadataTable> {
        dbg!("hi");
        Ok(match_nodes!(input.into_children();
            [file_table(f), position_table(p)] => MetadataTable::new(f, p)
        ))
    }
}

pub fn parse_metadata(
    input_str: &str,
) -> Result<MetadataTable, Box<Error<Rule>>> {
    let inputs = MetadataParser::parse(Rule::metadata_table, input_str)?;
    let input = inputs.single()?;
    Ok(MetadataParser::metadata_table(input)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metadata() {
        let input_str = r#"
FILES
    0: "test.calyx"
    1: "test2.calyx"
    2: "test3.calyx"
POSITIONS
    0: 0 0
    1: 0 1
    2: 0 2
            "#;

        let metadata = parse_metadata(input_str).unwrap();
        dbg!(metadata);
    }
}
