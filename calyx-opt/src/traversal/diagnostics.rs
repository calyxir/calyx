use calyx_utils::{CalyxResult, Error};

use super::{Action, VisResult};

/// A pass that implements reporting Diagnostics
pub trait DiagnosticPass {
    /// Return an iterator of the diagnostics gathered by this pass.
    fn diagnostics(&self) -> &DiagnosticContext;
}

/// A type for accumulating multiple errors
#[derive(Default, Debug)]
pub struct DiagnosticContext {
    errors: Vec<Error>,
    warnings: Vec<Error>,
}

impl DiagnosticContext {
    /// Report an `error`
    pub fn err(&mut self, error: Error) {
        self.errors.push(error);
    }

    /// Report a `warning`
    pub fn warning(&mut self, warning: Error) {
        self.warnings.push(warning)
    }

    /// Accumulates `error` into the context, and returns `Ok(Action::Continue)`.
    /// This is useful for when we need to raise an Error because we couldn't
    /// construct some value that we needed to continue the computation.
    pub fn early_return_err(&mut self, error: Error) -> VisResult {
        self.err(error);
        Ok(Action::Continue)
    }

    pub fn warning_iter(&self) -> impl Iterator<Item = &Error> {
        self.warnings.iter()
    }

    pub fn errors_iter(&self) -> impl Iterator<Item = &Error> {
        self.errors.iter()
    }
}

/// Accumuate the error in a [`Result`] type into the [`DiagnosticContext`].
pub trait DiagnosticResult {
    fn accumulate_err(self, diag: &mut DiagnosticContext) -> Self;
}

impl<T> DiagnosticResult for CalyxResult<T>
where
    T: Default,
{
    fn accumulate_err(self, diag: &mut DiagnosticContext) -> Self {
        match self {
            Ok(act) => Ok(act),
            Err(err) => {
                diag.err(err);
                Ok(T::default())
            }
        }
    }
}

impl DiagnosticResult for VisResult {
    fn accumulate_err(self, diag: &mut DiagnosticContext) -> Self {
        match self {
            Ok(act) => Ok(act),
            Err(err) => {
                diag.err(err);
                Ok(Action::Continue)
            }
        }
    }
}
