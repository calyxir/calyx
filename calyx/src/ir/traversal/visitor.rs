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
pub trait Visitor {
    /// Instantiate this pass using the default() method and run it on the
    /// context.
    fn do_pass_default(
        context: &mut Context,
    ) -> FutilResult<Self>
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
    fn do_pass(
        &mut self,
        context: &mut Context,
    ) -> FutilResult<()>
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
                    .and_then(|| {
                        // Create a clone of the reference to the Control
                        // program.
                        let control_ref = Rc::clone(&comp.control);
                        // Borrow the control program mutably and visit it.
                        let _ = control_ref
                            .borrow_mut()
                            .visit(self, &mut comp, signatures)?;
                        // Never skip the .finish method.
                        Ok(Action::Continue)
                    })?
                    .and_then(|| self.finish(&mut comp, signatures))?;
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
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Exceuted after the traversal ends.
    /// This method is always invoked regardless of the `Action` returned from
    /// the children.
    fn finish(
        &mut self,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted before visiting the children of a `ir::Seq` node.
    fn start_seq(
        &mut self,
        _s: &ir::Seq,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted after visiting the children of a `ir::Seq` node.
    fn finish_seq(
        &mut self,
        _s: &ir::Seq,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted before visiting the children of a `ir::Par` node.
    fn start_par(
        &mut self,
        _s: &ir::Par,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted after visiting the children of a `ir::Par` node.
    fn finish_par(
        &mut self,
        _s: &ir::Par,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted before visiting the children of a `ir::If` node.
    fn start_if(
        &mut self,
        _s: &ir::If,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted after visiting the children of a `ir::If` node.
    fn finish_if(
        &mut self,
        _s: &ir::If,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted before visiting the children of a `ir::If` node.
    fn start_while(
        &mut self,
        _s: &ir::While,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted after visiting the children of a `ir::If` node.
    fn finish_while(
        &mut self,
        _s: &ir::While,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted before visiting the children of a `ir::Enable` node.
    fn start_enable(
        &mut self,
        _s: &ir::Enable,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted after visiting the children of a `ir::Enable` node.
    fn finish_enable(
        &mut self,
        _s: &ir::Enable,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted before visiting the children of a `ir::Empty` node.
    fn start_empty(
        &mut self,
        _s: &ir::Empty,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    /// Excecuted after visiting the children of a `ir::Empty` node.
    fn finish_empty(
        &mut self,
        _s: &ir::Empty,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }
}

/// `Visitable` describes types that can be visited by things implementing `Visitor`.
/// This performs a recursive walk of the tree.
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
            Control::Seq(data) => visitor
                .start_seq(data, component, sigs)?
                .and_then(|| data.stmts.visit(visitor, component, sigs))?
                .pop()
                .and_then(|| visitor.finish_seq(data, component, sigs))?,
            Control::Par(data) => visitor
                .start_par(data, component, sigs)?
                .and_then(|| data.stmts.visit(visitor, component, sigs))?
                .pop()
                .and_then(|| visitor.finish_par(data, component, sigs))?,
            Control::If(data) => visitor
                .start_if(data, component, sigs)?
                .and_then(|| data.tbranch.visit(visitor, component, sigs))?
                .and_then(|| data.fbranch.visit(visitor, component, sigs))?
                .pop()
                .and_then(|| visitor.finish_if(data, component, sigs))?,
            Control::While(data) => visitor
                .start_while(data, component, sigs)?
                .and_then(|| data.body.visit(visitor, component, sigs))?
                .pop()
                .and_then(|| visitor.finish_while(data, component, sigs))?,
            Control::Enable(data) => visitor
                .start_enable(data, component, sigs)?
                .pop()
                .and_then(|| visitor.finish_enable(data, component, sigs))?,
            Control::Empty(data) => visitor
                .start_empty(data, component, sigs)?
                .pop()
                .and_then(|| visitor.finish_empty(data, component, sigs))?,
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
            match t.visit(visitor, component, sigs)? {
                Action::Continue | Action::SkipChildren | Action::Change(_) => {
                    continue
                }
                Action::Stop => return Ok(Action::Stop),
            };
        }
        Ok(Action::Continue)
    }
}
