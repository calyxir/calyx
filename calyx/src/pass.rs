// Inspired by this blog post: http://thume.ca/2019/04/18/writing-a-compiler-in-rust/

use crate::ast::*;

/** The `Visitor` trait parameterized on an `Error` type.
For each node `x` in the Ast, there are the functions `start_x`
and `finish_x`. The start functions are called at the beginning
of the traversal for each node, and the finish functions are called
at the end of the traversal for each node. You can use the finish
functions to wrap error with more information. */
pub trait Visitor<Err> {
    fn start_namespace(&mut self, _n: &mut Namespace) -> Result<(), Err> {
        Ok(())
    }

    fn finish_namespace(
        &mut self,
        _n: &mut Namespace,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_component(&mut self, _c: &mut Component) -> Result<(), Err> {
        Ok(())
    }

    fn finish_component(
        &mut self,
        _c: &mut Component,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_seq(&mut self, _s: &mut Control) -> Result<(), Err> {
        Ok(())
    }

    fn finish_seq(
        &mut self,
        _s: &mut Control,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_par(&mut self, _s: &mut Control) -> Result<(), Err> {
        Ok(())
    }

    fn finish_par(
        &mut self,
        _s: &mut Control,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_if(&mut self, _s: &mut Control) -> Result<(), Err> {
        Ok(())
    }

    fn finish_if(
        &mut self,
        _s: &mut Control,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_ifen(&mut self, _s: &mut Control) -> Result<(), Err> {
        Ok(())
    }

    fn finish_ifen(
        &mut self,
        _s: &mut Control,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_while(&mut self, _s: &mut Control) -> Result<(), Err> {
        Ok(())
    }

    fn finish_while(
        &mut self,
        _s: &mut Control,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_print(&mut self, _s: &mut Control) -> Result<(), Err> {
        Ok(())
    }

    fn finish_print(
        &mut self,
        _s: &mut Control,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_enable(&mut self, _s: &mut Control) -> Result<(), Err> {
        Ok(())
    }

    fn finish_enable(
        &mut self,
        _s: &mut Control,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_disable(&mut self, _s: &mut Control) -> Result<(), Err> {
        Ok(())
    }

    fn finish_disable(
        &mut self,
        _s: &mut Control,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_empty(&mut self, _s: &mut Control) -> Result<(), Err> {
        Ok(())
    }

    fn finish_empty(
        &mut self,
        _s: &mut Control,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }
}

/** `Visitable` describes types that can be visited by things
implementing `Visitor`. This performs a recursive walk of the tree.
It calls `Visitor::start_*` on the way down, and `Visitor::finish_*`
on the way up. */
pub trait Visitable {
    fn visit<Err>(&mut self, visitor: &mut dyn Visitor<Err>)
        -> Result<(), Err>;
}

impl<V: Visitable> Visitable for Vec<V> {
    fn visit<Err>(
        &mut self,
        visitor: &mut dyn Visitor<Err>,
    ) -> Result<(), Err> {
        for t in self {
            t.visit(visitor)?;
        }
        Ok(())
    }
}

impl Visitable for Namespace {
    fn visit<Err>(
        &mut self,
        visitor: &mut dyn Visitor<Err>,
    ) -> Result<(), Err> {
        visitor.start_namespace(self)?;
        let res = self.components.visit(visitor);
        visitor.finish_namespace(self, res)
    }
}

impl Visitable for Component {
    fn visit<Err>(
        &mut self,
        visitor: &mut dyn Visitor<Err>,
    ) -> Result<(), Err> {
        visitor.start_component(self)?;
        let res = self.control.visit(visitor);
        visitor.finish_component(self, res)
    }
}

impl Visitable for Control {
    fn visit<Err>(
        &mut self,
        visitor: &mut dyn Visitor<Err>,
    ) -> Result<(), Err> {
        // call start_* functions on visitor. This needs to be in a separate
        // match because otherwise the types don't work out.
        // XXX(sam) figure out a better way to do this
        match self {
            Control::Seq(_) => visitor.start_seq(self),
            Control::Par(_) => visitor.start_par(self),
            Control::If {
                cond: _,
                tbranch: _,
                fbranch: _,
            } => visitor.start_if(self),
            Control::Ifen {
                cond: _,
                tbranch: _,
                fbranch: _,
            } => visitor.start_ifen(self),
            Control::While { cond: _, body: _ } => visitor.start_while(self),
            Control::Print(_) => visitor.start_print(self),
            Control::Enable(_) => visitor.start_enable(self),
            Control::Disable(_) => visitor.start_disable(self),
            Control::Empty => visitor.start_empty(self),
        }?;

        match self {
            Control::Seq(stmts) => {
                let res = stmts.visit(visitor);
                visitor.finish_seq(self, res)
            }
            Control::Par(stmts) => {
                let res = stmts.visit(visitor);
                visitor.finish_par(self, res)
            }
            Control::If {
                cond: _,
                tbranch,
                fbranch,
            } => {
                // closure to combine the results
                let res = (|| {
                    tbranch.visit(visitor)?;
                    fbranch.visit(visitor)
                })();
                visitor.finish_if(self, res)
            }
            Control::Ifen {
                cond: _,
                tbranch,
                fbranch,
            } => {
                let res = (|| {
                    tbranch.visit(visitor)?;
                    fbranch.visit(visitor)
                })();
                visitor.finish_ifen(self, res)
            }
            Control::While { cond: _, body } => {
                let res = body.visit(visitor);
                visitor.finish_ifen(self, res)
            }
            Control::Print(_) => visitor.finish_print(self, Ok(())),
            Control::Enable(_) => visitor.finish_enable(self, Ok(())),
            Control::Disable(_) => visitor.finish_disable(self, Ok(())),
            Control::Empty => visitor.finish_empty(self, Ok(())),
        }
    }
}
