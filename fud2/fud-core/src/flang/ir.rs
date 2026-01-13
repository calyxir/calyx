//! An internal representation of fud2 plans expressed with flang.

use camino::Utf8PathBuf;
use cranelift_entity::{PrimaryMap, entity_impl};

use crate::exec::OpRef;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PathRef(u32);
entity_impl!(PathRef, "path");

#[derive(Debug, PartialEq)]
/// A `Step` is a call to an op taking in the files `args` and assigning the results to the files `rets`.
pub struct Step {
    op: OpRef,
    args: Vec<PathRef>,
    rets: Vec<PathRef>,
}

impl Step {
    pub fn op_ref(&self) -> OpRef {
        self.op
    }

    pub fn args(&self) -> &[PathRef] {
        &self.args
    }

    pub fn rets(&self) -> &[PathRef] {
        &self.rets
    }
}

impl Step {
    fn from_parts(op: OpRef, args: &[PathRef], rets: &[PathRef]) -> Self {
        Self::from_vecs(op, args.to_vec(), rets.to_vec())
    }
    fn from_vecs(op: OpRef, args: Vec<PathRef>, rets: Vec<PathRef>) -> Self {
        Self { op, args, rets }
    }
}

/// A flang program.
#[derive(Default, Debug, PartialEq)]
pub struct Ir {
    paths: PrimaryMap<PathRef, Utf8PathBuf>,
    steps: Vec<Step>,
}

impl Ir {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an op to the current IR.
    pub fn push(&mut self, op: OpRef, args: &[PathRef], rets: &[PathRef]) {
        self.steps.push(Step::from_parts(op, args, rets));
    }

    /// Appends an op to the current IR using Vec args.
    pub fn push_vec(
        &mut self,
        op: OpRef,
        args: Vec<PathRef>,
        rets: Vec<PathRef>,
    ) {
        self.steps.push(Step::from_vecs(op, args, rets));
    }

    /// Gets a `PathRef` give a reference to a `path`. If none is found, a new reference is
    /// created.
    pub fn path_ref(&mut self, path: &Utf8PathBuf) -> PathRef {
        for (r, p) in &self.paths {
            if p == path {
                return r;
            }
        }
        self.paths.push(path.clone())
    }

    pub fn path(&self, r: PathRef) -> &Utf8PathBuf {
        &self.paths[r]
    }

    pub fn iter(&self) -> impl Iterator<Item = &Step> {
        self.steps.iter()
    }
}

impl<'a> IntoIterator for &'a Ir {
    type Item = &'a Step;
    type IntoIter = std::slice::Iter<'a, Step>;

    fn into_iter(self) -> Self::IntoIter {
        self.steps.iter()
    }
}

impl IntoIterator for Ir {
    type Item = Step;
    type IntoIter = std::vec::IntoIter<Step>;

    fn into_iter(self) -> Self::IntoIter {
        self.steps.into_iter()
    }
}
