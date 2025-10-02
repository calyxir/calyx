use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

use super::{
    ast::{Token, TokenKind},
    span::Span,
};

pub type Wrap<T> = Rc<T>;

#[derive(Clone, Debug)]
pub struct FileReadError {
    msg: String,
}

impl Display for FileReadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl Error for FileReadError {}

impl FileReadError {
    pub fn from_msg(msg: &str) -> Wrap<dyn Error> {
        Wrap::new(FileReadError {
            msg: msg.to_string(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct NoMoreToLexError {}
impl Display for NoMoreToLexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "no more to lex")
    }
}

impl Error for NoMoreToLexError {}

#[derive(Clone, Debug)]
pub struct InvalidStartToIdError<'a> {
    pub context: Span<'a>,
    pub char: char,
}

impl Display for InvalidStartToIdError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "id cannot start with {}", self.char)
    }
}

impl<'a> Error for InvalidStartToIdError<'a> {}

#[derive(Clone, Debug)]
pub struct UnexpectedTokenError<'a> {
    pub found_token: Token<'a>,
    pub expected_token_kind: TokenKind,
}

impl Display for UnexpectedTokenError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "expected {} but found {}",
            self.expected_token_kind, self.found_token.kind
        )
    }
}

impl Error for UnexpectedTokenError<'_> {}

#[derive(Clone, Debug)]
pub(super) struct UnexpectedCharError {
    pub found_char: Option<char>,
    pub expected_char: Vec<char>,
}

impl UnexpectedCharError {
    pub fn from_parts(
        found_char: Option<char>,
        expected_char: &[char],
    ) -> Wrap<dyn Error> {
        Wrap::new(UnexpectedCharError {
            found_char,
            expected_char: expected_char.to_vec(),
        })
    }
}

impl Display for UnexpectedCharError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let joined_chars = self
            .expected_char
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let found = if let Some(c) = self.found_char {
            c.to_string()
        } else {
            "nothing".to_string()
        };
        write!(f, "expected one of [{joined_chars}] but found {found}")
    }
}

impl Error for UnexpectedCharError {}

#[derive(Clone, Debug)]
pub struct EmptyVarListInAssignment<'a> {
    pub _span: Span<'a>,
}

impl Display for EmptyVarListInAssignment<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "no variables found to assign to")
    }
}

impl Error for EmptyVarListInAssignment<'_> {}
