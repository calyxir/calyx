use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir, GetAttributes, LibrarySignatures};

const NODE_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::NODE_ID);
const BEGIN_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::BEGIN_ID);
const END_ID: ir::Attribute = ir::Attribute::Internal(ir::InternalAttr::END_ID);

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

fn remove_ids_static(sc: &mut ir::StaticControl) {
    let atts = sc.get_mut_attributes();
    atts.remove(BEGIN_ID);
    atts.remove(END_ID);
    atts.remove(NODE_ID);
    match sc {
        ir::StaticControl::Empty(_) | ir::StaticControl::Invoke(_) => (),
        ir::StaticControl::Enable(ir::StaticEnable { group, .. }) => {
            group.borrow_mut().remove_attribute(NODE_ID)
        }
        ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
            remove_ids_static(body)
        }
        ir::StaticControl::Seq(ir::StaticSeq { stmts, .. })
        | ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
            for stmt in stmts {
                remove_ids_static(stmt);
            }
        }
        ir::StaticControl::If(ir::StaticIf {
            tbranch, fbranch, ..
        }) => {
            remove_ids_static(tbranch);
            remove_ids_static(fbranch);
        }
    }
}

fn remove_ids(c: &mut ir::Control) {
    let atts = c.get_mut_attributes();
    atts.remove(BEGIN_ID);
    atts.remove(END_ID);
    atts.remove(NODE_ID);
    match c {
        ir::Control::Empty(_) | ir::Control::Invoke(_) => (),
        ir::Control::Enable(ir::Enable { group, .. }) => {
            group.borrow_mut().remove_attribute(NODE_ID)
        }
        ir::Control::While(ir::While { body, .. })
        | ir::Control::Repeat(ir::Repeat { body, .. }) => {
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
        ir::Control::Static(sc) => remove_ids_static(sc),
    }
}
