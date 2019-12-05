// Inspired by this blog post: http://thume.ca/2019/04/18/writing-a-compiler-in-rust/

use crate::lang::ast::*;

pub struct Changes {
    new_comps: Vec<Component>,
    new_struct: Vec<Structure>,
    new_node: Option<Control>,
}

impl Changes {
    pub fn add_component(&mut self, comp: Component) {
        self.new_comps.push(comp);
    }

    pub fn add_structure(&mut self, structure: Structure) {
        self.new_struct.push(structure);
    }

    pub fn change_node(&mut self, control: Control) {
        self.new_node = Some(control);
    }

    fn new() -> Self {
        Changes {
            new_comps: vec![],
            new_struct: vec![],
            new_node: None,
        }
    }
}

/** The `Visitor` trait parameterized on an `Error` type.
For each node `x` in the Ast, there are the functions `start_x`
and `finish_x`. The start functions are called at the beginning
of the traversal for each node, and the finish functions are called
at the end of the traversal for each node. You can use the finish
functions to wrap error with more information. */
pub trait Visitor<Err> {
    fn new() -> Self
    where
        Self: Sized;

    fn name(&self) -> String;

    fn do_pass(&mut self, syntax: &mut Namespace) -> &mut Self
    where
        Self: Sized,
    {
        let mut changes = Changes::new();
        for comp in &mut syntax.components {
            comp.control
                .visit(self, &mut changes)
                .unwrap_or_else(|_x| panic!("{} failed!", self.name()));
            comp.structure.append(&mut changes.new_struct);
            changes.new_struct = vec![];
        }
        syntax.components.append(&mut changes.new_comps);
        self
    }

    // fn start_namespace(&mut self, _n: &mut Namespace) -> Result<(), Err> {
    //     Ok(())
    // }

    // fn finish_namespace(
    //     &mut self,
    //     _n: &mut Namespace,
    //     res: Result<(), Err>,
    // ) -> Result<(), Err> {
    //     res
    // }

    // fn start_component(&mut self, _c: &mut Component) -> Result<(), Err> {
    //     Ok(())
    // }

    // fn finish_component(
    //     &mut self,
    //     _c: &mut Component,
    //     res: Result<(), Err>,
    // ) -> Result<(), Err> {
    //     res
    // }

    fn start_seq(&mut self, _s: &mut Seq, _c: &mut Changes) -> Result<(), Err> {
        Ok(())
    }

    fn finish_seq(
        &mut self,
        _s: &mut Seq,
        _c: &mut Changes,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_par(&mut self, _s: &mut Par, _c: &mut Changes) -> Result<(), Err> {
        Ok(())
    }

    fn finish_par(
        &mut self,
        _s: &mut Par,
        _x: &mut Changes,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_if(&mut self, _s: &mut If, _c: &mut Changes) -> Result<(), Err> {
        Ok(())
    }

    fn finish_if(
        &mut self,
        _s: &mut If,
        _x: &mut Changes,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_ifen(
        &mut self,
        _s: &mut Ifen,
        _c: &mut Changes,
    ) -> Result<(), Err> {
        Ok(())
    }

    fn finish_ifen(
        &mut self,
        _s: &mut Ifen,
        _x: &mut Changes,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_while(
        &mut self,
        _s: &mut While,
        _c: &mut Changes,
    ) -> Result<(), Err> {
        Ok(())
    }

    fn finish_while(
        &mut self,
        _s: &mut While,
        _x: &mut Changes,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_print(
        &mut self,
        _s: &mut Print,
        _x: &mut Changes,
    ) -> Result<(), Err> {
        Ok(())
    }

    fn finish_print(
        &mut self,
        _s: &mut Print,
        _x: &mut Changes,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_enable(
        &mut self,
        _s: &mut Enable,
        _x: &mut Changes,
    ) -> Result<(), Err> {
        Ok(())
    }

    fn finish_enable(
        &mut self,
        _s: &mut Enable,
        _x: &mut Changes,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_disable(
        &mut self,
        _s: &mut Disable,
        _x: &mut Changes,
    ) -> Result<(), Err> {
        Ok(())
    }

    fn finish_disable(
        &mut self,
        _s: &mut Disable,
        _x: &mut Changes,
        res: Result<(), Err>,
    ) -> Result<(), Err> {
        res
    }

    fn start_empty(
        &mut self,
        _s: &mut Empty,
        _x: &mut Changes,
    ) -> Result<(), Err> {
        Ok(())
    }

    fn finish_empty(
        &mut self,
        _s: &mut Empty,
        _x: &mut Changes,
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
    fn visit<Err>(
        &mut self,
        visitor: &mut dyn Visitor<Err>,
        changes: &mut Changes,
    ) -> Result<(), Err>;
}

// Blanket impl for Vectors of Visitables
impl<V: Visitable> Visitable for Vec<V> {
    fn visit<Err>(
        &mut self,
        visitor: &mut dyn Visitor<Err>,
        changes: &mut Changes,
    ) -> Result<(), Err> {
        for t in self {
            t.visit(visitor, changes)?;
        }
        Ok(())
    }
}

// impl Visitable for Namespace {
//     fn visit<Err>(
//         &mut self,
//         structure: &mut Structure,
//         visitor: &mut dyn Visitor<Err>,
//     ) -> Result<(), Err> {
//         visitor.start_namespace(self)?;
//         let res = self.components.visit(structure, visitor);
//         visitor.finish_namespace(self, res)
//     }
// }

// impl Visitable for Component {
//     fn visit<Err>(
//         &mut self,
//         _structure: &mut Structure,
//         visitor: &mut dyn Visitor<Err>,
//     ) -> Result<(), Err> {
//         visitor.start_component(self)?;
//         let res = self.control.visit(&mut self.structure, visitor);
//         visitor.finish_component(self, res)
//     }
// }

impl Visitable for Control {
    fn visit<Err>(
        &mut self,
        visitor: &mut dyn Visitor<Err>,
        changes: &mut Changes,
    ) -> Result<(), Err> {
        match self {
            Control::Seq { data } => {
                visitor.start_seq(data, changes)?;
                let res = data.stmts.visit(visitor, changes);
                let res2 = visitor.finish_seq(data, changes, res);
                match &changes.new_node {
                    Some(c) => {
                        *self = c.clone();
                    }
                    None => (),
                }
                res2
            }
            Control::Par { data } => {
                visitor.start_par(data, changes)?;
                let res = data.stmts.visit(visitor, changes);
                let res2 = visitor.finish_par(data, changes, res);
                match &changes.new_node {
                    Some(c) => {
                        *self = c.clone();
                    }
                    None => (),
                }
                res2
            }
            Control::If { data } => {
                visitor.start_if(data, changes)?;
                // closure to combine the results
                let res = (|| {
                    data.tbranch.visit(visitor, changes)?;
                    data.fbranch.visit(visitor, changes)
                })();
                let res2 = visitor.finish_if(data, changes, res);
                match &changes.new_node {
                    Some(c) => {
                        *self = c.clone();
                    }
                    None => (),
                }
                res2
            }
            Control::Ifen { data } => {
                visitor.start_ifen(data, changes)?;
                let res = (|| {
                    data.tbranch.visit(visitor, changes)?;
                    data.fbranch.visit(visitor, changes)
                })();
                let res2 = visitor.finish_ifen(data, changes, res);
                match &changes.new_node {
                    Some(c) => {
                        *self = c.clone();
                    }
                    None => (),
                }
                res2
            }
            Control::While { data } => {
                visitor.start_while(data, changes)?;
                let res = data.body.visit(visitor, changes);
                let res2 = visitor.finish_while(data, changes, res);
                match &changes.new_node {
                    Some(c) => {
                        *self = c.clone();
                    }
                    None => (),
                }
                res2
            }
            Control::Print { data } => {
                let res = visitor.start_print(data, changes);
                let res2 = visitor.finish_print(data, changes, res);
                match &changes.new_node {
                    Some(c) => {
                        *self = c.clone();
                    }
                    None => (),
                }
                res2
            }
            Control::Enable { data } => {
                let res = visitor.start_enable(data, changes);
                let res2 = visitor.finish_enable(data, changes, res);
                match &changes.new_node {
                    Some(c) => {
                        *self = c.clone();
                    }
                    None => (),
                }
                res2
            }
            Control::Disable { data } => {
                let res = visitor.start_disable(data, changes);
                let res2 = visitor.finish_disable(data, changes, res);
                match &changes.new_node {
                    Some(c) => {
                        *self = c.clone();
                    }
                    None => (),
                }
                res2
            }
            Control::Empty { data } => {
                let res = visitor.start_empty(data, changes);
                let res2 = visitor.finish_empty(data, changes, res);
                match &changes.new_node {
                    Some(c) => {
                        *self = c.clone();
                    }
                    None => (),
                }
                res2
            }
        }
    }
}
