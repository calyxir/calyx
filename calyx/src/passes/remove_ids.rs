use crate::ir::GetAttributes;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures,
};

const NODE_ID: &str = "NODE_ID";
const BEGIN_ID: &str = "BEGIN_ID";
const END_ID: &str = "END_ID";

#[derive(Default)]
/// Removes NODE_ID, BEGIN_ID, and END_ID from each control statement
pub struct RemoveIds;

impl Named for RemoveIds {
    fn name() -> &'static str {
        "remove-ids"
    }

    fn description() -> &'static str {
        "removes the NODE_ID, BEGIN_ID, and END_ID from the control flow"
    }
}

impl Visitor for RemoveIds {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        remove_ids(&mut comp.control.borrow_mut());
        Ok(Action::Stop)
    }
}

fn remove_ids(c: &mut ir::Control) {
    if let Some(atts) = c.get_mut_attributes() {
        atts.remove(BEGIN_ID);
        atts.remove(END_ID);
        atts.remove(NODE_ID);
    }
    match c {
        ir::Control::Empty(_)
        | ir::Control::Invoke(_)
        | ir::Control::Enable(_) => (),
        ir::Control::While(ir::While { body, .. }) => {
            remove_ids(body);
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            remove_ids(tbranch);
            remove_ids(fbranch);
        }
        ir::Control::Seq(ir::Seq { stmts, .. })
        | ir::Control::Par(ir::Par { stmts, .. }) => {
            for stmt in stmts {
                remove_ids(stmt);
            }
        }
    }
}
