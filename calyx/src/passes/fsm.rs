use crate::lang::context::Context;
use crate::passes;
use crate::passes::visitor::Visitor;
use crate::utils::{calculate_hash, NameGenerator};

pub fn generate(syntax: &mut Context, names: &mut NameGenerator) {
    passes::fsm_enable::FsmEnable::new().do_pass(syntax);
    // let mut prev_hash = calculate_hash(&syntax);
    loop {
        passes::fsm_if::FsmIf::new(names).do_pass(syntax);
        passes::fsm_ifen::FsmIfen::new(names).do_pass(syntax);
        passes::fsm_while::FsmWhile::new(names).do_pass(syntax);
        passes::fsm_seq::FsmSeq::new(names).do_pass(syntax);
        passes::fsm_par::FsmPar::new(names).do_pass(syntax);
        // let hash = calculate_hash(&syntax);
        // if hash == prev_hash {
        //     break;
        // }
        // prev_hash = hash;
    }
}
