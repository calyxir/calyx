// Abstract Syntax Tree for Futil. See link below for the grammar
// https://github.com/cucapra/futil/blob/master/grammar.md

type Id = String;

#[derive(Debug)]
pub struct Namespace {
    pub name: Id,
    pub components: Vec<Component>,
}

#[derive(Debug)]
pub struct Component {
    pub name: Id,
    pub inputs: Vec<Portdef>,
    pub outputs: Vec<Portdef>,
    pub structure: Vec<Structure>,
    pub control: Control,
}

#[derive(Debug)]
pub struct Portdef {
    pub name: Id,
    pub width: i64,
}

#[derive(Debug)]
pub enum Structure {
    Decl { name: Id, instance: Compinst },
    Wire { src: Port, dest: Port },
}

#[derive(Debug)]
pub enum Port {
    Comp { component: Id, port: String },
    This { port: String },
}

#[derive(Debug)]
pub struct Compinst {
    pub name: Id,
    pub param: Vec<i64>,
}

// Need Boxes for recursive data structure
// Cannot have recursive data structure without
// indirection
#[derive(Debug)]
pub enum Control {
    Seq(Vec<Control>),

    Par(Vec<Control>),

    If {
        cond: Port,
        tbranch: Box<Control>,
        fbranch: Box<Control>,
    },
    Ifen {
        cond: Port,
        tbranch: Box<Control>,
        fbranch: Box<Control>,
    },
    While {
        cond: Port,
        body: Box<Control>,
    },
    Print(Id),
    Enable(Vec<String>),
    Disable(Vec<String>),
    Empty,
}
