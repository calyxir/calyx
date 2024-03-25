use std::vec;

use calyx_utils::{CalyxResult, Error};

use super::{Action, VisResult};

/// A type for accumulating multiple errors
#[derive(Default, Debug)]
pub struct DiagnosticContext {
    errors: Vec<Error>,
}

impl DiagnosticContext {
    pub fn err(&mut self, error: Error) {
        self.errors.push(error);
    }

    /// Accumulates `error` into the context, and returns `Ok(Action::Continue)`.
    /// This is useful for when we need to raise an Error because we couldn't
    /// construct some value that we needed to continue the computation.
    pub fn early_return_err(&mut self, error: Error) -> VisResult {
        self.err(error);
        Ok(Action::Continue)
    }
}

impl IntoIterator for DiagnosticContext {
    type Item = Error;
    type IntoIter = vec::IntoIter<Error>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

pub trait DiagnosticResult {
    fn accumulate_err(self, diag: &mut DiagnosticContext) -> Self;
}

pub trait DiagnosticPass {
    fn diagnostics(self) -> impl Iterator<Item = Error>;
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
