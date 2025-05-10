use std::rc::Rc;
use vast::v05::ast as v;

/// Helper for generating a Verilog conditional inside of
/// an `always` block. `exprs` is a list of tuples mapping
/// some condition to a v::Sequential that should be executed
/// when the condition is true.
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

/// Special case of `cond` where you want to write to the same register
/// in every branch and you want non-blocking assignments everywhere.
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
