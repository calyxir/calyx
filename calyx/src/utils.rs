use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

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

pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
