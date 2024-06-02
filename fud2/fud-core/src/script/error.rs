use rhai::EvalAltResult;

use std::fmt::Display;

#[derive(Debug)]
pub(super) struct RhaiSystemError {
    kind: RhaiSystemErrorKind,
    pub position: rhai::Position,
}

#[derive(Debug)]
pub(super) enum RhaiSystemErrorKind {
    ErrorSetupRef(String),
}

impl RhaiSystemError {
    pub(super) fn setup_ref(v: rhai::Dynamic) -> Self {
        Self {
            kind: RhaiSystemErrorKind::ErrorSetupRef(v.to_string()),
            position: rhai::Position::NONE,
        }
    }

    pub(super) fn with_pos(mut self, p: rhai::Position) -> Self {
        self.position = p;
        self
    }
}

impl Display for RhaiSystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            RhaiSystemErrorKind::ErrorSetupRef(v) => {
                write!(f, "Unable to construct SetupRef: `{v:?}`")
            }
        }
    }
}

impl std::error::Error for RhaiSystemError {}

impl Into<Box<EvalAltResult>> for RhaiSystemError {
    fn into(self) -> Box<EvalAltResult> {
        Box::new(EvalAltResult::ErrorSystem("".to_string(), Box::new(self)))
    }
}
