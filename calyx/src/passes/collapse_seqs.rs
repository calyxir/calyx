use crate::lang::ast::{Control, Seq};
use crate::passes::visitor::{Changes, Visitor};

/** Collapses nested Seqs, i.e. (seq (seq (seq ...)))
becomes (seq ...) because all the other seqs are redundant because
they create a time scope with only step. */
#[derive(Debug)]
pub struct Count {}

impl Visitor<()> for Count {
    fn new() -> Self {
        Count {}
    }

    fn name(&self) -> String {
        "Collapse Seqs".to_string()
    }

    // we want to do this on the way back up so that
    // we collapse multiple layers of nested seqs.
    fn finish_seq(
        &mut self,
        con: &mut Seq,
        _changes: &mut Changes,
        _res: Result<(), ()>,
    ) -> Result<(), ()> {
        if let [Control::Seq { data }] = con.stmts.as_slice() {
            con.stmts = data.stmts.clone()
        }

        Ok(())
    }
}
