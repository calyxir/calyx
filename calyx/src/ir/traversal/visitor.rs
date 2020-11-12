//! Implements a visitor for `ir::Control` programs.
//! Program passes implemented as the Visitor are directly invoked on
//! `ir::Context` to compile every `ir::Component` using the pass.
use super::action::{Action, VisResult};
use crate::errors::FutilResult;
use crate::frontend::library::ast::LibrarySignatures;
use crate::ir::{self, Component, Context, Control};
use std::rc::Rc;

/// Trait that describes named things. Calling `do_pass` and `do_pass_default`
/// require this to be implemented. This has to be a separate trait from `Visitor`
/// because these methods don't recieve `self` which means that it is impossible
/// to create dynamic trait objects.
pub trait Named {
    /// The name of a pass. Is used for identifying passes.
    fn name() -> &'static str;

    /// A short description of the pass.
    fn description() -> &'static str;
}

/// The visiting interface for a `ir::Control` program.
/// Contains two kinds of functions:
/// 1. start_<node>: Called when visiting <node> top-down.
/// 2. finish_<node>: Called when visiting <node> bottow-up.
///
/// The function do not provide mutable references to the Component itself.
/// The idiomatic way to mutating is the Component is using its helper methods
/// or getting an internally mutable reference to it.
///
/// A pass will usually override one or more function and rely on the default
/// visitors to automatically visit the children.
pub trait Visitor<T: Default> {
    /// Instantiate this pass using the default() method and run it on the
    /// context.
    fn do_pass_default(context: &mut Context) -> FutilResult<Self>
    where
        Self: Default + Sized + Named,
    {
        let mut visitor = Self::default();
        visitor.do_pass(context)?;
        Ok(visitor)
    }

    /// Run the visitor on the program Context.
    /// The function makes complex use of interior mutability. See inline
    /// comments for an explanation.
    fn do_pass(&mut self, context: &mut Context) -> FutilResult<()>
    where
        Self: Sized + Named,
    {
        let signatures = &context.lib_sigs;
        context
            .components
            // Mutably borrow the components in the context
            .iter_mut()
            .map(|mut comp| {
                self.start(&mut comp, signatures)?
                    .and_then(|x| {
                        // Create a clone of the reference to the Control
                        // program.
                        let control_ref = Rc::clone(&comp.control);
                        // Borrow the control program mutably and visit it.
                        let action_tuple = control_ref
                            .borrow_mut()
                            .visit(self, x, &mut comp, signatures)?;
                        Ok(Action::continue_with(action_tuple.data))
                    })?
                    .and_then(|x| self.finish(x, &mut comp, signatures))?
                    .apply_change(&mut comp.control.borrow_mut())?;
                Ok(())
            })
            .collect::<FutilResult<_>>()?;

        Ok(())
    }

    /// Exceuted before the traversal begins.
    fn start(
        &mut self,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        Ok(Action::continue_default())
    }

    /// Exceuted after the traversal ends.
    /// This method is always invoked regardless of the `Action` returned from
    /// the children.
    fn finish(
        &mut self,
        data: T,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        Ok(Action::continue_with(data))
    }

    /// Excecuted before visiting the children of a `ir::Seq` node.
    fn start_seq(
        &mut self,
        _s: &mut ir::Seq,
        data: T,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        Ok(Action::continue_with(data))
    }

    /// Excecuted after visiting the children of a `ir::Seq` node.
    fn finish_seq(
        &mut self,
        _s: &mut ir::Seq,
        data: T,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        Ok(Action::continue_with(data))
    }

    /// Excecuted before visiting the children of a `ir::Par` node.
    fn start_par(
        &mut self,
        _s: &mut ir::Par,
        data: T,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        Ok(Action::continue_with(data))
    }

    /// Excecuted after visiting the children of a `ir::Par` node.
    fn finish_par(
        &mut self,
        _s: &mut ir::Par,
        data: T,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        Ok(Action::continue_with(data))
    }

    /// Excecuted before visiting the children of a `ir::If` node.
    fn start_if(
        &mut self,
        _s: &mut ir::If,
        data: T,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        Ok(Action::continue_with(data))
    }

    /// Excecuted after visiting the children of a `ir::If` node.
    fn finish_if(
        &mut self,
        _s: &mut ir::If,
        data: T,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        Ok(Action::continue_with(data))
    }

    /// Excecuted before visiting the children of a `ir::If` node.
    fn start_while(
        &mut self,
        _s: &mut ir::While,
        data: T,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        Ok(Action::continue_with(data))
    }

    /// Excecuted after visiting the children of a `ir::If` node.
    fn finish_while(
        &mut self,
        _s: &mut ir::While,
        data: T,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        Ok(Action::continue_with(data))
    }

    /// Excecuted before visiting the children of a `ir::Enable` node.
    fn start_enable(
        &mut self,
        _s: &mut ir::Enable,
        data: T,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        Ok(Action::continue_with(data))
    }

    /// Excecuted after visiting the children of a `ir::Enable` node.
    fn finish_enable(
        &mut self,
        _s: &mut ir::Enable,
        data: T,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        Ok(Action::continue_with(data))
    }

    /// Excecuted before visiting the children of a `ir::Empty` node.
    fn start_empty(
        &mut self,
        _s: &mut ir::Empty,
        data: T,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        Ok(Action::continue_with(data))
    }

    /// Excecuted after visiting the children of a `ir::Empty` node.
    fn finish_empty(
        &mut self,
        _s: &mut ir::Empty,
        data: T,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        Ok(Action::continue_with(data))
    }
}

/// `Visitable` describes types that can be visited by things implementing `Visitor`.
/// This performs a recursive walk of the tree.
/// It calls `Visitor::start_*` on the way down, and `Visitor::finish_*` on
/// the way up.
pub trait Visitable<T: Default> {
    /// Perform the traversal.
    fn visit(
        &mut self,
        visitor: &mut dyn Visitor<T>,
        data: T,
        component: &mut Component,
        signatures: &LibrarySignatures,
    ) -> VisResult<T>;
}

impl<T: Default> Visitable<T> for Control {
    fn visit(
        &mut self,
        visitor: &mut dyn Visitor<T>,
        data: T,
        component: &mut Component,
        sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        match self {
            Control::Seq(ctrl) => visitor
                .start_seq(ctrl, data, component, sigs)?
                .and_then(|x| ctrl.stmts.visit(visitor, x, component, sigs))?
                .pop()
                .and_then(|x| visitor.finish_seq(ctrl, x, component, sigs))?,
            Control::Par(ctrl) => visitor
                .start_par(ctrl, data, component, sigs)?
                .and_then(|x| ctrl.stmts.visit(visitor, x, component, sigs))?
                .pop()
                .and_then(|x| visitor.finish_par(ctrl, x, component, sigs))?,
            Control::If(ctrl) => visitor
                .start_if(ctrl, data, component, sigs)?
                .and_then(|x| ctrl.tbranch.visit(visitor, x, component, sigs))?
                .and_then(|x| ctrl.fbranch.visit(visitor, x, component, sigs))?
                .pop()
                .and_then(|x| visitor.finish_if(ctrl, x, component, sigs))?,
            Control::While(ctrl) => visitor
                .start_while(ctrl, data, component, sigs)?
                .and_then(|x| {
                    Control::Enable(ir::Enable::from(ctrl.cond.clone()))
                        .visit(visitor, x, component, sigs)
                })?
                .and_then(|x| ctrl.body.visit(visitor, x, component, sigs))?
                .pop()
                .and_then(|x| visitor.finish_while(ctrl, x, component, sigs))?,
            Control::Enable(ctrl) => visitor
                .start_enable(ctrl, data, component, sigs)?
                .pop()
                .and_then(|x| {
                    visitor.finish_enable(ctrl, x, component, sigs)
                })?,
            Control::Empty(ctrl) => visitor
                .start_empty(ctrl, data, component, sigs)?
                .pop()
                .and_then(|x| visitor.finish_empty(ctrl, x, component, sigs))?,
        }
        .apply_change(self)
    }
}

/// Blanket implementation for Vectors of Visitables
impl<T: Default, V: Visitable<T>> Visitable<T> for Vec<V> {
    fn visit(
        &mut self,
        visitor: &mut dyn Visitor<T>,
        mut data: T,
        component: &mut Component,
        sigs: &LibrarySignatures,
    ) -> VisResult<T> {
        for t in self {
            let res = t.visit(visitor, data, component, sigs)?;
            match res.action {
                Action::Continue | Action::SkipChildren | Action::Change(_) => {
                    data = res.data;
                    continue;
                }
                Action::Stop => return Ok(Action::stop_with(res.data)),
            };
        }
        Ok(Action::continue_default())
    }
}
