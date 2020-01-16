// Abstract Syntax Tree for library declarations in Futil
use crate::errors::Error;
use crate::lang::ast::Id;
use sexpy::Sexpy;
use std::fs;
use std::path::PathBuf;

pub fn parse_file(file: &PathBuf) -> Result<Library, Error> {
    let content = &fs::read(file)?;
    let string_content = std::str::from_utf8(content)?;
    match Library::parse(string_content) {
        Ok(ns) => Ok(ns),
        Err(msg) => Err(Error::ParseError(msg)),
    }
}

#[derive(Sexpy, Clone, Debug)]
#[sexpy(head = "define/library")]
pub struct Library {
    pub primitives: Vec<Primitive>,
}

#[derive(Sexpy, Clone, Debug)]
#[sexpy(head = "define/prim")]
pub struct Primitive {
    pub name: String,
    #[sexpy(surround)]
    pub params: Vec<Id>,
    #[sexpy(surround)]
    pub inputs: Vec<PrimPortdef>,
    #[sexpy(surround)]
    pub outputs: Vec<PrimPortdef>,
}

#[derive(Sexpy, Clone, Debug)]
#[sexpy(head = "port")]
pub struct PrimPortdef {
    pub name: String,
    pub width: Width,
}

#[derive(Sexpy, Clone, Debug)]
#[sexpy(nohead, nosurround)]
pub enum Width {
    Const { value: i64 },
    Param { value: Id },
}

impl Library {
    #[allow(unused)]
    pub fn new() -> Library {
        let lib: Vec<Primitive> = Vec::new();
        Library { primitives: lib }
    }

    #[allow(unused)]
    pub fn merge(libraries: Vec<Library>) -> Library {
        let mut primitives = vec![];
        for lib in libraries {
            primitives.extend(lib.primitives.into_iter())
        }
        Library { primitives }
    }
}
