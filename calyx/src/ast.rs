use sexp::Sexp;

// Abstract Syntax Tree for Futil. See link below for the grammar
// https://github.com/cucapra/futil/blob/master/grammar.md

pub struct Namespace<'a> {
    name: &'a str,
    components: Vec<Component<'a>>,
}

pub struct Component<'a> {
    name: &'a str,
    inputs: Vec<Portdef<'a>>,
    outputs: Vec<Portdef<'a>>,
    structure: Vec<Structure<'a>>,
    control: Control<'a>,
}

pub struct Portdef<'a> {
    name: &'a str,
    port_width: i32,
}

pub enum Structure<'a> {
    New { name: &'a str, instance: Compinst<'a> },
    Wire { src: Port<'a>, dest: Port<'a> },
}

pub enum Port<'a> {
    Comp { component: &'a str, port: &'a str },
    This { port: &'a str },
}

pub struct Compinst<'a> {
    name: &'a str,
    param: Vec<i32>,
}

// Need Boxes for recursive data structure
// Cannot have recursive data structure without
// indirection
pub enum Control<'a> {
    Seq { cexp: Vec<Control<'a>> },
    Par { cexp: Vec<Control<'a>> },
    If { cond: Port<'a>, t: Box<Control<'a>>, f: Box<Control<'a>> },
    Ifen { cond: Port<'a>, t: Box<Control<'a>>, f: Box<Control<'a>> },
    While { cond: Port<'a>, body: Box<Control<'a>> },
    Print { id: &'a str },
    Enable { components: Vec<&'a str> },
    Disable { components: Vec<&'a str> },
    Empty {},
}