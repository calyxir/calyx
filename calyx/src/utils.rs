use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::{
    hash::{Hash, Hasher},
    io::Write,
    path::PathBuf,
    str::FromStr,
};

/// Simple HashMap-based name generator that generates new names for each
/// prefix.
/// **Note**: The name generator is *not* hygienic!
/// For example:
/// ```
/// namegen.gen_name("seq");  // Generates "seq0"
/// ... // 8 more calls.
/// namegen.gen_name("seq");  // Generates "seq10"
/// namegen.gen_name("seq1"); // CONFLICT! Generates "seq10".
/// ```
#[derive(Clone, Debug, Default)]
pub struct NameGenerator {
    name_hash: HashMap<String, i64>,
}

impl NameGenerator {
    /// Returns a new String that starts with `prefix`.
    /// For example:
    /// ```
    /// namegen.gen_name("seq");  // Generates "seq0"
    /// namegen.gen_name("seq");  // Generates "seq1"
    /// ```
    pub fn gen_name(&mut self, prefix: String) -> String {
        // Insert default value for this prefix if there is no entry.
        let count = self.name_hash.entry(prefix.clone())
            .and_modify(|v| *v += 1)
            .or_default();

        format!("{}{}", prefix, count)
    }
}

/// TODO(rachit): Document this.
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
