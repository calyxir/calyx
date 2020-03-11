use crate::context::Context;
use crate::errors;
use crate::lang::ast;
use crate::lang::component::Component;
use crate::passes::visitor::{Action, VisResult, Visitor};

/// Pass that removes if statments where both branches are enables:
/// ```lisp
///    (if (@ cond port) (comps ...)
///        (enable A B ...)
///        (enable C B ...))
/// ```
/// It does this by connecting the condition port to all
/// the `valid` ports of the registers in the true branch
/// and connecting the inverse of the condition port to
/// the `valid` ports of the registers in the false branch
///
/// This pass currently does not support memories or other side
/// effecting components.
#[derive(Default)]
pub struct RemoveIf {}

impl Visitor for RemoveIf {
    fn name(&self) -> String {
        "remove simple if statements".to_string()
    }

    fn finish_if(
        &mut self,
        con: &mut ast::If,
        this_comp: &mut Component,
        ctx: &Context,
    ) -> VisResult {
        // get node and port for the comparison component
        use ast::Port;
        let (cmp_idx, cmp_port) = match &con.port {
            Port::Comp { component, port } => {
                (this_comp.get_inst_index(&component)?, port)
            }
            Port::This { port } => (this_comp.get_io_index(&port)?, port),
        };

        let add_structure_tbranch =
            |this_comp: &mut Component, en_comp: &ast::Id| {
                this_comp.add_wire(
                    cmp_idx,
                    &cmp_port,
                    this_comp.get_inst_index(en_comp)?,
                    "valid",
                )
            };

        let add_structure_fbranch =
            |this_comp: &mut Component, en_comp: &ast::Id| {
                // XXX(sam) randomly generate this name
                let name = format!("{}_not", en_comp);
                let neg_comp = ctx.instantiate_primitive(
                    &name,
                    &"std_not".to_string(), // XXX(sam) this is silly
                    &[1],
                )?;
                let neg =
                    this_comp.add_primitive(&name, "std_not", &neg_comp, &[1]);
                this_comp.add_wire(cmp_idx, &cmp_port, neg, "in")?;
                this_comp.add_wire(
                    neg,
                    "out",
                    this_comp.get_inst_index(en_comp)?,
                    "valid",
                )
            };

        use ast::Control::Enable;
        match (&*con.tbranch, &*con.fbranch) {
            (Enable { data: tbranch }, Enable { data: fbranch }) => {
                // if statement has the right form
                for en_comp in &tbranch.comps {
                    let sig = resolve_signature(this_comp, en_comp)?;
                    if sig.has_input("valid") {
                        add_structure_tbranch(this_comp, en_comp)?;
                    }
                }

                for en_comp in &fbranch.comps {
                    let sig = resolve_signature(this_comp, en_comp)?;
                    if sig.has_input("valid") {
                        add_structure_fbranch(this_comp, en_comp)?;
                    }
                }

                let mut comps = vec![];
                comps.append(&mut con.cond);
                comps.append(&mut tbranch.comps.clone());
                comps.append(&mut fbranch.comps.clone());

                Ok(Action::Change(ast::Control::enable(comps)))
            }
            _ => Ok(Action::Continue),
        }
    }
}

fn resolve_signature<'a>(
    this_comp: &'a mut Component,
    en_comp: &str,
) -> Result<&'a ast::Signature, errors::Error> {
    let sig = this_comp.resolved_sigs.get(en_comp);
    match sig {
        Some(sig) => Ok(sig),
        None => Err(errors::Error::UndefinedComponent(en_comp.to_string())),
    }
}
