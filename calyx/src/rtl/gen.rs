use crate::lang::ast::*;
use crate::utils::combine;
use std::collections::HashMap;
use std::fs;

// Connections is a hashmap that maps src wires
// to the set of all of their destination wires
// This can then be used when instancing components
// to look up wire names
type Connections = HashMap<Port, Vec<Port>>;
// Environment type for all components in scope. This
// includes all primitives and all components in the
// same namespace
type Components = HashMap<String, Component>;

// Intermediate data structure conducive to string formatting
pub struct RtlInst {
    comp_name: String,
    id: String,
    params: Vec<i64>,
    ports: HashMap<String, String>,// Maps Port names to wires
}

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

pub fn gen_component(c: &Component, comp: &Components) -> (Component, Connections, Vec<RtlInst>) {
    unimplemented!();
}

fn gen_comp_ports(inputs: Vec<String>, outputs: Vec<String>) -> String {
    let mut strings = Vec::new();
    strings.extend(inputs);
    strings.extend(outputs);

    return combine(&strings, ",\n", "\n");
}

fn gen_outputs(vec: Vec<Portdef>) -> Vec<String> {
    let strings: Vec<String> = vec
        .into_iter()
        .map(|pd| format!("{}{}", "output ", gen_portdef(pd)))
        .collect();
    return strings;
}

fn gen_inputs(vec: Vec<Portdef>) -> Vec<String> {
    let strings: Vec<String> = vec
        .into_iter()
        .map(|pd| format!("{}{}", "input  ", gen_portdef(pd)))
        .collect();
    return strings;
}

fn gen_portdef(pd: Portdef) -> String {
    return format!("logic [{}:0] {}", pd.width - 1, pd.name);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portdef1() {
        let pd = Portdef {
            name: "in0".to_string(),
            width: 8,
        };
        let s = gen_portdef(pd);
        println!("{}", s);
        assert_eq!(s, "logic [7:0] in0");
    }

}
