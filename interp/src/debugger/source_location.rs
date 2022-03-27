use calyx::errors::Error;
use serde::{self, Deserialize};
use std::{collections::HashMap, fs, path::PathBuf};

type NamedTag = (String, u64);

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct SourceMap(HashMap<NamedTag, String>);

impl SourceMap {
    /// Lookup the source location for the given named tag. Tags for a specific
    /// named instance are looked for first, falling back to position tags with
    /// an empty name if nothing more specific is available
    pub fn lookup(&self, key: &NamedTag) -> Option<&String> {
        self.0
            .get(key)
            .or_else(|| self.0.get(&("".to_string(), key.1)))
    }

    pub fn from_file(path: &Option<PathBuf>) -> Result<Option<Self>, Error> {
        if let Some(path) = path {
            let v = fs::read(path)?;
            let file_contents = std::str::from_utf8(&v)?;
            let map: Self = serde_json::from_str(file_contents)
                .expect("Source map failed to deserialize");
            Ok(Some(map))
        } else {
            Ok(None)
        }
    }
}
