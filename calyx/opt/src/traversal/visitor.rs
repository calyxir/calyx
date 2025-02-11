//! Implements a visitor for `ir::Control` programs.
//! Program passes implemented as the Visitor are directly invoked on
//! [`ir::Context`] to compile every [`ir::Component`] using the pass.
use super::action::{Action, VisResult};
use super::{CompTraversal, ConstructVisitor, Named, Order};
use calyx_ir::{
    self as ir, Component, Context, Control, LibrarySignatures, StaticControl,
};
use calyx_utils::CalyxResult;
use std::rc::Rc;

/// The visiting interface for a [`ir::Control`](crate::Control) program.
/// Contains two kinds of functions:
/// 1. start_<node>: Called when visiting <node> top-down.
/// 2. finish_<node>: Called when visiting <node> bottow-up.
///
/// A pass will usually override one or more function and rely on the default
/// visitors to automatically visit the children.
pub trait Visitor {
    /// Precondition for this pass to run on the program. If this function returns
    /// None, the pass triggers. Otherwise it aborts and logs the string as the reason.
    fn precondition(_ctx: &ir::Context) -> Option<String>
    where
        Self: Sized,
    {
        None
    }

    #[inline(always)]
    /// Transform the [`ir::Context`] before visiting the components.
    fn start_context(&mut self, _ctx: &mut ir::Context) -> VisResult {
        Ok(Action::Continue)
    }

    #[inline(always)]
    /// Transform the [`ir::Context`] after visiting the components.
    fn finish_context(&mut self, _ctx: &mut ir::Context) -> VisResult {
        Ok(Action::Continue)
    }

    /// Define the iteration order in which components should be visited
    #[inline(always)]
    fn iteration_order() -> Order
    where
        Self: Sized,
    {
        Order::No
    }

    /// Define the traversal over a component.
    /// Calls [Visitor::start], visits each control node, and finally calls
    /// [Visitor::finish].
    fn traverse_component(
        &mut self,
        comp: &mut ir::Component,
        signatures: &LibrarySignatures,
        components: &[Component],
    ) -> CalyxResult<()>
    where
        Self: Sized,
    {
        self.start(comp, signatures, components)?
            .and_then(|| {
                // Create a clone of the reference to the Control
                // program.
                let control_ref = Rc::clone(&comp.control);
                if let Control::Empty(_) = &*control_ref.borrow() {
                    // Don't traverse if the control program is empty.
                    return Ok(Action::Continue);
                }
                // Mutably borrow the control program and traverse.
                control_ref
                    .borrow_mut()
                    .visit(self, comp, signatures, components)?;
                Ok(Action::Continue)
            })?
            .and_then(|| self.finish(comp, signatures, components))?
            .apply_change(&mut comp.control.borrow_mut());
        Ok(())
    }

    /// Run the visitor on a given program [`ir::Context`].
    /// The function mutably borrows the `control` program in each component and
    /// traverses it.
    ///
    /// After visiting a component, it called [ConstructVisitor::clear_data] to
    /// reset the struct.
    ///
    /// # Panics
    /// Panics if the pass attempts to use the control program mutably.
    fn do_pass(&mut self, context: &mut Context) -> CalyxResult<()>
    where
        Self: Sized + ConstructVisitor + Named,
    {
        if let Some(msg) = Self::precondition(&*context) {
            log::info!("Skipping `{}': {msg}", Self::name());
            return Ok(());
        }

        self.start_context(context)?;

        let signatures = &context.lib;
        let comps = std::mem::take(&mut context.components);

        // Temporarily take ownership of components from context.
        let mut po = CompTraversal::new(comps, Self::iteration_order());
        po.apply_update(|comp, comps| {
            self.traverse_component(comp, signatures, comps)?;
            self.clear_data();
            Ok(())
        })?;
        context.components = po.take();

        self.finish_context(context)?;

        Ok(())
    }

    /// Build a [Default] implementation of this pass and call [Visitor::do_pass]
    /// using it.
    #[inline(always)]
    fn do_pass_default(context: &mut Context) -> CalyxResult<Self>
    where
        Self: ConstructVisitor + Sized + Named,
    {
        let mut visitor = Self::from(&*context)?;
        visitor.do_pass(context)?;
        Ok(visitor)
    }

    /// Executed before the traversal begins.
    fn start(
        &mut self,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after the traversal ends.
    /// This method is always invoked regardless of the [Action] returned from
    /// the children.
    fn finish(
        &mut self,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::Seq] node.
    fn start_seq(
        &mut self,
        _s: &mut ir::Seq,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::Seq] node.
    fn finish_seq(
        &mut self,
        _s: &mut ir::Seq,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::Par] node.
    fn start_par(
        &mut self,
        _s: &mut ir::Par,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::Par] node.
    fn finish_par(
        &mut self,
        _s: &mut ir::Par,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::If] node.
    fn start_if(
        &mut self,
        _s: &mut ir::If,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::If] node.
    fn finish_if(
        &mut self,
        _s: &mut ir::If,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::While] node.
    fn start_while(
        &mut self,
        _s: &mut ir::While,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::While] node.
    fn finish_while(
        &mut self,
        _s: &mut ir::While,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::Repeat] node.
    fn start_repeat(
        &mut self,
        _s: &mut ir::Repeat,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::Repeat] node.
    fn finish_repeat(
        &mut self,
        _s: &mut ir::Repeat,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the contents of an [ir::StaticControl] node.
    fn start_static_control(
        &mut self,
        _s: &mut ir::StaticControl,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the conetnts of an [ir::StaticControl] node.
    fn finish_static_control(
        &mut self,
        _s: &mut ir::StaticControl,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed at an [ir::Enable] node.
    fn enable(
        &mut self,
        _s: &mut ir::Enable,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed at an [ir::StaticEnable] node.
    fn static_enable(
        &mut self,
        _s: &mut ir::StaticEnable,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::StaticIf] node.
    fn start_static_if(
        &mut self,
        _s: &mut ir::StaticIf,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::StaticIf] node.
    fn finish_static_if(
        &mut self,
        _s: &mut ir::StaticIf,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::StaticRepeat] node.
    fn start_static_repeat(
        &mut self,
        _s: &mut ir::StaticRepeat,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::StaticRepeat] node.
    fn finish_static_repeat(
        &mut self,
        _s: &mut ir::StaticRepeat,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    // Executed before visiting the children of a [ir::StaticSeq] node.
    fn start_static_seq(
        &mut self,
        _s: &mut ir::StaticSeq,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    // Executed after visiting the children of a [ir::StaticSeq] node.
    fn finish_static_seq(
        &mut self,
        _s: &mut ir::StaticSeq,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    // Executed before visiting the children of a [ir::StaticPar] node.
    fn start_static_par(
        &mut self,
        _s: &mut ir::StaticPar,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    // Executed after visiting the children of a [ir::StaticPar] node.
    fn finish_static_par(
        &mut self,
        _s: &mut ir::StaticPar,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed at an [ir::Invoke] node.
    fn invoke(
        &mut self,
        _s: &mut ir::Invoke,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed at a [ir::StaticInvoke] node.
    fn static_invoke(
        &mut self,
        _s: &mut ir::StaticInvoke,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed at an [ir::Empty] node.
    fn empty(
        &mut self,
        _s: &mut ir::Empty,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }
}

/// Describes types that can be visited by things implementing [Visitor].
/// This performs a recursive walk of the tree.
///
/// It calls `Visitor::start_*` on the way down, and `Visitor::finish_*` on
/// the way up.
pub trait Visitable {
    /// Perform the traversal.
    fn visit(
        &mut self,
        visitor: &mut dyn Visitor,
        component: &mut Component,
        signatures: &LibrarySignatures,
        components: &[ir::Component],
    ) -> VisResult;
}

impl Visitable for Control {
    fn visit(
        &mut self,
        visitor: &mut dyn Visitor,
        component: &mut Component,
        sigs: &LibrarySignatures,
        comps: &[ir::Component],
    ) -> VisResult {
        let res = match self {
            Control::Seq(ctrl) => visitor
                .start_seq(ctrl, component, sigs, comps)?
                .and_then(|| ctrl.stmts.visit(visitor, component, sigs, comps))?
                .pop()
                .and_then(|| {
                    visitor.finish_seq(ctrl, component, sigs, comps)
                })?,
            Control::Par(ctrl) => visitor
                .start_par(ctrl, component, sigs, comps)?
                .and_then(|| ctrl.stmts.visit(visitor, component, sigs, comps))?
                .pop()
                .and_then(|| {
                    visitor.finish_par(ctrl, component, sigs, comps)
                })?,
            Control::If(ctrl) => visitor
                .start_if(ctrl, component, sigs, comps)?
                .and_then(|| {
                    ctrl.tbranch.visit(visitor, component, sigs, comps)
                })?
                .and_then(|| {
                    ctrl.fbranch.visit(visitor, component, sigs, comps)
                })?
                .pop()
                .and_then(|| visitor.finish_if(ctrl, component, sigs, comps))?,
            Control::While(ctrl) => visitor
                .start_while(ctrl, component, sigs, comps)?
                .and_then(|| ctrl.body.visit(visitor, component, sigs, comps))?
                .pop()
                .and_then(|| {
                    visitor.finish_while(ctrl, component, sigs, comps)
                })?,
            Control::Repeat(ctrl) => visitor
                .start_repeat(ctrl, component, sigs, comps)?
                .and_then(|| ctrl.body.visit(visitor, component, sigs, comps))?
                .pop()
                .and_then(|| {
                    visitor.finish_repeat(ctrl, component, sigs, comps)
                })?,
            Control::Enable(ctrl) => {
                visitor.enable(ctrl, component, sigs, comps)?
            }
            Control::Static(sctrl) => visitor
                .start_static_control(sctrl, component, sigs, comps)?
                .and_then(|| sctrl.visit(visitor, component, sigs, comps))?
                .pop()
                .and_then(|| {
                    visitor.finish_static_control(sctrl, component, sigs, comps)
                })?,
            Control::Empty(ctrl) => {
                visitor.empty(ctrl, component, sigs, comps)?
            }
            Control::Invoke(data) => {
                visitor.invoke(data, component, sigs, comps)?
            }
        };
        Ok(res.apply_change(self))
    }
}

impl Visitable for StaticControl {
    fn visit(
        &mut self,
        visitor: &mut dyn Visitor,
        component: &mut Component,
        signatures: &LibrarySignatures,
        components: &[ir::Component],
    ) -> VisResult {
        let res = match self {
            StaticControl::Empty(ctrl) => {
                visitor.empty(ctrl, component, signatures, components)?
            }
            StaticControl::Enable(ctrl) => visitor
                .static_enable(ctrl, component, signatures, components)?,
            StaticControl::Repeat(ctrl) => visitor
                .start_static_repeat(ctrl, component, signatures, components)?
                .and_then(|| {
                    ctrl.body.visit(visitor, component, signatures, components)
                })?
                .pop()
                .and_then(|| {
                    visitor.finish_static_repeat(
                        ctrl, component, signatures, components,
                    )
                })?,
            StaticControl::Seq(ctrl) => visitor
                .start_static_seq(ctrl, component, signatures, components)?
                .and_then(|| {
                    ctrl.stmts.visit(visitor, component, signatures, components)
                })?
                .pop()
                .and_then(|| {
                    visitor.finish_static_seq(
                        ctrl, component, signatures, components,
                    )
                })?,
            StaticControl::Par(ctrl) => visitor
                .start_static_par(ctrl, component, signatures, components)?
                .and_then(|| {
                    ctrl.stmts.visit(visitor, component, signatures, components)
                })?
                .pop()
                .and_then(|| {
                    visitor.finish_static_par(
                        ctrl, component, signatures, components,
                    )
                })?,
            StaticControl::If(sctrl) => visitor
                .start_static_if(sctrl, component, signatures, components)?
                .and_then(|| {
                    sctrl
                        .tbranch
                        .visit(visitor, component, signatures, components)
                })?
                .and_then(|| {
                    sctrl
                        .fbranch
                        .visit(visitor, component, signatures, components)
                })?
                .pop()
                .and_then(|| {
                    visitor.finish_static_if(
                        sctrl, component, signatures, components,
                    )
                })?,
            StaticControl::Invoke(sin) => {
                visitor.static_invoke(sin, component, signatures, components)?
            }
        };
        Ok(res.apply_static_change(self))
    }
}

/// Blanket implementation for Vectors of Visitables
impl<V: Visitable> Visitable for Vec<V> {
    fn visit(
        &mut self,
        visitor: &mut dyn Visitor,
        component: &mut Component,
        sigs: &LibrarySignatures,
        components: &[ir::Component],
    ) -> VisResult {
        for t in self {
            let res = t.visit(visitor, component, sigs, components)?;
            match res {
                Action::Continue
                | Action::SkipChildren
                | Action::Change(_)
                | Action::StaticChange(_) => {
                    continue;
                }
                Action::Stop => return Ok(Action::Stop),
            };
        }
        Ok(Action::Continue)
    }
}
