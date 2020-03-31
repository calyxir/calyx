use crate::backend::traits::{Backend, Emitable};
use crate::errors;
use crate::lang::pretty_print::display;
use crate::lang::{ast, ast::Control, component, context};
use bumpalo::Bump;
use pretty::RcDoc as D;
use std::io::Write;

/// Implements a simple Verilog backend. The backend
/// only accepts Futil programs with control that is
/// a top level `seq` with only `enable`s as children:
/// ```
/// (seq (enable A B)
///      (enable B C)
///       ...)
/// ```
/// or control that is just a single `enable`.
/// ```
/// (enable A B)
/// ```
pub struct VerilogBackend {}

impl Backend for VerilogBackend {
    fn name() -> &'static str {
        "verilog"
    }

    fn validate(ctx: &context::Context) -> Result<(), errors::Error> {
        let prog: ast::NamespaceDef = ctx.clone().into();
        for comp in &prog.components {
            match &comp.control {
                // Control::Seq { data } => {
                //     for con in &data.stmts {
                //         match con {
                //             Control::Enable { .. } => (),
                //             _ => return Err(errors::Error::MalformedControl),
                //         }
                //     }
                // }
                Control::Enable { .. } | Control::Empty { .. } => (),
                _ => return Err(errors::Error::MalformedControl),
            }
        }
        Ok(())
    }

    fn emit<W: Write>(
        ctx: &context::Context,
        file: W,
    ) -> Result<(), errors::Error> {
        let prog: ast::NamespaceDef = ctx.clone().into();

        // build Vec of tuples first so that `comps` lifetime is longer than
        // `docs` lifetime
        let comps: Vec<(&ast::ComponentDef, component::Component)> = prog
            .components
            .iter()
            .map(|cd| (cd, ctx.get_component(&cd.name).unwrap()))
            .collect();

        let mut arena = Bump::new();
        let docs = comps.iter().map(|(cd, comp)| cd.doc(&arena, &comp));
        display(
            D::intersperse(docs, D::line().append(D::line())),
            Some(file),
        );
        arena.reset();
        Ok(())
    }
}
