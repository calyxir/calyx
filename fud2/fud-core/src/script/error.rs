use rhai::EvalAltResult;

use std::fmt::Display;

#[derive(Debug)]
pub(super) struct RhaiSystemError {
    kind: RhaiSystemErrorKind,
    pub position: rhai::Position,
}

#[derive(Debug)]
pub(super) enum RhaiSystemErrorKind {
    SetupRef(String),
    StateRef(String),
    BeganOp(String, String),
    NoOp,
    NoDep(String),
    DupTarget(String),

    /// The string is the type name of non-string value.
    ExpectedString(String),
    ExpectedShell,
    ExpectedShellDeps,
    EmptyOp,
}

impl RhaiSystemError {
    pub(super) fn setup_ref(v: rhai::Dynamic) -> Self {
        Self {
            kind: RhaiSystemErrorKind::SetupRef(v.to_string()),
            position: rhai::Position::NONE,
        }
    }

    pub(super) fn state_ref(v: rhai::Dynamic) -> Self {
        Self {
            kind: RhaiSystemErrorKind::StateRef(v.to_string()),
            position: rhai::Position::NONE,
        }
    }

    pub(super) fn began_op(old_name: &str, new_name: &str) -> Self {
        Self {
            kind: RhaiSystemErrorKind::BeganOp(
                old_name.to_string(),
                new_name.to_string(),
            ),
            position: rhai::Position::NONE,
        }
    }

    pub(super) fn no_op() -> Self {
        Self {
            kind: RhaiSystemErrorKind::NoOp,
            position: rhai::Position::NONE,
        }
    }

    pub(super) fn no_dep(dep: &str) -> Self {
        Self {
            kind: RhaiSystemErrorKind::NoDep(dep.to_string()),
            position: rhai::Position::NONE,
        }
    }

    pub(super) fn expected_string(v: &str) -> Self {
        Self {
            kind: RhaiSystemErrorKind::ExpectedString(v.to_string()),
            position: rhai::Position::NONE,
        }
    }

    pub(super) fn dup_target(target: &str) -> Self {
        Self {
            kind: RhaiSystemErrorKind::DupTarget(target.to_string()),
            position: rhai::Position::NONE,
        }
    }

    pub(super) fn expected_shell() -> Self {
        Self {
            kind: RhaiSystemErrorKind::ExpectedShell,
            position: rhai::Position::NONE,
        }
    }

    pub(super) fn expected_shell_deps() -> Self {
        Self {
            kind: RhaiSystemErrorKind::ExpectedShellDeps,
            position: rhai::Position::NONE,
        }
    }

    pub(super) fn empty_op() -> Self {
        Self {
            kind: RhaiSystemErrorKind::EmptyOp,
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
            RhaiSystemErrorKind::SetupRef(v) => {
                write!(f, "Unable to construct SetupRef: `{v:?}`")
            }
            RhaiSystemErrorKind::StateRef(v) => {
                write!(f, "Unable to construct StateRef: `{v:?}`")
            }
            RhaiSystemErrorKind::BeganOp(old_name, new_name) => {
                write!(f, "Unable to build two ops at once: trying to build `{new_name:?}` but already building `{old_name:?}`")
            }
            RhaiSystemErrorKind::NoOp => {
                write!(
                    f,
                    "Function may only be called within the body of an op."
                )
            }
            RhaiSystemErrorKind::NoDep(dep) => {
                write!(f, "Unable to find dep: `{dep:?}`. A call to `shell` with `{dep:?}` as an output must occur prior to this call.")
            }
            RhaiSystemErrorKind::ExpectedString(v) => {
                write!(f, "Expected string, got: `{v:?}`.")
            }
            RhaiSystemErrorKind::DupTarget(target) => {
                write!(f, "Duplicate target: `{target:?}`. Consider removing a shell command generating `{target:?}`.")
            }
            RhaiSystemErrorKind::ExpectedShell => {
                write!(f, "Expected `shell`, got `shell_deps`. Ops may contain only one of `shell` or `shell_deps` calls, not calls to both")
            }
            RhaiSystemErrorKind::ExpectedShellDeps => {
                write!(f, "Expected `shell_deps`, got `shell`. Ops may contain only one of `shell` or `shell_deps` calls, not calls to both")
            }
            RhaiSystemErrorKind::EmptyOp => {
                write!(f, "Error: Op must contain at least one call to `shell` or `shell_deps`.")
            }
        }
    }
}

impl std::error::Error for RhaiSystemError {}

impl From<RhaiSystemError> for Box<EvalAltResult> {
    fn from(value: RhaiSystemError) -> Self {
        Box::new(EvalAltResult::ErrorSystem("".to_string(), Box::new(value)))
    }
}
