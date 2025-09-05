//! The recursive decent parser for parsing plan files.

use camino::Utf8PathBuf;
use std::fs;

use super::{ast::*, span::Span};

pub trait Error {
    /// Returns the error message corresponding to the error.
    fn msg(&self) -> String;

    fn copy(&self) -> Box<dyn Error>;
}

#[derive(Clone, Debug)]
pub struct FileReadError {
    msg: String,
}

impl Error for FileReadError {
    fn msg(&self) -> String {
        self.msg.clone()
    }

    fn copy(&self) -> Box<dyn Error> {
        Box::new(self.clone())
    }
}

#[derive(Clone, Debug)]
pub struct NoMoreToLexError {}
impl Error for NoMoreToLexError {
    fn msg(&self) -> String {
        "No more characters in input file.".to_string()
    }

    fn copy(&self) -> Box<dyn Error> {
        Box::new(self.clone())
    }
}

#[derive(Clone, Debug)]
pub struct InvalidIdentifierError {}
impl Error for InvalidIdentifierError {
    fn msg(&self) -> String {
        "Syntax Error: invalid identifier".to_string()
    }

    fn copy(&self) -> Box<dyn Error> {
        Box::new(self.clone())
    }
}

#[derive(Clone, Debug)]
pub struct NoParseStarted {}
impl Error for NoParseStarted {
    fn msg(&self) -> String {
        "Error: no parse started".to_string()
    }

    fn copy(&self) -> Box<dyn Error> {
        Box::new(self.clone())
    }
}

#[derive(Clone, Debug)]
pub struct UnexpectedToken {
    found_token: Token,
    expected_token_kind: TokenKind,
}
impl Error for UnexpectedToken {
    fn msg(&self) -> String {
        format!(
            "expected {:?} but found {:?}",
            self.expected_token_kind, self.found_token.kind
        )
    }

    fn copy(&self) -> Box<dyn Error> {
        Box::new(self.clone())
    }
}

#[derive(Debug)]
struct Lexer {
    buf: Vec<char>,
    file_path: Option<Utf8PathBuf>,
    cursor: usize,
}

pub type ParseError = Box<dyn Error>;

impl Lexer {
    fn from_str(src: &str) -> Lexer {
        Lexer {
            buf: src.chars().collect(),
            file_path: None,
            cursor: 0,
        }
    }
    fn from_path(file_path: &Utf8PathBuf) -> Result<Lexer, ParseError> {
        if !file_path.is_file() {
            let msg = format!("Error: {file_path} is not a file");
            return Err(Box::new(FileReadError { msg }));
        };

        let path = file_path.as_path();
        match fs::read_to_string(path) {
            Ok(s) => {
                let buf = s.chars().collect();
                Ok(Lexer {
                    buf,
                    cursor: 0,
                    file_path: Some(file_path.clone()),
                })
            }
            Err(msg) => {
                let msg = format!("Error: {msg}");
                Err(Box::new(FileReadError { msg }))
            }
        }
    }

    fn has_next_token(&mut self) -> bool {
        if self.cursor >= self.buf.len() {
            return false;
        }

        while self.buf[self.cursor].is_whitespace() {
            self.cursor += 1;
            if self.cursor >= self.buf.len() {
                return false;
            }
        }

        true
    }

    fn next_token(&mut self) -> Result<Token, ParseError> {
        if self.cursor >= self.buf.len() {
            return Err(Box::new(NoMoreToLexError {}));
        }

        // This language is whitespace agnostic because semantic whitespace is weirdly hard to
        // implement while also allowing unicode.
        while self.buf[self.cursor].is_whitespace() {
            self.cursor += 1;
            if self.cursor >= self.buf.len() {
                return Err(Box::new(NoMoreToLexError {}));
            }
        }

        // Lex characters which have some special meaning.
        let span = Span {
            file_path: self.file_path.clone(),
            lo: self.cursor,
            hi: self.cursor,
        };
        let maybe_tok = match self.buf[self.cursor] {
            '=' => Some(Token {
                kind: TokenKind::Assign,
                span,
            }),
            '(' => Some(Token {
                kind: TokenKind::OpenParen,
                span,
            }),
            ')' => Some(Token {
                kind: TokenKind::CloseParen,
                span,
            }),
            ';' => Some(Token {
                kind: TokenKind::Semicolon,
                span,
            }),
            ',' => Some(Token {
                kind: TokenKind::Comma,
                span,
            }),
            _ => None,
        };

        if let Some(tok) = maybe_tok {
            self.cursor += 1;
            return Ok(tok);
        }

        // Lex identifiers.
        //
        // Identifiers are defined as some alphabetic character or "_" followed by a string of
        // alphanumeric or "_" characters.
        let first_char = self.buf[self.cursor];
        if !first_char.is_alphabetic() && first_char != '_' {
            // Just try to keep going by advancing until whitespace as some semblance of a recovery
            // procedure.
            while self.cursor < self.buf.len()
                && !self.buf[self.cursor].is_whitespace()
            {
                self.cursor += 1;
            }
            return Err(Box::new(InvalidIdentifierError {}));
        }

        let id_string: String = self
            .buf
            .split_at(self.cursor)
            .1
            .iter()
            .take_while(|&&c| c.is_alphanumeric() || c == '_')
            .collect();
        let lo = self.cursor;
        let len_in_chars = id_string.chars().count();
        let hi = lo + len_in_chars - 1;
        self.cursor += len_in_chars;
        let span = Span {
            file_path: self.file_path.clone(),
            lo,
            hi,
        };
        Ok(Token {
            kind: TokenKind::Id(id_string),
            span,
        })
    }
}

type TokenStream = Vec<Token>;

struct Parser {
    buf: Vec<Token>,
    cursor: usize,
    cache: Option<Result<AssignmentList, ParseError>>,
}

/// TODO: make this a state machine so I can implement upon parse errors.
impl Parser {
    fn from_token_stream(buf: TokenStream) -> Self {
        Parser {
            buf,
            cursor: 0,
            cache: None,
        }
    }

    fn parse(&mut self) -> Result<AssignmentList, ParseError> {
        if self.cache.is_none() {
            self.cache = Some(self.parse_assignment_list());
        }
        match self.cache.as_ref().unwrap() {
            Err(e) => Err(e.copy()),
            Ok(ast) => Ok(ast.clone()),
        }
    }

    fn parse_assignment_list(&mut self) -> Result<AssignmentList, ParseError> {
        let mut assigns = vec![];
        while self.cursor < self.buf.len() {
            let assign = self.parse_assignment()?;
            assigns.push(assign);
        }
        Ok(AssignmentList { assigns })
    }

    fn parse_id(&mut self) -> Result<Id, ParseError> {
        if self.cursor >= self.buf.len() {
            return Err(Box::new(NoMoreToLexError {}));
        }

        let cursor = self.cursor;
        self.cursor += 1;
        match &self.buf[cursor] {
            Token {
                kind: TokenKind::Id(id),
                ..
            } => Ok(id.clone()),
            t => Err(Box::new(UnexpectedToken {
                found_token: t.clone(),
                expected_token_kind: TokenKind::Id("<id>".to_string()),
            })),
        }
    }

    fn parse_simple_token_kind(
        &mut self,
        tok: TokenKind,
    ) -> Result<(), ParseError> {
        if self.cursor >= self.buf.len() {
            return Err(Box::new(NoMoreToLexError {}));
        }

        let cur_tok = self.buf[self.cursor].clone();
        self.cursor += 1;

        if matches!(tok, TokenKind::Id(_)) {
            panic!("Invalid input tok, must not be TokenKind::Id(_)");
        }

        if std::mem::discriminant(&tok) == std::mem::discriminant(&cur_tok.kind)
        {
            Ok(())
        } else {
            Err(Box::new(UnexpectedToken {
                found_token: cur_tok,
                expected_token_kind: tok,
            }))
        }
    }

    fn parse_id_list(&mut self) -> Result<Vec<Id>, ParseError> {
        if !matches!(self.buf[self.cursor].kind, TokenKind::Id(_)) {
            return Ok(vec![]);
        }

        let mut res = vec![];
        let var = self.parse_id()?;
        res.push(var);
        while matches!(self.buf[self.cursor].kind, TokenKind::Comma) {
            self.parse_simple_token_kind(TokenKind::Comma)?;
            let var = self.parse_id()?;
            res.push(var);
        }
        Ok(res)
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        let name = self.parse_id()?;
        self.parse_simple_token_kind(TokenKind::OpenParen)?;
        let args = self.parse_id_list()?;
        self.parse_simple_token_kind(TokenKind::CloseParen)?;
        Ok(Function { name, args })
    }

    fn parse_assignment(&mut self) -> Result<Assignment, ParseError> {
        let vars = self.parse_id_list()?;
        self.parse_simple_token_kind(TokenKind::Assign)?;
        let value = self.parse_function()?;
        self.parse_simple_token_kind(TokenKind::Semicolon)?;
        Ok(Assignment { vars, value })
    }
}

fn parse_from_lexer(lexer: &mut Lexer) -> Result<AssignmentList, ParseError> {
    let mut toks = vec![];
    while lexer.has_next_token() {
        let tok = lexer.next_token()?;
        toks.push(tok);
    }
    let mut parser = Parser::from_token_stream(toks);
    parser.parse()
}

pub fn parse_from_str(src: &str) -> Result<AssignmentList, ParseError> {
    let mut lexer = Lexer::from_str(src);
    parse_from_lexer(&mut lexer)
}

pub fn parse_from_path(
    path: &Utf8PathBuf,
) -> Result<AssignmentList, ParseError> {
    let mut lexer = Lexer::from_path(path)?;
    parse_from_lexer(&mut lexer)
}
