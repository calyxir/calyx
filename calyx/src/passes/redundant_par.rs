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

        // If the result wasn't an Err, do the transformation.
        Ok(enabled
            .map(|ens| ast::Enable {
                comps: ens
                    .iter()
                    .cloned()
                    .flat_map(|en| en.comps.clone())
                    .collect(),
            })
            .map_or(Action::Continue, |en| {
                Action::Change(ast::Control::Enable { data: en })
            }))

    }
}
