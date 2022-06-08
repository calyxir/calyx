use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::GetAttributes;

#[derive(Default)]
/// Transforms a par of seq blocks into a seq of par blocks. For the
/// transformation to occur, each seq block must have the same number of statements,
/// and the nth statement of each seq block in the par block
/// must all have the same number of clock cycles.
///
/// #Example
/// par {
///    seq { @static(M) A0; @static(N) B0; @static(P) C0; }
///    seq { @static(M) A1; @static(N) B1; @static(P) C1; }
///    seq { @static(M) A2; @static(N) B2; @static(P) C2; }
///}
///
/// into
///
/// seq {
///     par{ @static(M) A0; @static(M) A1; @static(M) A2;}
///     par{ @static(N) B0; @static(N) B1; @static(N) B2;}
///     par{ @static(P) C0; @static(P) C1; @static(P) C2;}
/// }
///  
pub struct ParSeqToSeqPar;

impl Named for ParSeqToSeqPar {
    fn name() -> &'static str {
        "parseq-to-seqpar"
    }

    fn description() -> &'static str {
        "Transform `par` of `seq` to `seq` of `par` under correct conditions"
    }
}

impl Visitor for ParSeqToSeqPar {
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut new_seq = ir::Seq {
            stmts: Vec::new(),
            attributes: ir::Attributes::default(),
        };

        let mut transformation_ok = true;
        let mut all_seqs_iterated = false;
        let mut n = 0;

        while !all_seqs_iterated && transformation_ok {
            let mut new_par = ir::Par {
                stmts: Vec::new(),
                attributes: ir::Attributes::default(),
            };

            let mut each_seq_has_nth_stmt = true;
            let mut set_num_cycles = false;
            let mut num_cycles = 0;

            all_seqs_iterated = true;
            (*s.stmts).iter().for_each(|x| match x {
                ir::Control::Seq(seq) => {
                    match seq.stmts.get(n) {
                        Some(stmt) => {
                            all_seqs_iterated = false;
                            if let Some(&num_cycles_cur) = stmt
                                .get_attributes()
                                .and_then(|atts| atts.get("static"))
                            {
                                if !set_num_cycles {
                                    num_cycles = num_cycles_cur;
                                    set_num_cycles = true;
                                }
                                if num_cycles_cur == num_cycles {
                                    new_par
                                        .stmts
                                        .push(ir::Control::clone(stmt));
                                } else {
                                    //don't have same number of cycles
                                    transformation_ok = false;
                                }
                            } else {
                                //no static attribute for nth statement in seq
                                transformation_ok = false;
                            }
                        }
                        None => {
                            each_seq_has_nth_stmt = false;
                        }
                    }
                }
                //not all statements in the par are seqs
                _ => transformation_ok = false,
            });
            if !each_seq_has_nth_stmt && !all_seqs_iterated {
                //seqs are not all same length
                transformation_ok = false;
            }
            if !all_seqs_iterated && transformation_ok {
                new_seq.stmts.push(ir::Control::Par(new_par));
            }
            n += 1;
        }

        if transformation_ok {
            Ok(Action::Change(Box::new(ir::Control::Seq(new_seq))))
        } else {
            Ok(Action::Continue)
        }
    }
}
