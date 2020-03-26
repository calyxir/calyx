use crate::lang::component::Component;
use crate::lang::{ast, context::Context};
use crate::passes::visitor::{Action, VisResult, Visitor};

/// Pass that collapses `(par (enable A B) (enable C D))`
/// into `(enable A B C D)`
#[derive(Default)]
pub struct RedundantPar {}

impl Visitor for RedundantPar {
    fn name(&self) -> String {
        "remove redudant par".to_string()
    }

    // use finish_par so that we collapse things on the way
    // back up the tree and potentially catch more cases
    fn finish_par(
        &mut self,
        s: &mut ast::Par,
        _: &mut Component,
        _: &Context,
    ) -> VisResult {
        // If any of the ast nodes was not an enable, returns an Err.
        let enabled: Result<Vec<&ast::Enable>, ()> = s
            .stmts
            .iter()
            .map(|control| match control {
                ast::Control::Enable { data } => Ok(data),
                _ => Err(()),
            })
            .collect();

        Ok(enabled.map_or(Action::Continue, |ens| {
            let to_en = ens
                .into_iter()
                .map(|en| en.comps.clone())
                .flatten()
                .collect::<Vec<_>>();

            Action::Change(ast::Control::Enable {
                data: ast::Enable { comps: to_en },
            })
        }))
    }
}
