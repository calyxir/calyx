use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Structure to generate unique names that are somewhat readable
#[derive(Debug)]
pub struct NameGenerator {
    name_hash: HashMap<String, i64>,
}

impl Default for NameGenerator {
    fn default() -> Self {
        NameGenerator {
            name_hash: HashMap::new(),
        }
    }
}

#[allow(unused)]
impl NameGenerator {
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
#[allow(unused)]
pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
