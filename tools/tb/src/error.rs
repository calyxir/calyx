use crate::config::InvalidConfigVar;
use std::{fmt::Display, io};

#[derive(Debug)]
pub enum LocalError {
    IO(io::Error),
    Figment(figment::Error),
    InvalidConfig(Vec<InvalidConfigVar>),
    Other(String),
}

impl LocalError {
    pub fn other<S: AsRef<str>>(msg: S) -> Self {
        Self::Other(msg.as_ref().to_string())
    }
}

impl From<io::Error> for LocalError {
    fn from(value: io::Error) -> Self {
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
            LocalError::InvalidConfig(errors) => {
                writeln!(f, "We detected some errors in your config:")?;
                for error in errors {
                    match error {
                        InvalidConfigVar::Missing(config_var, _) => {
                            writeln!(
                                f,
                                "- missing key '{}': {}",
                                config_var.key(),
                                config_var.description()
                            )?;
                        }
                        InvalidConfigVar::Incorrect(config_var, error) => {
                            writeln!(
                                f,
                                "- incorrect key '{}': {}",
                                config_var.key(),
                                error
                            )?;
                        }
                    }
                }
                Ok(())
            }
            LocalError::Other(msg) => msg.fmt(f),
        }
    }
}

pub type LocalResult<T> = Result<T, LocalError>;
