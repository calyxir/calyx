use crate::error::{LocalError, LocalResult};
use figment::providers::Format;
use figment::value::Value;
use figment::Figment;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ConfigVarValidator(fn(&Value) -> LocalResult<()>);

impl ConfigVarValidator {
    pub fn new(predicate: fn(&Value) -> LocalResult<()>) -> Self {
        Self(predicate)
    }
}

impl Default for ConfigVarValidator {
    fn default() -> Self {
        Self(|_| Ok(()))
    }
}

#[derive(Debug, Clone)]
pub struct ConfigVar {
    key: String,
    description: String,
    validator: ConfigVarValidator,
}

impl ConfigVar {
    pub(crate) fn from<S: AsRef<str>, T: AsRef<str>>(
        key: S,
        description: T,
        validator: ConfigVarValidator,
    ) -> Self {
        Self {
            key: key.as_ref().to_string(),
            description: description.as_ref().to_string(),
            validator,
        }
    }

    pub fn key(&self) -> &String {
        &self.key
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn validate(&self, value: &Value) -> LocalResult<()> {
        self.validator.0(value)
    }
}

pub struct Config {
    /// DO NOT USE DIRECTLY. use [`Config::get`] instead.
    figment: Figment,
    profile: String, // since figment doesn't want to work
    required: Vec<ConfigVar>,
}

impl Config {
    pub fn from<P: AsRef<Path>, S: AsRef<str>>(
        path: P,
        profile: S,
    ) -> LocalResult<Self> {
        use figment::providers::Toml;
        let toml = Toml::file(path);
        Ok(Self {
            figment: Figment::from(toml),
            profile: profile.as_ref().to_string(),
            required: Vec::new(),
        })
    }

    pub fn get<S: AsRef<str>>(&self, key: S) -> LocalResult<Value> {
        self.figment
            .find_value(&self.fix_key(key))
            .map_err(Into::into)
    }

    pub fn require<S: AsRef<str>, T: AsRef<str>, V: Into<Value>>(
        &mut self,
        key: S,
        default: Option<V>,
        description: T,
        validator: ConfigVarValidator,
    ) {
        if let Some(default) = default {
            if self.get(key.as_ref()).is_err() {
                let new_figment = std::mem::take(&mut self.figment);
                self.figment = new_figment
                    .join((self.fix_key(key.as_ref()), default.into()));
            }
        }
        self.required
            .push(ConfigVar::from(key, description, validator));
    }

    pub(crate) fn doctor(&self) -> LocalResult<()> {
        let mut errors = vec![];
        for required_key in &self.required {
            match self.get(&required_key.key) {
                Ok(value) => required_key.validate(&value)?,
                Err(error) => {
                    errors.push((required_key.clone(), Box::new(error)))
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(LocalError::MissingConfig(errors))
        }
    }

    fn fix_key<S: AsRef<str>>(&self, key: S) -> String {
        format!("{}.{}", self.profile, key.as_ref())
    }
}
