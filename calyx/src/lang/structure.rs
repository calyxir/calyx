use crate::lang::ast;

#[derive(Clone, Debug)]
pub enum StructureStmt {
    Decl {
        name: ast::Id,
        component: String,
    },
    Std {
        name: ast::Id,
        instance: ast::Compinst,
    },
    Wire {
        src: ast::Port,
        dest: ast::Port,
    },
}

#[derive(Clone, Debug)]
pub struct Structure {
    stmts: Vec<StructureStmt>,
}

impl Structure {
    pub fn new(stmts: Vec<StructureStmt>) -> Structure {
        Structure { stmts: stmts }
    }
}
