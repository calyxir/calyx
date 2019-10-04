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

    fn start_seq(&mut self, _s: &mut Seq) -> Result<(), Err> {
        Ok(())
    }

    fn finish_seq(
        &mut self,
        _s: &mut Seq,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_par(&mut self, _s: &mut Par) -> Result<(), Err> {
        Ok(())
    }

    fn finish_par(
        &mut self,
        _s: &mut Par,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_if(&mut self, _s: &mut If) -> Result<(), Err> {
        Ok(())
    }

    fn finish_if(
        &mut self,
        _s: &mut If,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_ifen(&mut self, _s: &mut Ifen) -> Result<(), Err> {
        Ok(())
    }

    fn finish_ifen(
        &mut self,
        _s: &mut Ifen,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_while(&mut self, _s: &mut While) -> Result<(), Err> {
        Ok(())
    }

    fn finish_while(
        &mut self,
        _s: &mut While,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_print(&mut self, _s: &mut Print) -> Result<(), Err> {
        Ok(())
    }

    fn finish_print(
        &mut self,
        _s: &mut Print,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_enable(&mut self, _s: &mut Enable) -> Result<(), Err> {
        Ok(())
    }

    fn finish_enable(
        &mut self,
        _s: &mut Enable,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_disable(&mut self, _s: &mut Disable) -> Result<(), Err> {
        Ok(())
    }

    fn finish_disable(
        &mut self,
        _s: &mut Disable,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_empty(&mut self, _s: &mut Empty) -> Result<(), Err> {
        Ok(())
    }

    fn finish_empty(
        &mut self,
        _s: &mut Empty,
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

// Blanket impl for Vectors of Visitables
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
        match self {
            Control::Seq { data } => {
                visitor.start_seq(data)?;
                let res = data.stmts.visit(visitor);
                visitor.finish_seq(data, res)
            }
            Control::Par { data } => {
                visitor.start_par(data)?;
                let res = data.stmts.visit(visitor);
                visitor.finish_par(data, res)
            }
            Control::If { data } => {
                visitor.start_if(data)?;
                // closure to combine the results
                let res = (|| {
                    data.tbranch.visit(visitor)?;
                    data.fbranch.visit(visitor)
                })();
                visitor.finish_if(data, res)
            }
            Control::Ifen { data } => {
                visitor.start_ifen(data)?;
                let res = (|| {
                    data.tbranch.visit(visitor)?;
                    data.fbranch.visit(visitor)
                })();
                visitor.finish_ifen(data, res)
            }
            Control::While { data } => {
                visitor.start_while(data)?;
                let res = data.body.visit(visitor);
                visitor.finish_while(data, res)
            }
            Control::Print { data } => {
                let res = visitor.start_print(data);
                visitor.finish_print(data, res)
            }
            Control::Enable { data } => {
                let res = visitor.start_enable(data);
                visitor.finish_enable(data, res)
            }
            Control::Disable { data } => {
                let res = visitor.start_disable(data);
                visitor.finish_disable(data, res)
            }
            Control::Empty { data } => {
                let res = visitor.start_empty(data);
                visitor.finish_empty(data, res)
            }
        }
    }
}
