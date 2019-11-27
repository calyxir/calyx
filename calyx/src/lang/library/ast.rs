// Abstract Syntax Tree for library declarations in Futil
use crate::lang::ast::Id;

#[derive(Clone, Debug)]
pub struct Library {
    pub primitives: Vec<Primitive>,
}

#[derive(Clone, Debug)]
pub struct Primitive {
    pub name: String,
    pub params: Vec<Id>,
    pub inputs: Vec<PrimPortdef>,
    pub outputs: Vec<PrimPortdef>,
}

#[derive(Clone, Debug)]
pub struct PrimPortdef {
    pub name: String,
    pub width: Width,
}

#[derive(Clone, Debug)]
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
