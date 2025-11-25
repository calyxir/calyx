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

/// The assignment lists of a flang program.
#[derive(Default, Debug, PartialEq)]
pub struct Ir {
    paths: PrimaryMap<PathRef, Utf8PathBuf>,

    /// The input files to be read from stdin.
    stdins: Vec<PathRef>,

    /// The input files to be written to stdout.
    stdouts: Vec<PathRef>,

    /// The input files.
    inputs: Vec<PathRef>,

    /// The output files.
    outputs: Vec<PathRef>,

    /// The list of steps in the IR
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

    pub fn path_ref_of_str(&mut self, path: &str) -> PathRef {
        for (r, p) in &self.paths {
            if p == path {
                return r;
            }
        }
        self.paths.push(path.into())
    }

    pub fn path(&self, r: PathRef) -> &Utf8PathBuf {
        &self.paths[r]
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Step> {
        self.steps.iter()
    }

    pub fn set_path(&mut self, path_ref: PathRef, path: Utf8PathBuf) {
        self.paths[path_ref] = path;
    }

    pub fn extend_inputs_buf(&mut self, path: &[Utf8PathBuf]) {
        let buf: Vec<PathRef> = path.iter().map(|f| self.path_ref(f)).collect();
        self.inputs.extend(buf);
    }

    pub fn extend_outputs_buf(&mut self, path: &[Utf8PathBuf]) {
        let buf: Vec<PathRef> = path.iter().map(|f| self.path_ref(f)).collect();
        self.outputs.extend(buf);
    }

    pub fn extend_stdins_buf(&mut self, path: &[Utf8PathBuf]) {
        let buf: Vec<PathRef> = path.iter().map(|f| self.path_ref(f)).collect();
        self.stdins.extend(buf);
    }

    pub fn extend_stdouts_buf(&mut self, path: &[Utf8PathBuf]) {
        let buf: Vec<PathRef> = path.iter().map(|f| self.path_ref(f)).collect();
        self.stdouts.extend(buf);
    }

    pub fn push_input(&mut self, path: PathRef) {
        self.inputs.push(path);
    }

    pub fn push_output(&mut self, path: PathRef) {
        self.outputs.push(path);
    }

    pub fn push_stdin(&mut self, path: PathRef) {
        self.stdins.push(path);
    }

    pub fn push_stdout(&mut self, path: PathRef) {
        self.stdouts.push(path);
    }

    pub fn to_path_buf(
        &self,
        buf: &[PathRef],
    ) -> impl Iterator<Item = &Utf8PathBuf> {
        buf.iter().map(|&f| self.path(f))
    }

    pub fn to_path_buf_vec(&self, buf: &[PathRef]) -> Vec<Utf8PathBuf> {
        self.to_path_buf(buf).cloned().collect()
    }

    pub fn inputs(&self) -> &[PathRef] {
        &self.inputs
    }

    pub fn inputs_buf_vec(&self) -> Vec<Utf8PathBuf> {
        self.to_path_buf_vec(self.inputs())
    }

    pub fn outputs(&self) -> &[PathRef] {
        &self.outputs
    }

    pub fn outputs_buf(&self) -> impl Iterator<Item = &Utf8PathBuf> {
        self.to_path_buf(self.outputs())
    }

    pub fn outputs_buf_vec(&self) -> Vec<Utf8PathBuf> {
        self.to_path_buf_vec(self.outputs())
    }

    pub fn stdins(&self) -> &[PathRef] {
        &self.stdins
    }

    pub fn stdins_buf(&self) -> impl Iterator<Item = &Utf8PathBuf> {
        self.to_path_buf(self.stdins())
    }

    pub fn stdins_buf_vec(&self) -> Vec<Utf8PathBuf> {
        self.to_path_buf_vec(self.stdins())
    }

    pub fn stdouts(&self) -> &[PathRef] {
        &self.stdouts
    }

    pub fn stdouts_buf(&self) -> impl Iterator<Item = &Utf8PathBuf> {
        self.to_path_buf(self.stdouts())
    }

    pub fn stdouts_buf_vec(&self) -> Vec<Utf8PathBuf> {
        self.to_path_buf_vec(self.stdouts())
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
