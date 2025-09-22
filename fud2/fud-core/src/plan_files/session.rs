use std::{fs, io};

use camino::{Utf8Path, Utf8PathBuf};

use super::{
    ast::AssignmentList,
    error::ParseError,
    parser::{Lexer, Parser},
};

#[derive(Debug)]
pub struct ParseSession {
    buf: Vec<char>,
    file_path: Option<Utf8PathBuf>,
}

impl ParseSession {
    pub fn from_path(file_path: &Utf8Path) -> io::Result<Self> {
        if !file_path.is_file() {
            let msg = format!("{file_path} is not a file");
            return Err(io::Error::other(msg));
        };

        let buf = fs::read_to_string(file_path)?.chars().collect();
        let owned_path = file_path.to_owned();
        Ok(ParseSession {
            buf,
            file_path: Some(owned_path),
        })
    }

    pub fn with_str_buf(src: &str) -> Self {
        let char_buf = src.chars().collect();
        ParseSession {
            buf: char_buf,
            file_path: None,
        }
    }

    pub fn buf(&self) -> &[char] {
        &self.buf
    }

    pub fn file_path(&self) -> &Option<Utf8PathBuf> {
        &self.file_path
    }

    pub fn parse(&self) -> Result<AssignmentList, ParseError> {
        let mut toks = vec![];
        let mut lexer = Lexer::from_parts(self, 0);
        while lexer.has_next_token() {
            let tok = lexer.next_token()?;
            toks.push(tok);
        }
        let mut parser = Parser::from_parts(toks, self);
        parser.parse()
    }
}
