//! An internal representation of fud2 plans expressed with flang.

use camino::Utf8PathBuf;
use cranelift_entity::{PrimaryMap, entity_impl};

use crate::exec::OpRef;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PathRef(u32);
entity_impl!(PathRef, "path");

#[derive(Debug, PartialEq)]
pub struct IrAssign {
    op: OpRef,
    args: Vec<PathRef>,
    rets: Vec<PathRef>,
}

impl IrAssign {
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

impl IrAssign {
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
    assignments: Vec<IrAssign>,
}

impl Ir {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an assignment to the current IR.
    pub fn push(&mut self, op: OpRef, args: &[PathRef], rets: &[PathRef]) {
        self.assignments.push(IrAssign::from_parts(op, args, rets));
    }

    /// Appends an assignment to the current IR using Vec args.
    pub fn push_vec(
        &mut self,
        op: OpRef,
        args: Vec<PathRef>,
        rets: Vec<PathRef>,
    ) {
        self.assignments.push(IrAssign::from_vecs(op, args, rets));
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

    pub fn iter<'a>(&'a self) -> Iter<'a> {
        self.into_iter()
    }
}

pub struct Iter<'a> {
    ir: &'a Ir,
    idx: usize,
}

impl<'a> IntoIterator for &'a Ir {
    type Item = &'a IrAssign;

    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter { ir: self, idx: 0 }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a IrAssign;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.ir.assignments.len() {
            let out = &self.ir.assignments[self.idx];
            self.idx += 1;
            Some(out)
        } else {
            None
        }
    }
}

impl IntoIterator for Ir {
    type Item = IrAssign;

    type IntoIter = std::vec::IntoIter<IrAssign>;

    fn into_iter(self) -> Self::IntoIter {
        self.assignments.into_iter()
    }
}
