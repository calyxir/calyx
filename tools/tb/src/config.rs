use crate::error::{LocalError, LocalResult};
use figment::providers::Format;
use figment::value::Value;
use figment::Figment;
use std::fmt::Debug;
use std::path::Path;
use std::rc::Rc;

pub type ConfigVarValidatorPredicate = fn(&Value) -> LocalResult<()>;

/// TODO: make this declarative, allow building complex things in some sort of
/// eDSL fashion, with helpers for like "this must be a string", "this must be a
/// command and running it yields this output", etc.
pub struct ConfigVarValidator {
    predicates: Vec<ConfigVarValidatorPredicate>,
}

impl ConfigVarValidator {
    pub fn when(predicate: ConfigVarValidatorPredicate) -> Self {
        Self {
            predicates: vec![predicate],
        }
    }

    pub fn and(mut self, predicate: ConfigVarValidatorPredicate) -> Self {
        self.predicates.push(predicate);
        self
    }

    pub(crate) fn run(&self, value: &Value) -> LocalResult<()> {
        self.predicates
            .iter()
            .try_for_each(|predicate| predicate(value))
    }
}

impl Default for ConfigVarValidator {
    fn default() -> Self {
        Self::when(|_| Ok(()))
    }
}

impl Debug for ConfigVarValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ConfigVarValidator {{ predicates: vec![{}] }}",
            self.predicates
                .iter()
                .map(|p| format!("{:p}", p))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[derive(Debug, Clone)]
pub struct ConfigVar {
    key: String,
    description: String,
    validator: Rc<ConfigVarValidator>,
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
            validator: Rc::new(validator),
        }
    }

    pub fn key(&self) -> &String {
        &self.key
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn validate(&self, value: &Value) -> LocalResult<()> {
        self.validator.run(value)
    }
}

#[derive(Debug)]
pub enum InvalidConfigVar {
    Missing(ConfigVar, Box<LocalError>),
    Incorrect(ConfigVar, Box<LocalError>),
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
                self.set(&key, default);
            }
        }
        self.required
            .push(ConfigVar::from(key, description, validator));
    }

    pub(crate) fn doctor(&self) -> LocalResult<()> {
        let mut errors = vec![];
        for required_key in &self.required {
            match self.get(&required_key.key) {
                Ok(value) => {
                    if let Err(error) = required_key.validate(&value) {
                        errors.push(InvalidConfigVar::Incorrect(
                            required_key.clone(),
                            Box::new(error),
                        ))
                    }
                }
                Err(error) => errors.push(InvalidConfigVar::Missing(
                    required_key.clone(),
                    Box::new(error),
                )),
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(LocalError::InvalidConfig(errors))
        }
    }

    pub(crate) fn set<S: AsRef<str>, V: Into<Value>>(
        &mut self,
        key: S,
        value: V,
    ) {
        let new_figment = std::mem::take(&mut self.figment);
        self.figment =
            new_figment.join((self.fix_key(key.as_ref()), value.into()));
    }

    fn fix_key<S: AsRef<str>>(&self, key: S) -> String {
        format!("{}.{}", self.profile, key.as_ref())
    }
}
