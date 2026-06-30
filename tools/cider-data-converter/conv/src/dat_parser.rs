use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_while_m_n},
    character::complete::{anychar, line_ending, multispace0},
    combinator::{eof, map_res, opt},
    error::Error,
    multi::{many_till, many1},
    sequence::{preceded, tuple},
};

fn is_hex_digit(c: char) -> bool {
    c.is_ascii_hexdigit()
}

fn from_hex(input: &str) -> Result<u8, std::num::ParseIntError> {
    u8::from_str_radix(input, 16)
}

fn parse_hex(input: &str) -> IResult<&str, u8> {
    map_res(take_while_m_n(1, 2, is_hex_digit), from_hex)(input)
}

/// Parse a single line of hex characters into a vector of bytes in the order
/// the characters are given, i.e. reversed.
fn hex_line(input: &str) -> IResult<&str, LineOrComment> {
    // strip any leading whitespace
    let (input, bytes) = preceded(
        tuple((multispace0, opt(tag("0x")))),
        many1(parse_hex),
    )(input)?;

    Ok((input, LineOrComment::Line(bytes)))
}

fn comment(input: &str) -> IResult<&str, LineOrComment> {
    // skip any whitespace
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("//")(input)?;
    let (input, _) = many_till(anychar, alt((line_ending, eof)))(input)?;
    Ok((input, LineOrComment::Comment))
}
/// Parse a line which only contains whitespace
fn empty_line(input: &str) -> IResult<&str, LineOrComment> {
    // skip any whitespace
    let (input, _) = multispace0(input)?;
    Ok((input, LineOrComment::EmptyLine))
}

pub fn line_or_comment(
    input: &str,
) -> Result<LineOrComment, nom::Err<Error<&str>>> {
    let (_, res) = alt((hex_line, comment, empty_line))(input)?;
    Ok(res)
}

#[derive(Debug, PartialEq)]
pub enum LineOrComment {
    Line(Vec<u8>),
    Comment,
    EmptyLine,
}

/// Parse a single line of hex characters, or a comment. Returns None if it's a
/// comment or an empty line and Some(Vec<u8>) if it's a hex line. Panics on a
/// parse error.
///
/// For the fallible version, see `line_or_comment`.
pub fn unwrap_line_or_comment(input: &str) -> Option<Vec<u8>> {
    match line_or_comment(input).expect("hex parse failed") {
        LineOrComment::Line(vec) => Some(vec),
        LineOrComment::Comment => None,
        LineOrComment::EmptyLine => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment() {
        assert_eq!(comment("// comment"), Ok(("", LineOrComment::Comment)));
        assert_eq!(comment("// comment\n"), Ok(("", LineOrComment::Comment)));
    }

    #[test]
    fn test_hex_line() {
        assert_eq!(hex_line("0x01"), Ok(("", LineOrComment::Line(vec![1]))));
        assert_eq!(hex_line("0x02"), Ok(("", LineOrComment::Line(vec![2]))));
        assert_eq!(hex_line("0x03"), Ok(("", LineOrComment::Line(vec![3]))));
        assert_eq!(hex_line("0x04"), Ok(("", LineOrComment::Line(vec![4]))));
        assert_eq!(hex_line("0x05"), Ok(("", LineOrComment::Line(vec![5]))));
        assert_eq!(hex_line("0x06"), Ok(("", LineOrComment::Line(vec![6]))));
        assert_eq!(hex_line("0x07"), Ok(("", LineOrComment::Line(vec![7]))));
        assert_eq!(hex_line("0x08"), Ok(("", LineOrComment::Line(vec![8]))));
        assert_eq!(hex_line("0x09"), Ok(("", LineOrComment::Line(vec![9]))));
        assert_eq!(hex_line("0x0a"), Ok(("", LineOrComment::Line(vec![10]))));
        assert_eq!(hex_line("0x0b"), Ok(("", LineOrComment::Line(vec![11]))));
        assert_eq!(hex_line("0x0c"), Ok(("", LineOrComment::Line(vec![12]))));
        assert_eq!(hex_line("0x0d"), Ok(("", LineOrComment::Line(vec![13]))));
        assert_eq!(hex_line("0x0e"), Ok(("", LineOrComment::Line(vec![14]))));
        assert_eq!(hex_line("0x0f"), Ok(("", LineOrComment::Line(vec![15]))));
        assert_eq!(hex_line("0xff"), Ok(("", LineOrComment::Line(vec![255]))));
        assert_eq!(
            hex_line("0x00ff"),
            Ok(("", LineOrComment::Line(vec![0, 255])))
        );
    }

    #[test]
    fn test_from_hex() {
        assert_eq!(from_hex("0"), Ok(0));
        assert_eq!(from_hex("1"), Ok(1));
        assert_eq!(from_hex("2"), Ok(2));
        assert_eq!(from_hex("3"), Ok(3));
        assert_eq!(from_hex("4"), Ok(4));
        assert_eq!(from_hex("5"), Ok(5));
        assert_eq!(from_hex("6"), Ok(6));
        assert_eq!(from_hex("7"), Ok(7));
        assert_eq!(from_hex("8"), Ok(8));
        assert_eq!(from_hex("9"), Ok(9));
        assert_eq!(from_hex("a"), Ok(10));
        assert_eq!(from_hex("b"), Ok(11));
        assert_eq!(from_hex("c"), Ok(12));
        assert_eq!(from_hex("d"), Ok(13));
        assert_eq!(from_hex("e"), Ok(14));
        assert_eq!(from_hex("f"), Ok(15));

        assert_eq!(from_hex("FF"), Ok(255));
        assert_eq!(from_hex("ff"), Ok(255));
    }
}
