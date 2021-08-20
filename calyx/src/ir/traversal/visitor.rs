//! Implements a visitor for `ir::Control` programs.
//! Program passes implemented as the Visitor are directly invoked on
//! [`ir::Context`] to compile every [`ir::Component`] using the pass.
use super::action::{Action, VisResult};
use super::PostOrder;
use crate::errors::CalyxResult;
use crate::ir::{self, Component, Context, Control, LibrarySignatures};
use std::rc::Rc;

/// Trait that describes named things. Calling [`do_pass`](Visitor::do_pass) and [`do_pass_default`](Visitor::do_pass_default).
/// require this to be implemented.
///
/// This has to be a separate trait from [`Visitor`] because these methods don't recieve `self` which
/// means that it is impossible to create dynamic trait objects.
pub trait Named {
    /// The name of a pass. Is used for identifying passes.
    fn name() -> &'static str;

    /// A short description of the pass.
    fn description() -> &'static str;
}

/// Implementator of trait provide various logging methods.
pub trait Loggable {
    /// Log output to STDERR.
    /// `context` is the location from which the logger is being called.
    /// Usage:
    /// ```
    /// self.elog("number-of-groups", groups.len());
    /// ```
    fn elog<S, T>(&self, context: S, msg: T)
    where
        S: std::fmt::Display,
        T: std::fmt::Display;
}

/// Blanket implementation for Loggable for traits implementing Named
impl<T> Loggable for T
where
    T: Named,
{
    fn elog<S, M>(&self, context: S, msg: M)
    where
        S: std::fmt::Display,
        M: std::fmt::Display,
    {
        eprintln!("{}.{}: {}", T::name(), context, msg)
    }
}

/// Trait defining method that can be used to construct a Visitor from an
/// [ir::Context].
/// This is useful when a pass needs to construct information using the context
/// *before* visiting the components.
///
/// For passes that don't need to use the context, this trait can be automatically
/// be derived from [Default].
pub trait ConstructVisitor {
    /// Construct the visitor using information from the Context
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized;

    /// Clear the data stored in the visitor. Called before traversing the
    /// next component by [ir::traversal::Visitor].
    fn clear_data(&mut self);
}

/// Derive ConstructVisitor when [Default] is provided for a visitor.
impl<T: Default + Sized + Visitor> ConstructVisitor for T {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self> {
        Ok(T::default())
    }

    fn clear_data(&mut self) {
        *self = T::default();
    }
}

/// The visiting interface for a [`ir::Control`](crate::ir::Control) program.
/// Contains two kinds of functions:
/// 1. start_<node>: Called when visiting <node> top-down.
/// 2. finish_<node>: Called when visiting <node> bottow-up.
///
/// A pass will usually override one or more function and rely on the default
/// visitors to automatically visit the children.
pub trait Visitor {
    /// Returns true if this pass requires a post-order traversal of the
    /// components.
    /// In a post-order traversal, if component `B` uses a component `A`,
    /// then `A` is guaranteed to be traversed before `B`.
    #[inline(always)]
    fn require_postorder() -> bool
    where
        Self: Sized,
    {
        false
    }

    /// Define the traversal over a component.
    /// Calls [Visitor::start], visits each control node, and finally calls
    /// [Visitor::finish].
    fn traverse_component(
        &mut self,
        comp: &mut ir::Component,
        signatures: &LibrarySignatures,
    ) -> CalyxResult<()>
    where
        Self: Sized,
    {
        self.start(comp, signatures)?
            .and_then(|| {
                // Create a clone of the reference to the Control
                // program.
                let control_ref = Rc::clone(&comp.control);
                if let Control::Empty(_) = &*control_ref.borrow() {
                    // Don't traverse if the control program is empty.
                    return Ok(Action::Continue);
                }
                // Mutably borrow the control program and traverse.
                control_ref.borrow_mut().visit(self, comp, signatures)?;
                Ok(Action::Continue)
            })?
            .and_then(|| self.finish(comp, signatures))?
            .apply_change(&mut comp.control.borrow_mut());
        Ok(())
    }

    /// Run the visitor on a given program [`ir::Context`](crate::ir::Context).
    /// The function mutably borrows the [`control`](crate::ir::Component::control)
    /// program in each component and traverses it.
    ///
    /// After visiting a component, it called [ConstructVisitor::clear_data] to
    /// reset the struct.
    ///
    /// # Panics
    /// Panics if the pass attempts to use the control program mutably.
    fn do_pass(&mut self, context: &mut Context) -> CalyxResult<()>
    where
        Self: Sized + ConstructVisitor,
    {
        let signatures = &context.lib;

        if Self::require_postorder() {
            // Temporarily take ownership of components from context.
            let comps = context.components.drain(..).collect();
            let mut po = PostOrder::new(comps);
            po.apply_update(|comp| {
                self.traverse_component(comp, signatures)?;
                self.clear_data();
                Ok(())
            })?;
            context.components = po.take();
        } else {
            context
                .components
                // Mutably borrow the components in the context
                .iter_mut()
                .try_for_each(|comp| {
                    self.traverse_component(comp, signatures)?;
                    self.clear_data();
                    Ok(()) as CalyxResult<_>
                })?;
        }

        Ok(())
    }

    /// Build a [Default] implementation of this pass and call [Visitor::do_pass]
    /// using it.
    #[inline(always)]
    fn do_pass_default(context: &mut Context) -> CalyxResult<Self>
    where
        Self: ConstructVisitor + Sized,
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
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::Seq] node.
    fn start_seq(
        &mut self,
        _s: &mut ir::Seq,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::Seq] node.
    fn finish_seq(
        &mut self,
        _s: &mut ir::Seq,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::Par] node.
    fn start_par(
        &mut self,
        _s: &mut ir::Par,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::Par] node.
    fn finish_par(
        &mut self,
        _s: &mut ir::Par,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::If] node.
    fn start_if(
        &mut self,
        _s: &mut ir::If,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::If] node.
    fn finish_if(
        &mut self,
        _s: &mut ir::If,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::While] node.
    fn start_while(
        &mut self,
        _s: &mut ir::While,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::While] node.
    fn finish_while(
        &mut self,
        _s: &mut ir::While,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed at an [ir::Enable] node.
    fn enable(
        &mut self,
        _s: &mut ir::Enable,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed at an [ir::Invoke] node.
    fn invoke(
        &mut self,
        _s: &mut ir::Invoke,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed at an [ir::Empty] node.
    fn empty(
        &mut self,
        _s: &mut ir::Empty,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
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
    ) -> VisResult;
}

impl Visitable for Control {
    fn visit(
        &mut self,
        visitor: &mut dyn Visitor,
        component: &mut Component,
        sigs: &LibrarySignatures,
    ) -> VisResult {
        let res = match self {
            Control::Seq(ctrl) => visitor
                .start_seq(ctrl, component, sigs)?
                .and_then(|| ctrl.stmts.visit(visitor, component, sigs))?
                .pop()
                .and_then(|| visitor.finish_seq(ctrl, component, sigs))?,
            Control::Par(ctrl) => visitor
                .start_par(ctrl, component, sigs)?
                .and_then(|| ctrl.stmts.visit(visitor, component, sigs))?
                .pop()
                .and_then(|| visitor.finish_par(ctrl, component, sigs))?,
            Control::If(ctrl) => visitor
                .start_if(ctrl, component, sigs)?
                .and_then(|| ctrl.tbranch.visit(visitor, component, sigs))?
                .and_then(|| ctrl.fbranch.visit(visitor, component, sigs))?
                .pop()
                .and_then(|| visitor.finish_if(ctrl, component, sigs))?,
            Control::While(ctrl) => visitor
                .start_while(ctrl, component, sigs)?
                .and_then(|| ctrl.body.visit(visitor, component, sigs))?
                .pop()
                .and_then(|| visitor.finish_while(ctrl, component, sigs))?,
            Control::Enable(ctrl) => visitor.enable(ctrl, component, sigs)?,
            Control::Empty(ctrl) => visitor.empty(ctrl, component, sigs)?,
            Control::Invoke(data) => visitor.invoke(data, component, sigs)?,
        };
        Ok(res.apply_change(self))
    }
}

/// Blanket implementation for Vectors of Visitables
impl<V: Visitable> Visitable for Vec<V> {
    fn visit(
        &mut self,
        visitor: &mut dyn Visitor,
        component: &mut Component,
        sigs: &LibrarySignatures,
    ) -> VisResult {
        for t in self {
            let res = t.visit(visitor, component, sigs)?;
            match res {
                Action::Continue | Action::SkipChildren | Action::Change(_) => {
                    continue;
                }
                Action::Stop => return Ok(Action::Stop),
            };
        }
        Ok(Action::Continue)
    }
}
