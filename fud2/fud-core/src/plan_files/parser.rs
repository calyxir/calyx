//! The recursive decent parser for parsing plan files.

use super::{ast::*, error::*, session::ParseSession, span::Span};

#[derive(Debug)]
pub(super) struct Lexer<'a> {
    sess: &'a ParseSession,
    cursor: usize,
}

impl<'a> Lexer<'a> {
    pub fn from_parts(sess: &'a ParseSession, cursor: usize) -> Self {
        Lexer { sess, cursor }
    }

    pub fn has_next_token(&mut self) -> bool {
        let buf = self.sess.buf();
        if self.cursor >= buf.len() {
            return false;
        }

        while buf[self.cursor].is_whitespace() {
            self.cursor += 1;
            if self.cursor >= buf.len() {
                return false;
            }
        }

        true
    }

    pub fn next_token(&mut self) -> Result<Token<'a>, ParseError<'a>> {
        let buf = self.sess.buf();
        if self.cursor >= buf.len() {
            return Err(UnexpectedChar::from_parts(None, 'a'));
        }

        // This language is whitespace agnostic because semantic whitespace is weirdly hard to
        // implement while also allowing unicode.
        while buf[self.cursor].is_whitespace() {
            self.cursor += 1;
            if self.cursor >= buf.len() {
                return Err(UnexpectedChar::from_parts(None, 'a'));
            }
        }

        // Lex characters which have some special meaning.
        let span = Span {
            sess: self.sess,
            lo: self.cursor,
            hi: self.cursor,
        };
        let maybe_tok = match buf[self.cursor] {
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
        let first_char = buf[self.cursor];
        if !first_char.is_alphabetic() && first_char != '_' {
            let error = InvalidStartToId {
                context: Span {
                    hi: self.cursor,
                    lo: self.cursor,
                    sess: self.sess,
                },
                char: first_char,
            };
            // Just try to keep going by advancing until whitespace as some semblance of a recovery
            // procedure.
            while self.cursor < buf.len() && !buf[self.cursor].is_whitespace() {
                self.cursor += 1;
            }
            return Err(Wrap::new(error));
        }

        let id_string: String = buf
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
            sess: self.sess,
            lo,
            hi,
        };
        Ok(Token {
            kind: TokenKind::Id(id_string),
            span,
        })
    }
}

type TokenStream<'a> = Vec<Token<'a>>;

pub struct Parser<'a> {
    buf: Vec<Token<'a>>,
    cursor: usize,
    cache: Option<Result<AssignmentList, ParseError<'a>>>,
}

/// TODO: make this a state machine for better parse errors.
impl<'a> Parser<'a> {
    pub fn from_token_stream(buf: TokenStream<'a>) -> Self {
        Parser {
            buf,
            cursor: 0,
            cache: None,
        }
    }

    pub fn parse(&mut self) -> Result<AssignmentList, ParseError<'a>> {
        if self.cache.is_none() {
            self.cache = Some(self.parse_assignment_list());
        }
        match self.cache.clone().unwrap() {
            Err(e) => Err(e),
            Ok(ast) => Ok(ast),
        }
    }

    fn parse_assignment_list(
        &mut self,
    ) -> Result<AssignmentList, ParseError<'a>> {
        let mut assigns = vec![];
        while self.cursor < self.buf.len() {
            let assign = self.parse_assignment()?;
            assigns.push(assign);
        }
        Ok(AssignmentList { assigns })
    }

    fn parse_id(&mut self) -> Result<Id, ParseError<'a>> {
        if self.cursor >= self.buf.len() {
            return Err(Wrap::new(NoMoreToLexError {}));
        }

        let cursor = self.cursor;
        self.cursor += 1;
        match &self.buf[cursor] {
            Token {
                kind: TokenKind::Id(id),
                ..
            } => Ok(id.clone()),
            t => Err(Wrap::new(UnexpectedToken {
                found_token: t.clone(),
                expected_token_kind: TokenKind::Id("<id>".to_string()),
            })),
        }
    }

    fn parse_simple_token_kind(
        &mut self,
        tok: TokenKind,
    ) -> Result<(), ParseError<'a>> {
        if self.cursor >= self.buf.len() {
            return Err(Wrap::new(NoMoreToLexError {}));
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
            Err(Wrap::new(UnexpectedToken {
                found_token: cur_tok,
                expected_token_kind: tok,
            }))
        }
    }

    fn parse_id_list(&mut self) -> Result<Vec<Id>, ParseError<'a>> {
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

    fn parse_function(&mut self) -> Result<Function, ParseError<'a>> {
        let name = self.parse_id()?;
        self.parse_simple_token_kind(TokenKind::OpenParen)?;
        let args = self.parse_id_list()?;
        self.parse_simple_token_kind(TokenKind::CloseParen)?;
        Ok(Function { name, args })
    }

    fn parse_assignment(&mut self) -> Result<Assignment, ParseError<'a>> {
        let vars = self.parse_id_list()?;
        self.parse_simple_token_kind(TokenKind::Assign)?;
        let value = self.parse_function()?;
        self.parse_simple_token_kind(TokenKind::Semicolon)?;
        Ok(Assignment { vars, value })
    }
}
