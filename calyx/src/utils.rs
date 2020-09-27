use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::{
    hash::{Hash, Hasher},
    io::Write,
    path::PathBuf,
    str::FromStr,
};

/// Structure to generate unique names that are somewhat readable
#[derive(Clone, Debug)]
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

#[derive(Debug)]
pub enum OutputFile {
    Stdout,
    File(PathBuf),
}

impl FromStr for OutputFile {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "-" => Ok(OutputFile::Stdout),
            _ => Ok(OutputFile::File(PathBuf::from(s))),
        }
    }
}

impl ToString for OutputFile {
    fn to_string(&self) -> String {
        match self {
            OutputFile::Stdout => "-".to_string(),
            OutputFile::File(p) => p.to_str().unwrap().to_string(),
        }
    }
}

impl Default for OutputFile {
    fn default() -> Self {
        OutputFile::Stdout
    }
}

impl OutputFile {
    pub fn isatty(&self) -> bool {
        match self {
            OutputFile::Stdout => atty::is(atty::Stream::Stdout),
            OutputFile::File(_) => false,
        }
    }

    pub fn get_write(&self) -> Box<dyn Write> {
        match self {
            OutputFile::Stdout => Box::new(std::io::stdout()),
            OutputFile::File(path) => {
                Box::new(std::fs::File::create(path).unwrap())
            }
        }
    }
}

/// Calculates the hash of hashable trait using the default hasher
#[allow(unused)]
pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
