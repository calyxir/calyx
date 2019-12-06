use crate::lang::ast::Namespace;
use crate::lang::pretty_print::PrettyPrint;
use crate::passes;
use crate::passes::visitor::Visitor;
use crate::utils;

pub fn generate(syntax: &mut Namespace) {
    passes::fsm_enable::FsmList::new().do_pass(syntax);
    let mut prev_hash = utils::calculate_hash(&syntax);
    loop {
        passes::fsm_seq::FsmSeq::new().do_pass(syntax);
        let hash = utils::calculate_hash(&syntax);
        if hash == prev_hash {
            break;
        }
        prev_hash = hash;
    }
}
