use crate::config::ConfigVar;
use std::fmt::Display;

#[derive(Debug)]
pub enum LocalError {
    IO(std::io::Error),
    Figment(figment::Error),
    MissingConfig(Vec<(ConfigVar, Box<LocalError>)>),
    Other(String),
}

impl LocalError {
    pub fn other<S: AsRef<str>>(msg: S) -> Self {
        Self::Other(msg.as_ref().to_string())
    }
}

impl From<std::io::Error> for LocalError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<figment::Error> for LocalError {
    fn from(value: figment::Error) -> Self {
        Self::Figment(value)
    }
}

impl Display for LocalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalError::IO(io_err) => io_err.fmt(f),
            LocalError::Figment(figment_err) => figment_err.fmt(f),
            LocalError::MissingConfig(errors) => {
                writeln!(f, "We detected some errors in your config:")?;
                for (config_var, _) in errors {
                    writeln!(
                        f,
                        "- missing key '{}': {}",
                        config_var.key(),
                        config_var.description()
                    )?;
                }
                Ok(())
            }
            LocalError::Other(msg) => msg.fmt(f),
        }
    }
}

pub type LocalResult<T> = Result<T, LocalError>;
