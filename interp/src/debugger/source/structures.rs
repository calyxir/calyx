use calyx::errors::Error;
use serde::{self, de::Visitor, Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

use crate::errors::InterpreterResult;

#[derive(Hash, PartialEq, Eq, Debug, Clone)]

pub struct NamedTag(u64, String);

impl NamedTag {
    pub fn new_nameless(tag: u64) -> Self {
        Self(tag, String::new())
    }
}

impl From<(u64, String)> for NamedTag {
    fn from(i: (u64, String)) -> Self {
        Self(i.0, i.1)
    }
}

impl Serialize for NamedTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("({},{})", self.0, self.1))
    }
}

// TODO (Griffin): This whole thing needs to be replaced with a proper parser
// and not this whole nonsense. Kill me.
impl<'de> Deserialize<'de> for NamedTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NamedTagVisitor;

        impl<'de> Visitor<'de> for NamedTagVisitor {
            type Value = NamedTag;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str("string containing a two element tuple")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let mut list = v[1..(v.len() - 1)].split(',');
                let first = list.next().unwrap();
                let second = list.next().unwrap();

                let number: u64 = first.parse().unwrap();
                let string = second.trim().to_string();

                Ok(NamedTag(number, string))
            }
        }

        deserializer.deserialize_string(NamedTagVisitor)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct SourceMap(HashMap<NamedTag, String>);

impl SourceMap {
    /// Lookup the source location for the given named tag. Tags for a specific
    /// named instance are looked for first, falling back to position tags with
    /// an empty name if nothing more specific is available
    pub fn lookup(&self, key: (u64, String)) -> Option<&String> {
        let key = key.into();

        self.0
            .get(&key)
            .or_else(|| self.0.get(&NamedTag(key.0, "".to_string())))
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

    pub fn from_file_pest(
        path: &Option<PathBuf>,
    ) -> InterpreterResult<Option<Self>> {
        if let Some(path) = path {
            let v = fs::read(path)?;
            let file_contents = std::str::from_utf8(&v)?;
            let map: Self =
                super::metadata_parser::parse_metadata(file_contents)?;
            Ok(Some(map))
        } else {
            Ok(None)
        }
    }
}

impl From<HashMap<NamedTag, String>> for SourceMap {
    fn from(i: HashMap<NamedTag, String>) -> Self {
        Self(i)
    }
}
