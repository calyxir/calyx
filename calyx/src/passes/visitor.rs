// Inspired by this blog post: http://thume.ca/2019/04/18/writing-a-compiler-in-rust/

use crate::context::Context;
use crate::errors;
use crate::lang::ast::*;

pub enum Action {
    /// Continue AST traversal
    Continue,
    /// Stop AST traversal
    Stop,
    /// Change the current ast node. Implies ending
    /// the traversal for this branch of the AST
    Change(Control),
}

impl Action {
    /// Monadic helper function that sequences actions
    /// that return a VisResult.
    /// If `self` is `Continue` or `Change`, return the result of running `f`.
    /// Pass `Stop` through
    fn and_then<F>(self, mut other: F) -> VisResult
    where
        F: FnMut() -> VisResult,
    {
        match self {
            Action::Continue => other(),
            x => Ok(x),
        }
    }

    /// Applies the Change action if `self` is a Change action.
    /// Otherwise passes the action through unchanged
    fn apply_change(self, con: &mut Control) -> VisResult {
        match self {
            Action::Change(c) => {
                *con = c;
                Ok(Action::Continue)
            }
            x => Ok(x),
        }
    }
}

pub type VisResult = Result<Action, errors::Error>;

/** The `Visitor` trait parameterized on an `Error` type.
For each node `x` in the Ast, there are the functions `start_x`
and `finish_x`. The start functions are called at the beginning
of the traversal for each node, and the finish functions are called
at the end of the traversal for each node. You can use the finish
functions to wrap error with more information. */
pub trait Visitor {
    fn name(&self) -> String;

    fn do_pass(&mut self, context: &Context) -> &mut Self
    where
        Self: Sized,
    {
        for (id, mut comp) in context.definitions_mut() {
            println!("{:?}", id);
            let _res = comp.control.visit(self, context);
        }
        self
    }

    fn start_seq(&mut self, _s: &mut Seq, _c: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn finish_seq(&mut self, _s: &mut Seq, _c: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn start_par(&mut self, _s: &mut Par, _c: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn finish_par(&mut self, _s: &mut Par, _x: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn start_if(&mut self, _s: &mut If, _c: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn finish_if(&mut self, _s: &mut If, _x: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn start_ifen(&mut self, _s: &mut Ifen, _c: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn finish_ifen(&mut self, _s: &mut Ifen, _x: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn start_while(&mut self, _s: &mut While, _c: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn finish_while(&mut self, _s: &mut While, _x: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn start_print(&mut self, _s: &mut Print, _x: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn finish_print(&mut self, _s: &mut Print, _x: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn start_enable(&mut self, _s: &mut Enable, _x: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn finish_enable(&mut self, _s: &mut Enable, _x: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn start_disable(&mut self, _s: &mut Disable, _x: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn finish_disable(&mut self, _s: &mut Disable, _x: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn start_empty(&mut self, _s: &mut Empty, _x: &Context) -> VisResult {
        Ok(Action::Continue)
    }

    fn finish_empty(&mut self, _s: &mut Empty, _x: &Context) -> VisResult {
        Ok(Action::Continue)
    }
}

/** `Visitable` describes types that can be visited by things
implementing `Visitor`. This performs a recursive walk of the tree.
It calls `Visitor::start_*` on the way down, and `Visitor::finish_*`
on the way up. */
pub trait Visitable {
    fn visit(
        &mut self,
        visitor: &mut dyn Visitor,
        context: &Context,
    ) -> VisResult;
}

// Blanket impl for Vectors of Visitables
impl<V: Visitable> Visitable for Vec<V> {
    fn visit(
        &mut self,
        visitor: &mut dyn Visitor,
        context: &Context,
    ) -> VisResult {
        for t in self {
            match t.visit(visitor, context)? {
                Action::Continue | Action::Change(_) => continue,
                Action::Stop => return Ok(Action::Stop),
            };
        }
        Ok(Action::Continue)
    }
}

impl Visitable for Control {
    fn visit(
        &mut self,
        visitor: &mut dyn Visitor,
        context: &Context,
    ) -> VisResult {
        match self {
            Control::Seq { data } => visitor
                .start_seq(data, context)?
                .and_then(|| data.stmts.visit(visitor, context))?
                .and_then(|| visitor.finish_seq(data, context))?,
            Control::Par { data } => visitor
                .start_par(data, context)?
                .and_then(|| data.stmts.visit(visitor, context))?
                .and_then(|| visitor.finish_par(data, context))?,
            Control::If { data } => visitor
                .start_if(data, context)?
                .and_then(|| data.tbranch.visit(visitor, context))?
                .and_then(|| data.fbranch.visit(visitor, context))?
                .and_then(|| visitor.finish_if(data, context))?,
            Control::Ifen { data } => visitor
                .start_ifen(data, context)?
                .and_then(|| data.tbranch.visit(visitor, context))?
                .and_then(|| data.fbranch.visit(visitor, context))?
                .and_then(|| visitor.finish_ifen(data, context))?,
            Control::While { data } => visitor
                .start_while(data, context)?
                .and_then(|| data.body.visit(visitor, context))?
                .and_then(|| visitor.finish_while(data, context))?,
            Control::Print { data } => visitor
                .start_print(data, context)?
                .and_then(|| visitor.finish_print(data, context))?,
            Control::Enable { data } => visitor
                .start_enable(data, context)?
                .and_then(|| visitor.finish_enable(data, context))?,
            Control::Disable { data } => visitor
                .start_disable(data, context)?
                .and_then(|| visitor.finish_disable(data, context))?,
            Control::Empty { data } => visitor
                .start_empty(data, context)?
                .and_then(|| visitor.finish_empty(data, context))?,
        }
        .apply_change(self)
    }
}
