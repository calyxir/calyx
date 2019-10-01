use crate::ast::*;

/**
 * Combine concatenates [vec] into a single string, with each entry
 * separated by [delimiter], and [end] appended to the end result
 */
pub fn combine(vec: Vec<String>, delimiter: &str, end: &str) -> String {
    let mut s = String::new();
    let n = vec.len() - 1;
    for i in 0..n {
        s.push_str(vec[i].as_ref());
        s.push_str(delimiter);
    }
    s.push_str(vec[n].as_ref());
    s.push_str(end);
    return s;
}

pub fn gen_namespace(n: Namespace) {}

pub fn gen_component(c: Component) {}

fn gen_comp_ports(inputs: Vec<String>, outputs: Vec<String>) -> String {
    let mut strings = Vec::new();
    strings.extend(inputs);
    strings.extend(outputs);

    return combine(strings, ",\n", "\n");
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
