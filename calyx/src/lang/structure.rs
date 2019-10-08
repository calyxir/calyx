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

/** Structure holds information about the structure of the current component. */
#[derive(Clone, Debug)]
pub struct Structure {
    stmts: Vec<StructureStmt>,
}

impl Structure {
    // Control the creation method of Structure
    pub fn new(stmts: Vec<StructureStmt>) -> Structure {
        Structure { stmts: stmts }
    }

    // more future methods for manipulating the structure
}
