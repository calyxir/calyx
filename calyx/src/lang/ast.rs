// Abstract Syntax Tree for Futil. See link below for the grammar
// https://github.com/cucapra/futil/blob/master/grammar.md

pub type Id = String;

#[derive(Clone, Debug, Hash)]
pub struct Namespace {
    pub name: String,
    pub components: Vec<Component>,
}

#[derive(Clone, Debug, Hash)]
pub struct Component {
    pub name: String,
    pub inputs: Vec<Portdef>,
    pub outputs: Vec<Portdef>,
    pub structure: Vec<Structure>,
    pub control: Control,
}

#[derive(PartialEq, Clone, Debug, Hash)]
pub struct Portdef {
    pub name: String,
    pub width: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Port {
    Comp { component: Id, port: String },
    This { port: String },
}

#[derive(PartialEq, Clone, Debug, Hash)]
pub struct Compinst {
    pub name: String,
    pub params: Vec<i64>,
}

// ===================================
// Data definitions for Structure
// ===================================
#[derive(PartialEq, Clone, Debug, Hash)]
pub struct Decl {
    pub name: Id,
    pub component: String,
}

#[derive(PartialEq, Clone, Debug, Hash)]
pub struct Std {
    pub name: Id,
    pub instance: Compinst,
}

#[derive(PartialEq, Clone, Debug, Hash)]
pub struct Wire {
    pub src: Port,
    pub dest: Port,
}

#[derive(PartialEq, Clone, Debug, Hash)]
pub enum Structure {
    Decl { data: Decl },
    Std { data: Std },
    Wire { data: Wire },
}

#[allow(unused)]
impl Structure {
    pub fn decl(name: Id, component: String) -> Structure {
        Structure::Decl {
            data: Decl { name, component },
        }
    }

    pub fn std(name: Id, instance: Compinst) -> Structure {
        Structure::Std {
            data: Std { name, instance },
        }
    }

    pub fn wire(src: Port, dest: Port) -> Structure {
        Structure::Wire {
            data: Wire { src, dest },
        }
    }
}

// ===================================
// Data definitions for Control Ast
// ===================================

#[derive(Debug, Clone, Hash)]
pub struct Seq {
    pub stmts: Vec<Control>,
}

#[derive(Debug, Clone, Hash)]
pub struct Par {
    pub stmts: Vec<Control>,
}

#[derive(Debug, Clone, Hash)]
pub struct If {
    pub port: Port,
    pub cond: Vec<String>,
    pub tbranch: Box<Control>,
    pub fbranch: Box<Control>,
}

#[derive(Debug, Clone, Hash)]
pub struct Ifen {
    pub port: Port,
    pub cond: Vec<String>,
    pub tbranch: Box<Control>,
    pub fbranch: Box<Control>,
}

#[derive(Debug, Clone, Hash)]
pub struct While {
    pub port: Port,
    pub cond: Vec<String>,
    pub body: Box<Control>,
}

#[derive(Debug, Clone, Hash)]
pub struct Print {
    pub var: String,
}

#[derive(Debug, Clone, Hash)]
pub struct Enable {
    pub comps: Vec<String>,
}

#[derive(Debug, Clone, Hash)]
pub struct Disable {
    pub comps: Vec<String>,
}

#[derive(Debug, Clone, Hash)]
pub struct Empty {}

// Need Boxes for recursive data structure
// Cannot have recursive data structure without
// indirection
#[derive(Debug, Clone, Hash)]
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

#[allow(unused)]
impl Control {
    pub fn seq(stmts: Vec<Control>) -> Control {
        Control::Seq {
            data: Seq { stmts },
        }
    }

    pub fn par(stmts: Vec<Control>) -> Control {
        Control::Par {
            data: Par { stmts },
        }
    }

    // pub fn c_if(cond: Port, tbranch: Control, fbranch: Control) -> Control {
    //     Control::If {
    //         data: If {
    //             cond,
    //             tbranch: Box::new(tbranch),
    //             fbranch: Box::new(fbranch),
    //         },
    //     }
    // }

    // pub fn ifen(cond: Port, tbranch: Control, fbranch: Control) -> Control {
    //     Control::Ifen {
    //         data: Ifen {
    //             cond,
    //             tbranch: Box::new(tbranch),
    //             fbranch: Box::new(fbranch),
    //         },
    //     }
    // }

    // pub fn c_while(cond: Port, body: Control) -> Control {
    //     Control::While {
    //         data: While {
    //             cond,
    //             body: Box::new(body),
    //         },
    //     }
    // }

    pub fn print(var: String) -> Control {
        Control::Print {
            data: Print { var },
        }
    }

    pub fn enable(comps: Vec<String>) -> Control {
        Control::Enable {
            data: Enable { comps },
        }
    }

    pub fn disable(comps: Vec<String>) -> Control {
        Control::Disable {
            data: Disable { comps },
        }
    }

    pub fn empty() -> Control {
        Control::Empty { data: Empty {} }
    }
}
