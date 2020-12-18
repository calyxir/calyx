use crate::frontend::library::ast::LibrarySignatures;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::Control;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
pub struct WhileSpec;

impl Named for WhileSpec {
    fn name() -> &'static str {
        "while-spec"
    }

    fn description() -> &'static str {
        "Attempts to rewrite while loops to use speculative execution"
    }
}

impl Visitor<()> for WhileSpec {
    fn finish_while(
        &mut self,
        s: &mut ir::While,
        _data: (),
        comp: &mut ir::Component,
        ctx: &LibrarySignatures,
    ) -> VisResult<()> {
        let mut builder = ir::Builder::from(comp, ctx, false);
        let ir::While { port, cond, body } = s;

        if let ir::Control::Seq(seq) = &**body {
            if let (
                ir::Control::Enable(enable1),
                ir::Control::Enable(enable2),
                ir::Control::Enable(enable3),
            ) = (&seq.stmts[0], &seq.stmts[1], &seq.stmts[2])
            {
                let enable_a = ir::Control::enable(Rc::clone(&enable1.group));
                let enable_b = ir::Control::enable(Rc::clone(&enable2.group));
                let enable_c = ir::Control::enable(Rc::clone(&enable3.group));

                let a_spec = builder.add_group(
                    enable1.group.borrow().name.to_string() + "_spec",
                    HashMap::new(),
                );
                let b_spec = builder.add_group(
                    enable2.group.borrow().name.to_string() + "_spec",
                    HashMap::new(),
                );
                let c_spec = builder.add_group(
                    enable3.group.borrow().name.to_string() + "_spec",
                    HashMap::new(),
                );

                let enable_a_spec = ir::Control::enable(Rc::clone(&a_spec));
                let enable_b_spec = ir::Control::enable(Rc::clone(&b_spec));
                let enable_c_spec = ir::Control::enable(Rc::clone(&c_spec));

                let commit = builder.add_group("commit_spec", HashMap::new());
                let enable_commit = ir::Control::enable(Rc::clone(&commit));

                let seq1 = Control::seq(vec![enable_b, enable_c]);
                let seq2 = Control::seq(vec![
                    enable_a_spec,
                    enable_b_spec,
                    enable_c_spec,
                ]);
                let par = Control::par(vec![seq1, seq2]);
                let i = Control::if_(
                    Rc::clone(&port),
                    Rc::clone(&cond),
                    Box::new(enable_commit),
                    Box::new(Control::empty()),
                );
                let outer_seq = Control::seq(vec![enable_a, par, i]);

                let w = Control::while_(
                    Rc::clone(&port),
                    Rc::clone(&cond),
                    Box::new(outer_seq),
                );
                return Ok(Action::change_default(w));
            }
        }

        Ok(Action::stop_default())
    }
}