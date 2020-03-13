use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;

/// Combine concatenates [vec] into a single string, with each entry
/// separated by [delimiter], [start] prepended, and [end] appended to the end result.
#[allow(unused)]
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

#[allow(unused)]
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
#[allow(unused)]
pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

/// Takes a path and an optional suffix and attempts to
/// run `dot` to generate a `png` for the graph. Will
/// silently fail if `dot` doesn't exist.
#[allow(unused)]
pub fn dot_command(p: &PathBuf, suffix: Option<&str>) {
    let mut p = p.clone();
    suffix.map_or((), |suffix| add_suffix(&mut p, suffix));
    let mut dot_file = p.clone();
    dot_file.set_extension("dot");
    let mut png_file = p.clone();
    png_file.set_extension("png");
    let _res = Command::new("dot")
        .args(&[
            "-Tpng",
            dot_file
                .to_str()
                .expect("`dot_file` could not be turned into a string"),
            "-o",
            png_file
                .to_str()
                .expect("`png_file` could not be turned into a string"),
        ])
        .spawn();
}

/// Ignore the return result of an operation
#[allow(unused)]
pub fn ignore<T>(_t: T) {}

/// hacky method to add suffix to file stem. don't think there's a
/// better way though
#[allow(unused)]
pub fn add_suffix(path: &mut PathBuf, suffix: &str) {
    let cl = path.clone();
    let mut file = cl.file_stem().unwrap().to_str().unwrap().to_string();
    let ext = cl.extension();
    file.push_str(suffix);
    path.set_file_name(file);
    ext.map_or((), |x| ignore(path.set_extension(x)));
}
