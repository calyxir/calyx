use crate::ast::{Control, Namespace, Seq};
use crate::passes::visitor::{Visitable, Visitor};

#[derive(Debug)]
pub struct Count {}

impl Visitor<()> for Count {
    // we want to do this on the way back up so that
    // we collapse multiple layers of nested seqs.
    fn finish_seq(
        &mut self,
        con: &mut Seq,
        _res: Result<(), ()>,
    ) -> Result<(), ()> {
        match con.stmts.as_slice() {
            [Control::Seq { data }] => con.stmts = data.stmts.clone(),
            _ => (),
        }

        Ok(())
    }
}

/** Collapses nested Seqs, i.e. (seq (seq (seq ...)))
becomes (seq ...) because all the other seqs are redundant because
they create a time scope with only step. */
pub fn do_pass(n: &mut Namespace) -> Count {
    let mut count = Count {};
    let _ = n.visit(&mut count);
    println!("{:?}", n);
    count
}
