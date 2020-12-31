//! Implements a visitor for `ir::Control` programs.
//! Program passes implemented as the Visitor are directly invoked on
//! `ir::Context` to compile every `ir::Component` using the pass.
use super::action::{Action, VisResult};
use crate::errors::FutilResult;
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

/// The visiting interface for a [`ir::Control`](crate::ir::Control) program.
/// Contains two kinds of functions:
/// 1. start_<node>: Called when visiting <node> top-down.
/// 2. finish_<node>: Called when visiting <node> bottow-up.
///
/// A pass will usually override one or more function and rely on the default
/// visitors to automatically visit the children.
pub trait Visitor {
    /// Build a [Default] implementation of this pass and call [`Visitor::do_pass`]
    /// using it.
    fn do_pass_default(context: &mut Context) -> FutilResult<Self>
    where
        Self: Default + Sized + Named,
    {
        let mut visitor = Self::default();
        visitor.do_pass(context)?;
        Ok(visitor)
    }

    /// Run the visitor on a given program [`ir::Context`](crate::ir::Context).
    /// The function mutably borrows the [`control`](crate::ir::Component::control)
    /// program in each component and traverses it.
    ///
    /// # Panics
    /// Panics if the pass attempts to use the control program mutably.
    fn do_pass(&mut self, context: &mut Context) -> FutilResult<()>
    where
        Self: Sized + Named,
    {
        let signatures = &context.lib;
        context
            .components
            // Mutably borrow the components in the context
            .iter_mut()
            .map(|mut comp| {
                self.start(&mut comp, signatures)?
                    .and_then(|| {
                        // Create a clone of the reference to the Control
                        // program.
                        let control_ref = Rc::clone(&comp.control);
                        // Borrow the control program mutably and visit it.
                        control_ref
                            .borrow_mut()
                            .visit(self, &mut comp, signatures)?;
                        Ok(Action::Continue)
                    })?
                    .and_then(|| self.finish(&mut comp, signatures))?
                    .apply_change(&mut comp.control.borrow_mut())?;
                Ok(())
            })
            .collect::<FutilResult<_>>()?;

        Ok(())
    }

    /// Executed before the traversal begins.
    fn start(
        &mut self,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Exceuted after the traversal ends.
    /// This method is always invoked regardless of the [`Action`] returned from
    /// the children.
    fn finish(
        &mut self,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted before visiting the children of a [`ir::Seq`](crate::ir::Seq) node.
    fn start_seq(
        &mut self,
        _s: &mut ir::Seq,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted after visiting the children of a [`ir::Seq`](crate::ir::Seq) node.
    fn finish_seq(
        &mut self,
        _s: &mut ir::Seq,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted before visiting the children of a [`ir::Par`](crate::ir::Par) node.
    fn start_par(
        &mut self,
        _s: &mut ir::Par,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted after visiting the children of a [`ir::Par`](crate::ir::Par) node.
    fn finish_par(
        &mut self,
        _s: &mut ir::Par,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted before visiting the children of a [`ir::If`](crate::ir::If) node.
    fn start_if(
        &mut self,
        _s: &mut ir::If,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted after visiting the children of a [`ir::If`](crate::ir::If) node.
    fn finish_if(
        &mut self,
        _s: &mut ir::If,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted before visiting the children of a [`ir::While`](crate::ir::While) node.
    fn start_while(
        &mut self,
        _s: &mut ir::While,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted after visiting the children of a [`ir::While`](crate::ir::While) node.
    fn finish_while(
        &mut self,
        _s: &mut ir::While,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted at an [`ir::Enable`](crate::ir::Enable) node.
    fn enable(
        &mut self,
        _s: &mut ir::Enable,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted at an [`ir::Invoke`](crate::ir::Invoke) node.
    fn invoke(
        &mut self,
        _s: &mut ir::Invoke,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted at an [`ir::Empty`](crate::ir::Invoke) node.
    fn empty(
        &mut self,
        _s: &mut ir::Empty,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }
}

/// Describes types that can be visited by things implementing [`Visitor`].
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
        match self {
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
                .and_then(|| {
                    Control::Enable(ir::Enable::from(ctrl.cond.clone()))
                        .visit(visitor, component, sigs)
                })?
                .and_then(|| ctrl.body.visit(visitor, component, sigs))?
                .pop()
                .and_then(|| visitor.finish_while(ctrl, component, sigs))?,
            Control::Enable(ctrl) => visitor.enable(ctrl, component, sigs)?,
            Control::Empty(ctrl) => visitor.empty(ctrl, component, sigs)?,
            Control::Invoke(data) => visitor.invoke(data, component, sigs)?,
        }
        .apply_change(self)
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
