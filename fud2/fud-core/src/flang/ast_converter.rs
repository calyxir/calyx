use camino::Utf8PathBuf;
use cranelift_entity::PrimaryMap;
use std::ops;

use crate::{
    exec::{OpRef, Operation},
    flang::ast::{
        Assignment, AssignmentList, Op, Visitable, Visitor, VisitorResult,
    },
};

use super::{Ir, PathRef, ast, ir};

pub fn steps_to_ast(
    plan: &Vec<(OpRef, Vec<Utf8PathBuf>, Vec<Utf8PathBuf>)>,
    ops: &PrimaryMap<OpRef, Operation>,
) -> AssignmentList {
    let mut ast = AssignmentList { assigns: vec![] };
    for step in plan {
        let vars = step.1.clone();
        let args = step.2.clone();

        let fun = Op {
            name: ops[step.0].name.clone(),
            args,
        };

        let assignment = Assignment { vars, value: fun };
        ast.assigns.push(assignment);
    }

    ast
}

struct ASTToIr<'a> {
    ir: Ir,
    ops: &'a PrimaryMap<OpRef, Operation>,
}

impl ASTToIr<'_> {
    fn paths_to_refs(&mut self, vars: &Vec<Utf8PathBuf>) -> Vec<PathRef> {
        let mut out = vec![];
        for path in vars {
            let r = self.ir.path_ref(path);
            out.push(r);
        }
        out
    }
}

impl Visitor for ASTToIr<'_> {
    type Result = ops::ControlFlow<String>;

    fn visit_assignment(&mut self, a: &Assignment) -> Self::Result {
        let rets = self.paths_to_refs(&a.vars);
        let args = self.paths_to_refs(&a.value.args);
        for (r, op) in self.ops {
            if op.name == a.value.name {
                self.ir.push_vec(r, args, rets);
                return Self::Result::output();
            }
        }
        Self::Result::Break(format!("no op {} found", a.value.name))
    }
}

pub fn ast_to_prog(
    p: &ast::Prog,
    ops: &PrimaryMap<OpRef, Operation>,
) -> ir::Prog {
    let mut visitor = ASTToIr { ir: Ir::new(), ops };
    let res = p.ast.visit(&mut visitor);
    if let ops::ControlFlow::Break(e) = res {
        unimplemented!("{e}");
    }
    let mut to_path_ref =
        |v: &[Utf8PathBuf]| v.iter().map(|f| visitor.ir.path_ref(f)).collect();
    ir::Prog::from_parts(
        to_path_ref(&p.stdins),
        to_path_ref(&p.stdouts),
        to_path_ref(&p.inputs),
        to_path_ref(&p.outputs),
        visitor.ir,
    )
}
