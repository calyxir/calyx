// Abstract Syntax Tree for Futil. See link below for the grammar
// https://github.com/cucapra/futil/blob/master/grammar.md

pub type Id = String;

#[derive(Debug)]
pub struct Namespace {
    pub name: String,
    pub components: Vec<Component>,
}

#[derive(Clone, Debug)]
pub struct Component {
    pub name: String,
    pub inputs: Vec<Portdef>,
    pub outputs: Vec<Portdef>,
    pub structure: Vec<Structure>,
    pub control: Control,
}

#[derive(Clone, Debug)]
pub struct Portdef {
    pub name: Id,
    pub width: i64,
}

#[derive(Clone, Debug)]
pub enum Structure {
    Decl { name: Id, component: String },
    Std { name: Id, instance: Compinst },
    Wire { src: Port, dest: Port },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Port {
    Comp { component: Id, port: String },
    This { port: String },
}

#[derive(Clone, Debug)]
pub struct Compinst {
    pub name: Id,
    pub params: Vec<i64>,
}

// ==================================
// Data definitions for Control Ast
// ===================================

#[derive(Debug, Clone)]
pub struct Seq {
    pub stmts: Vec<Control>,
}

#[derive(Debug, Clone)]
pub struct Par {
    pub stmts: Vec<Control>,
}

#[derive(Debug, Clone)]
pub struct If {
    pub cond: Port,
    pub tbranch: Box<Control>,
    pub fbranch: Box<Control>,
}

#[derive(Debug, Clone)]
pub struct Ifen {
    pub cond: Port,
    pub tbranch: Box<Control>,
    pub fbranch: Box<Control>,
}

#[derive(Debug, Clone)]
pub struct While {
    pub cond: Port,
    pub body: Box<Control>,
}

#[derive(Debug, Clone)]
pub struct Print {
    pub var: String,
}

#[derive(Debug, Clone)]
pub struct Enable {
    pub comps: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Disable {
    pub comps: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Empty {}

// Need Boxes for recursive data structure
// Cannot have recursive data structure without
// indirection
#[derive(Debug, Clone)]
pub enum Control {
    Seq { data: Seq },
    Par { data: Par },
    If { data: If },
    Ifen { data: Ifen },
    While { data: While },
    Print { data: Print },
    Enable { data: Enable },
    Disable { data: Disable },
    Empty { data: Empty },
}
