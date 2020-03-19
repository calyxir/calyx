use crate::context::Context;
use crate::lang::ast;
use crate::lang::component::Component;
use crate::passes::visitor::{Action, VisResult, Visitor};

/// Pass that collapses 
///(seq
///    (seq (enable A B)
///         (enable C D))
///    (seq (enable E F)
///         (enable G H))
/// ..)
/// into 
/// (seq (enable A B)
///      (enable C D)
///      (enable E F)
///      (enable G H)
///  ..)
#[derive(Default)]
pub struct CollapeSeq {}

impl Visitor for CollapeSeq {
    fn name(&self) -> String {
        "remove redudant seq".to_string()
    }

    // use finish_seq so that we collapse things on the way
    // back up the tree and potentially catch more cases
    fn finish_seq(
        &mut self,
        s: &mut ast::Seq,
        _comp: &mut Component,
        _c: &Context,
    ) -> VisResult {
        let mut seqs: Vec<ast::Control> = vec![];
        for con in &s.stmts {
            match con {
                ast::Control::Seq { data } => {
                    seqs.append(&mut data.stmts.clone());
                }
                _ => seqs.push(con.clone()),
            }
        }
        Ok(Action::Change(ast::Control::seq (seqs)))
    }
}
