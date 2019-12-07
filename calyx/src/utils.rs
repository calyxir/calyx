use std::collections::HashMap;

/**
 * Combine concatenates [vec] into a single string, with each entry
 * separated by [delimiter], [start] prepended, and [end] appended to the end result.
 */
pub fn combine(vec: &[String], start: &str, delimiter: &str) -> String {
    if vec.is_empty() {
        "".to_string()
    } else {
        let mut s = String::new();
        let n = vec.len() - 1;
        for x in vec.iter().take(n) {
            s.push_str(x);
            s.push_str(delimiter);
        }
        s.push_str(start);
        s.push_str(vec[n].as_ref());
        s
    }
}

/// Structure to generate unique names that are somewhat readable
pub struct NameGenerator {
    name_hash: HashMap<String, i64>,
}

impl NameGenerator {
    pub fn new() -> Self {
        NameGenerator {
            name_hash: HashMap::new(),
        }
    }

    pub fn gen_name(&mut self, name: &str) -> String {
        let count = match self.name_hash.get(name) {
            None => 0,
            Some(c) => *c,
        };
        self.name_hash.insert(name.to_string(), count + 1);
        format!("{}{}", name, count)
    }
}

#[derive(Debug)]
pub struct Scoped<T> {
    current: T,
    stack: Vec<T>,
}

pub trait WithDefault {
    fn default() -> Self;
}

impl<T: WithDefault + Clone> Scoped<T> {
    pub fn new() -> Self {
        Scoped {
            current: T::default(),
            stack: vec![],
        }
    }

    pub fn set(&mut self, thing: T) {
        self.current = thing;
    }

    pub fn get(&mut self) -> T {
        self.current.clone()
    }

    pub fn push_scope(&mut self) {
        self.stack.push(self.current.clone());
        self.current = T::default();
    }

    pub fn pop_scope(&mut self) {
        match self.stack.pop() {
            None => (),
            Some(x) => {
                self.current = x;
            }
        }
    }
}

impl<T> WithDefault for Option<T> {
    fn default() -> Self {
        None
    }
}
