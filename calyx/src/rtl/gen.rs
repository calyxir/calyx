use crate::lang::ast::*;
use crate::lang::structure::StructureStmt;
use crate::utils::combine;
use std::collections::HashMap;
use std::fs;
use crate::rtl::templates::*;

/**
 * This file generates the intermediate data structures in templates.rs from 
 * an AST
 */

pub fn gen_namespace(n: &Namespace, build_dir: String) {
    let dir = format!("{}{}/", build_dir, n.name);
    fs::create_dir_all(dir);

    // Initialize Component Store 
    let mut comp: Components = HashMap::new();
    for c in &n.components {
        comp.insert(c.name.clone(), c.clone());
    }

    // TODO Add primitives to component store
    // Initialize wire store
    let mut conn: Connections = HashMap::new();
    for c in &n.components {
        gen_component(c, &comp);
    }
}

// TODO generate control logic
pub fn gen_component(c: &Component, comp: &Components) -> (Component, Connections, Vec<RtlInst>) {
    let conn: Connections = gen_connections(&(c.structure.get_stmts()));
    let insts: Vec<RtlInst> = gen_insts(&c.structure.get_stmts(), &conn, &comp);

    unimplemented!();
}
// ==================================
// Generating Assign Statements 
// ==================================

// TODO clean me up- Generates all Wire connections from Structure of a component
fn gen_connections(structure: &Vec<StructureStmt>) -> Connections {
    // Construct connections
    let mut conn: Connections = HashMap::new();
    let f = |mut c: Connections, s: &StructureStmt| {
        match s {
            StructureStmt::Wire {src, dest} => {
                match c.get_mut(&src) {
                    Some(v) => {
                        v.push(dest.clone());
                        return c;
                    },
                    None => {
                        let _ = c.insert(src.clone(), vec![dest.clone()]);
                        return c;
                    }
                }
            },
            _ => return c, 
        }
    };

    let conn = structure.iter().fold(conn, f);
    return conn;
}

// ==================================
// Generating Subcomponent Instances 
// ==================================

/**
 * Fetches the list of input and output ports for a given component
 */
fn port_list(component: String, comp: &Components) -> Vec<Portdef> {
    match comp.get(&component) {
        Some(c) => {
            let mut v: Vec<Portdef> = Vec::new();
            v.append(&mut c.inputs.clone());
            v.append(&mut c.outputs.clone());
            return v;
        },
        None => panic!("Component {} not defined", component),
    }
}

/**
 * Finds the name of the wire that will connect to the input port
 * Very inefficient :(
 */
fn find_wire(c: &Connections, pd: Portdef, id: Id) -> String {
    let to_find = Port::Comp { component: id, port: pd.name };
    for (src, dests) in c {
        if to_find == *src || dests.contains(&to_find) {
            return port_wire_id(src);
        }
    }
    return "".to_string();
}

/**
 * Generates a hashmap of ports and their connections to instance structures
 */
fn gen_inst_ports(c: &Connections, id: String, component: String, comp: &Components) -> HashMap<String, String> {
    let portdefs: Vec<Portdef> = port_list(component, comp);
    let mut map: HashMap<String, String> = HashMap::new();
    for pd in portdefs {
        let p: Port = Port::Comp { component: id.clone(), port: pd.name.clone()};
        map.insert(port_wire_id(&p), find_wire(&c, pd, id.clone()));
    }

    return map;
}

// Generates all instances of subcomponents in a structure
fn gen_insts(structure: &Vec<StructureStmt>, c: &Connections, comp: &Components) -> Vec<RtlInst> {
    unimplemented!();
    let mut insts: Vec<RtlInst> = Vec::new();
    let f = |mut insts: Vec<RtlInst>, s: &StructureStmt| -> Vec<RtlInst> {
        match s {
            StructureStmt::Decl {name, component} => {
                let map = gen_inst_ports(c, name.clone(), component.clone(), comp);
                let new_inst = RtlInst {
                    comp_name: component.clone(),
                    id: name.clone(),
                    params: vec![],
                    ports: map,
                };
                insts.push(new_inst);
                return insts;
            },
            _ => return insts, 
        }
    };
}

// ==================================
// Generating Toplevel Component Signature 
// ==================================

fn gen_comp_ports(inputs: Vec<Portdef>, outputs: Vec<Portdef>) -> String {
    let mut strings = Vec::new();
    let in_ports = inputs.into_iter().map(|pd| in_port(pd.width, pd.name));
    let out_ports = outputs.into_iter().map(|pd| out_port(pd.width, pd.name));

    strings.extend(in_ports);
    strings.extend(out_ports);

    return combine(&strings, ",\n", "\n");
}

// ==================================
//               Tests 
// ==================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portdef1() {
        let pd = Portdef {
            name: "in0".to_string(),
            width: 8,
        };
        let s = in_port(pd.width, pd.name);
        println!("{}", s);
        assert_eq!(s, "input  logic [7:0] in0");
    }

    #[test]
    fn portdef2() {
        let pd = Portdef {
            name: "out0".to_string(),
            width: 8,
        };
        let s = out_port(pd.width, pd.name);
        println!("{}", s);
        assert_eq!(s, "output logic [7:0] out0");
    }

    #[test]
    fn portdef3() {
        let pd = Portdef {
            name: "in0".to_string(),
            width: 1,
        };
        let s = in_port(pd.width, pd.name);
        println!("{}", s);
        assert_eq!(s, "input  logic in0");
    }

    #[test]
    fn portdef4() {
        let pd = Portdef {
            name: "out0".to_string(),
            width: 1,
        };
        let s = out_port(pd.width, pd.name);
        println!("{}", s);
        assert_eq!(s, "output logic out0");
    }
}
