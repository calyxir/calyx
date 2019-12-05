use crate::lang::ast::Namespace;
use crate::passes;
use crate::passes::visitor::Visitor;

pub fn generate(syntax: &mut Namespace) {
    passes::fsm_enable::FsmList::new().do_pass(syntax);
    passes::fsm_seq::FsmSeq::new().do_pass(syntax);
}
