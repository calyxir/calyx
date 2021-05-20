use std::rc::Rc;
use vast::v05::ast as v;

pub fn cond(
    clk: &str,
    exprs: Vec<(Option<v::Expr>, v::Sequential)>,
) -> v::Stmt {
    let mut always = v::ParallelProcess::new_always();
    always.set_event(v::Sequential::new_posedge(clk));

    if let Some(branch) = exprs.into_iter().rfold(None, |acc, (cond, expr)| {
        let branch = v::SequentialIfElse {
            cond,
            body: vec![expr],
            elsebr: acc,
        };
        Some(Rc::new(branch.into()))
    }) {
        always.add_seq((*branch).clone());
        always.into()
    } else {
        panic!("exprs needs to have at least 1 element")
    }
}

pub fn cond_non_blk_assign<E>(
    clk: &str,
    var: E,
    exprs: Vec<(Option<v::Expr>, v::Expr)>,
) -> v::Stmt
where
    E: Into<v::Expr> + Clone,
{
    cond(
        clk,
        exprs
            .into_iter()
            .map(|(cond, expr)| {
                (cond, v::Sequential::new_nonblk_assign(var.clone(), expr))
            })
            .collect(),
    )
}
