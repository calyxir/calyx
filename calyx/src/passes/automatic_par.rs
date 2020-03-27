use crate::lang::component::Component;
use crate::lang::{ast, context::Context};
use crate::passes::visitor::{Action, VisResult, Visitor};

/// Pass that collapses 
///(seq
///    (seq (enable A B)
///         (enable C D))
/// ..)
/// into 
/// (seq (enable A B C D)
///  ..)
/// given that there are no edges between the sub-graphs induced by (enable A B) and (enable C D)
/// since in this case there is no way for these subgraphs to depend on each other
/// XXX (zhijing): I think this pass need to be changed if we add `enable` CSP style components to futil semantics
#[derive(Default)]
pub struct AutomaticPar {}

impl Visitor for AutomaticPar {
    fn name(&self) -> String {
        "automatic parallelization".to_string()
    }

    // use finish_seq so that we collapse things on the way
    // back up the tree and potentially catch more cases
    fn finish_seq(
        &mut self,
        s: &mut ast::Seq,
        _comp: &mut Component,
        _c: &Context,
    ) -> VisResult {
    
        let prog: ast::NamespaceDef = _c.clone().into();
        let mut wires = vec![];
        for comp in &prog.components {
            for stru in &comp.structure {
                match stru {
                    ast::Structure::Wire{data:d} => {
                        wires.push(d.clone())
                    }
                    _ => continue,
                }
            }
        }
        let mut enabled: Vec<String> = vec![];
        let mut done = false;
        let mut i = 0;
        use ast::Control::Enable;
        while !done {
            if i==&s.stmts.len()-2 {
                done = true;
            }
            i += 1;
            match (&s.stmts[i-1], &s.stmts[i]) {
                (Enable { data: s1 }, Enable { data: s2 }) => {
                    for en_comp in &s1.comps {
                        print!("{:?}", en_comp);
                    }
                    for en_comp in &s2.comps {
                        print!("{:?}", en_comp);
                    }
                    //seqs.append(&mut data.stmts.clone());
                }
                _ => continue,
            }
        }
        Ok(Action::Continue)
    }
}
