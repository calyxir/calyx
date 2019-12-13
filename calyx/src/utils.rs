use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;

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
#[derive(Debug)]
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

/// Calculates the hash of hashable trait using the default hasher
pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

/// A generic data structure that supports scopes
#[derive(Debug)]
pub struct Scoped<T> {
    current: T,
    stack: Vec<T>,
}

/// Trait for things that have a default constructor
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

pub fn dot_command(file: &Option<PathBuf>) {
    match file {
        None => (),
        Some(p) => {
            let mut dot_file = p.clone();
            dot_file.set_extension("dot");
            let mut png_file = p.clone();
            png_file.set_extension("png");
            let _res = Command::new("dot")
                .args(&[
                    "-Tpng",
                    dot_file.to_str().unwrap(),
                    "-o",
                    png_file.to_str().unwrap(),
                ])
                .spawn();
        }
    }
}

pub fn ignore<T>(_t: T) {}
