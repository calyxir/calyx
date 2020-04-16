use crate::lang::component::Component;
use crate::lang::{ast, ast::Control, context::Context};
use crate::passes::remove_par::RemovePar;
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
/// Pass that collapses
/// ```
/// (par (enable A B)
///      (enable C D))
///      ..)
/// ```
/// into
/// ```
/// (par (enable A B C D)
///      ..)
/// ```
///
/// when the sub-graphs induced by (enable A B) and (enable C D) have no common
/// edges (i.e. cannot observe each other's computation).
///
/// For example, suppose that this were the structure graph of your component:
/// ```
/// ╭─╮    ╭─╮
/// │A│    │C│
/// ╰┬╯    ╰┬╯
///  │      │
///  │      │
///  v      v
/// ╭─╮    ╭─╮
/// │B│    │D│
/// ╰─╯    ╰─╯
/// ```
/// In this case, the program
/// ```
/// (par (enable A B) (enable C D))
/// ```
/// is equivalent to
/// ```
/// (par (enable A B C D))
/// ```
/// because there are no edges between the sub-graph induced by `A` and `B`
/// and the sub-graph induced by `C` and `D`.
///
/// If instead this were your component graph:
/// ```
/// ╭─╮    ╭─╮
/// │A│───>│C│
/// ╰┬╯    ╰┬╯
///  │      │
///  │      │
///  v      v
/// ╭─╮    ╭─╮
/// │B│    │D│
/// ╰─╯    ╰─╯
/// ```
/// then `par` should be collapsed to:
/// ```
/// ╭─╮   ╭──╮   ╭─╮
/// │A│──>│id│──>│C│
/// ╰┬╯   ╰──╯   ╰┬╯
///  │            │
///  │            │
///  v            v
/// ╭─╮          ╭─╮
/// │B│          │D│
/// ╰─╯          ╰─╯
/// ```
/// which we can represented as
/// ```
/// (par (enable A B C D))
/// ```
/// and replace the control of `enable A C` with
/// ```
/// (enable A id C)

pub struct ControlId {
    data: RemovePar,
}

impl ControlId {
    pub fn new(data: RemovePar) -> Self {
        ControlId { data }
    }
}

impl Named for ControlId {
    fn name() -> &'static str {
        "control-id"
    }

    fn description() -> &'static str {
        "the second par of merging parallel enables"
    }
}

impl Visitor for ControlId {
    fn finish_par(
        &mut self,
        par: &ast::Par,
        _comp: &mut Component,
        _ctx: &Context,
    ) -> VisResult {
        // get first enable statement and its index. this will act
        // as the accumulator for the statement that we are collapsing
        // things into
        let (start, mut cmp_acc) = match par.stmts.iter().enumerate().find_map(
            |(i, stmt)| match stmt {
                Control::Enable { data } => Some((i, data.clone())),
                _ => None,
            },
        ) {
            Some((s, c)) => (s, c),
            None => return Ok(Action::Continue),
        };

        // vec of new control statements including all elements up to the first
        // enable that we find
        let mut new_stmts: Vec<ast::Control> = par.stmts[..start].to_vec();

        // start interation from the start+1 because the start is in `cmp_stmt`.
        // This loop will keep trying to merge adjacent parallels as long as
        // there are no conflicts. When it detects a conflict, it attempts
        // to
        for stmt in par.stmts[start + 1..].iter() {
            match stmt {
                Control::Enable { data: enables2 } => {
                    // for every component in the `enables2` we check if any
                    // incoming/outgoing edge has an endpoint in `cmp_acc`
                    cmp_acc.comps.append(&mut enables2.comps.clone());
                }
                _ => {
                    new_stmts.push(stmt.clone());
                }
            }
        }

        if !cmp_acc.comps.is_empty() {
            new_stmts.push(Control::enable(cmp_acc.comps));
        }
        Ok(Action::Change(Control::par(new_stmts)))
    }

    fn start_enable(
        &mut self,
        enable: &ast::Enable,
        _comp: &mut Component,
        _ctx: &Context,
    ) -> VisResult {
        let mut new_comps: Vec<ast::Id> = enable.comps.clone();

        for c1 in enable.comps.iter() {
            match self.data.edge_clear.get(c1) {
                Some(comp_vec) => {
                    let mut ids: Vec<ast::Id> = comp_vec
                        .iter()
                        .filter(|c2| enable.comps.contains(c2))
                        .map(|c2| {
                            format!("{}_{}_id", c1.to_string(), c2.to_string())
                                .into()
                        })
                        .collect();
                    if !ids.is_empty() {
                        new_comps.append(&mut ids);
                    }
                }
                None => continue,
            }
        }

        Ok(Action::Change(Control::enable(new_comps)))
    }
}
