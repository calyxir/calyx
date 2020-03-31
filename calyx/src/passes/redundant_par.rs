use crate::lang::component::Component;
use crate::lang::{ast, context::Context};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};

/// Pass that collapses `(par (enable A B) (enable C D))`
/// into `(enable A B C D)`
#[derive(Default)]
pub struct RedundantPar {}

impl Named for RedundantPar {
    fn name() -> &'static str {
        "redudant-par"
    }

    fn description() -> &'static str {
        "removes redudant par statements"
    }
}

impl Visitor for RedundantPar {
    // use finish_par so that we collapse things on the way
    // back up the tree and potentially catch more cases
    fn finish_par(
        &mut self,
        s: &mut ast::Par,
        _comp: &mut Component,
        _c: &Context,
    ) -> VisResult {
        let mut enabled: Vec<ast::Id> = vec![];
        for con in &s.stmts {
            match con {
                ast::Control::Enable { data } => {
                    enabled.append(&mut data.comps.clone());
                }
                _ => return Ok(Action::Continue),
            }
        }
        let enable = ast::Enable { comps: enabled };
        Ok(Action::Change(ast::Control::Enable { data: enable }))
    }
}
