use std::fmt::Debug;
use std::io;
use std::rc::Rc;

use super::{
    ast::{Token, TokenKind},
    span::Span,
};

/// TODO: use std::error::Error
pub trait Error {
    /// Returns the error message corresponding to the error.
    fn msg(&self) -> String;
}

impl Error for serde_json::Error {
    fn msg(&self) -> String {
        self.to_string()
    }
}

impl Error for io::Error {
    fn msg(&self) -> String {
        self.to_string()
    }
}

impl Debug for dyn Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg())
    }
}

pub type Wrap<T> = Rc<T>;
pub type LexError = Wrap<dyn Error>;
pub type ParseError<'a> = Wrap<dyn Error + 'a>;

#[derive(Clone, Debug)]
pub struct FileReadError {
    msg: String,
}

impl<'a> FileReadError {
    pub fn from_msg(msg: &str) -> ParseError<'a> {
        Wrap::new(FileReadError {
            msg: msg.to_string(),
        })
    }
}

impl Error for FileReadError {
    fn msg(&self) -> String {
        self.msg.clone()
    }
}

#[derive(Clone, Debug)]
pub struct NoMoreToLexError {}
impl Error for NoMoreToLexError {
    fn msg(&self) -> String {
        "no more to lex".to_string()
    }
}

#[derive(Clone, Debug)]
pub struct InvalidStartToId<'a> {
    pub context: Span<'a>,
    pub char: char,
}

impl Error for InvalidStartToId<'_> {
    fn msg(&self) -> String {
        format!("found {}, invalid start to identifier", self.char)
    }
}

#[derive(Clone, Debug)]
pub struct UnexpectedToken<'a> {
    pub found_token: Token<'a>,
    pub expected_token_kind: TokenKind,
}
impl Error for UnexpectedToken<'_> {
    fn msg(&self) -> String {
        format!(
            "expected {:?} but found {:?}",
            self.expected_token_kind, self.found_token.kind
        )
    }
}

#[derive(Clone, Debug)]
pub(super) struct UnexpectedChar {
    pub found_char: Option<char>,
    pub expected_char: Vec<char>,
}

impl<'a> UnexpectedChar {
    pub fn from_parts(
        found_char: Option<char>,
        expected_char: &[char],
    ) -> ParseError<'a> {
        Wrap::new(UnexpectedChar {
            found_char,
            expected_char: expected_char.to_vec(),
        })
    }
}

impl Error for UnexpectedChar {
    fn msg(&self) -> String {
        format!(
            "expected {:?} but found {:?}",
            self.expected_char, self.found_char,
        )
    }
}

#[derive(Clone, Debug)]
pub struct EmptyVarListInAssignment<'a> {
    pub _span: Span<'a>,
}
impl Error for EmptyVarListInAssignment<'_> {
    fn msg(&self) -> String {
        "no variables found to assign to".to_string()
    }
}
