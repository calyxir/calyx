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
        comp: &mut Component,
        _c: &Context,
    ) -> VisResult {
    
        let mut seqs = s.clone();
        let mut enabled: Vec<String> = vec![];
        let mut done = false;
        let mut i = 0;
        use ast::Control::Enable;
        while !done {
            if i==&seqs.stmts.len()-2 {
                done = true;
            }
            match (&seqs.stmts[i], &seqs.stmts[i+1]) {
                (Enable { data: enables1 }, Enable { data: enables2}) => {
                    let mut en_index1 = vec![];
                    let mut en_index2 = vec![];
                    for en_comp in &enables1.comps {
                        //print!("{:?}", en_comp);
                        en_index1.push( (comp.structure.get_inst_index(en_comp)?).clone() );
                        
                    }
                    for en_comp in &enables2.comps {
                        en_index2.push( (comp.structure.get_inst_index(en_comp)?).clone() );
                    }
                    let mut changeable = true;
                    for e1 in &en_index1 {
                        for e2 in &en_index2{
                            if e1 == e2 {
                                changeable=false;
                                break
                            }else{
                                match comp.structure.graph.find_edge(*e1,*e2) {
                                    Some(_) => {
                                        changeable=false;
                                        break;
                                    }
                                    None => continue,
                                }
                            }
                        }
                        if !changeable {
                            break;
                        }
                    }
                    if !changeable {
                        i+=1;
                    } else {
                        let merge_enable: Vec<ast::Id> = enables1
                                    .comps
                                    .clone()
                                    .into_iter()
                                    .chain(enables2.comps.clone().into_iter())
                                    .collect();
                        seqs.stmts[i] = ast::Control::enable(merge_enable);
                        seqs.stmts.remove(i+1);
                    }
                    
                }
                _ => continue,
            }
        }
        Ok(Action::Change(ast::Control::Seq{data:seqs}))
    }
}
