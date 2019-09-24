// Abstract Syntax Tree for Futil. See link below for the grammar
// https://github.com/cucapra/futil/blob/master/grammar.md

#[derive(Debug)]
pub struct Namespace {
    pub name: String,
    pub components: Vec<Component>,
}

#[derive(Debug)]
pub struct Component {
    pub name: String,
    pub inputs: Vec<Portdef>,
    pub outputs: Vec<Portdef>,
    pub structure: Vec<Structure>,
    pub control: Control,
}

#[derive(Debug)]
pub struct Portdef {
    pub name: String,
    pub width: i64,
}

#[derive(Debug)]
pub enum Structure {
    Decl { name: String, instance: Compinst },
    Wire { src: Port, dest: Port },
}

#[derive(Debug)]
pub enum Port {
    Comp { component: String, port: String },
    This { port: String },
}

#[derive(Debug)]
pub struct Compinst {
    pub name: String,
    pub param: Vec<i64>,
}

// Need Boxes for recursive data structure
// Cannot have recursive data structure without
// indirection
#[derive(Debug)]
pub enum Control {
    Seq {
        cexp: Vec<Control>,
    },
    Par {
        cexp: Vec<Control>,
    },
    If {
        cond: Port,
        t: Box<Control>,
        f: Box<Control>,
    },
    Ifen {
        cond: Port,
        t: Box<Control>,
        f: Box<Control>,
    },
    While {
        cond: Port,
        body: Box<Control>,
    },
    Print {
        id: String,
    },
    Enable {
        components: Vec<String>,
    },
    Disable {
        components: Vec<String>,
    },
    Empty,
}
