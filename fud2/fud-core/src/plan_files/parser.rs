//! The recursive decent parser for parsing plan files.

use camino::Utf8PathBuf;
use std::error::Error;

use super::{ast::*, error::*, session::ParseSession, span::Span};

#[derive(Debug)]
pub(super) struct Lexer<'a> {
    sess: &'a ParseSession,
    cursor: usize,
}

const ESCAPE_CODES: [char; 2] = ['"', '\\'];

const VALID_NON_ALPHANUMERIC_CHARACTERS: [char; 6] =
    ['-', '_', '/', '\\', '.', ':'];

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

    pub fn lex_char(&mut self) -> Result<char, Wrap<dyn Error>> {
        let buf = self.sess.buf();
        if self.cursor >= buf.len() {
            return Err(Wrap::new(NoMoreToLexError {}));
        }
        // We are dealing with an escape character.
        if buf[self.cursor] == '\\' {
            if self.cursor + 1 >= buf.len() {
                return Err(Wrap::new(NoMoreToLexError {}));
            }
            let c = buf[self.cursor + 1];
            if ESCAPE_CODES.contains(&c) {
                self.cursor += 2;
                Ok(c)
            } else {
                Err(UnexpectedCharError::from_parts(Some(c), &ESCAPE_CODES))
            }
        } else {
            let c = buf[self.cursor];
            self.cursor += 1;
            Ok(c)
        }
    }

    pub fn lex_shorthand_id(
        &mut self,
    ) -> Result<Token<'a>, Wrap<dyn Error + 'a>> {
        let buf = self.sess.buf();
        let first_char = buf[self.cursor];
        if !first_char.is_alphabetic()
            && !VALID_NON_ALPHANUMERIC_CHARACTERS.contains(&first_char)
        {
            let error = InvalidStartToIdError {
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
            .take_while(|&&c| {
                c.is_alphanumeric()
                    || VALID_NON_ALPHANUMERIC_CHARACTERS.contains(&c)
            })
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

    pub fn next_token(&mut self) -> Result<Token<'a>, Wrap<dyn Error + 'a>> {
        let buf = self.sess.buf();
        if self.cursor >= buf.len() {
            return Err(UnexpectedCharError::from_parts(None, &['a']));
        }

        // This language is whitespace agnostic because semantic whitespace is weirdly hard to
        // implement while also allowing unicode.
        while buf[self.cursor].is_whitespace() {
            self.cursor += 1;
            if self.cursor >= buf.len() {
                return Err(UnexpectedCharError::from_parts(None, &['a']));
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
        // Identifiers are defined as a '"' followed by some characters with '"'s and '\' special
        // and escaped using '\' and a closing '"'.
        //
        // Identifiers must be extremely lax because they correspond to file paths which can
        // contain very many strange characters.
        //
        // For a shorthand, users can drop the quotes but cannot have whitespace or some stranger
        // characters in their identifiers.
        let first_char = buf[self.cursor];
        if first_char != '"' {
            self.lex_shorthand_id()
        } else {
            let lo = self.cursor;
            let mut id_string = vec![];
            self.cursor += 1;
            while self.cursor < buf.len() && buf[self.cursor] != '"' {
                id_string.push(self.lex_char()?);
            }
            if self.cursor >= buf.len() {
                return Err(Wrap::new(NoMoreToLexError {}));
            }

            let hi = self.cursor;
            self.cursor += 1;
            let span = Span {
                sess: self.sess,
                lo,
                hi,
            };
            Ok(Token {
                kind: TokenKind::Id(id_string.iter().collect()),
                span,
            })
        }
    }
}

type TokenStream<'a> = Vec<Token<'a>>;

pub struct Parser<'a> {
    buf: Vec<Token<'a>>,
    sess: &'a ParseSession,
    cursor: usize,
    cache: Option<Result<AssignmentList, Wrap<dyn Error + 'a>>>,
}

/// TODO: make this a state machine for better parse errors.
impl<'a> Parser<'a> {
    pub fn from_parts(buf: TokenStream<'a>, sess: &'a ParseSession) -> Self {
        Parser {
            buf,
            sess,
            cursor: 0,
            cache: None,
        }
    }

    pub fn parse(&mut self) -> Result<AssignmentList, Wrap<dyn Error + 'a>> {
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
    ) -> Result<AssignmentList, Wrap<dyn Error + 'a>> {
        let mut assigns = vec![];
        while self.cursor < self.buf.len() {
            let assign = self.parse_assignment()?;
            assigns.push(assign);
        }
        Ok(AssignmentList { assigns })
    }

    fn parse_var_id(&mut self) -> Result<VarId, Wrap<dyn Error + 'a>> {
        if self.cursor >= self.buf.len() {
            return Err(Wrap::new(NoMoreToLexError {}));
        }

        let cursor = self.cursor;
        self.cursor += 1;
        match &self.buf[cursor] {
            Token {
                kind: TokenKind::Id(id),
                ..
            } => Ok(Utf8PathBuf::from(id)),
            t => Err(Wrap::new(UnexpectedTokenError {
                found_token: t.clone(),
                expected_token_kind: TokenKind::Id("<id>".to_string()),
            })),
        }
    }

    fn parse_fun_id(&mut self) -> Result<FunId, Wrap<dyn Error + 'a>> {
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
            t => Err(Wrap::new(UnexpectedTokenError {
                found_token: t.clone(),
                expected_token_kind: TokenKind::Id("<id>".to_string()),
            })),
        }
    }

    fn parse_simple_token_kind(
        &mut self,
        tok: TokenKind,
    ) -> Result<(), Wrap<dyn Error + 'a>> {
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
            Err(Wrap::new(UnexpectedTokenError {
                found_token: cur_tok,
                expected_token_kind: tok,
            }))
        }
    }

    fn parse_id_list(&mut self) -> Result<Vec<VarId>, Wrap<dyn Error + 'a>> {
        if !matches!(self.buf[self.cursor].kind, TokenKind::Id(_)) {
            return Ok(vec![]);
        }

        let mut res = vec![];
        let var = self.parse_var_id()?;
        res.push(var);
        while matches!(self.buf[self.cursor].kind, TokenKind::Comma) {
            self.parse_simple_token_kind(TokenKind::Comma)?;
            let var = self.parse_var_id()?;
            res.push(var);
        }
        Ok(res)
    }

    fn parse_function(&mut self) -> Result<Op, Wrap<dyn Error + 'a>> {
        let name = self.parse_fun_id()?;
        self.parse_simple_token_kind(TokenKind::OpenParen)?;
        let args = self.parse_id_list()?;
        self.parse_simple_token_kind(TokenKind::CloseParen)?;
        Ok(Op { name, args })
    }

    fn parse_assignment(&mut self) -> Result<Assignment, Wrap<dyn Error + 'a>> {
        let vars = self.parse_id_list()?;
        if vars.is_empty() {
            let span = Span {
                hi: self.cursor,
                lo: self.cursor,
                sess: self.sess,
            };
            return Err(Wrap::new(EmptyVarListInAssignment { _span: span }));
        }
        self.parse_simple_token_kind(TokenKind::Assign)?;
        let value = self.parse_function()?;
        self.parse_simple_token_kind(TokenKind::Semicolon)?;
        Ok(Assignment { vars, value })
    }
}
