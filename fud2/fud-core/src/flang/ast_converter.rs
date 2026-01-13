use camino::Utf8PathBuf;
use cranelift_entity::PrimaryMap;
use std::ops;

use crate::{
    exec::{OpRef, Operation},
    flang::ast::{Assignment, Op, Visitable, Visitor, VisitorResult},
};

use super::{PathRef, Plan, ast};

struct ASTToIr<'a> {
    plan: Plan,
    ops: &'a PrimaryMap<OpRef, Operation>,
}

impl ASTToIr<'_> {
    fn paths_to_refs(&mut self, vars: &Vec<Utf8PathBuf>) -> Vec<PathRef> {
        let mut out = vec![];
        for path in vars {
            let r = self.plan.path_ref(path);
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
                self.plan.push_vec(r, args, rets);
                return Self::Result::output();
            }
        }
        Self::Result::Break(format!("no op {} found", a.value.name))
    }
}

pub fn ast_to_plan(p: &ast::Prog, ops: &PrimaryMap<OpRef, Operation>) -> Plan {
    let mut visitor = ASTToIr {
        plan: Plan::new(),
        ops,
    };
    let res = p.ast.visit(&mut visitor);
    if let ops::ControlFlow::Break(e) = res {
        unimplemented!("{e}");
    }
    let mut plan = visitor.plan;
    plan.extend_inputs_buf(&p.inputs);
    plan.extend_outputs_buf(&p.outputs);
    plan.extend_stdins_buf(&p.stdins);
    plan.extend_stdouts_buf(&p.stdouts);
    plan
}

pub fn plan_to_ast(p: &Plan, ops: &PrimaryMap<OpRef, Operation>) -> ast::Prog {
    let mut assigns = vec![];
    for a in p {
        let vars = p.to_path_buf_vec(a.rets());
        let args = p.to_path_buf_vec(a.args());
        let name = ops[a.op_ref()].name.clone();
        assigns.push(Assignment {
            vars,
            value: Op { name, args },
        });
    }
    ast::Prog {
        stdins: p.stdins_buf_vec(),
        stdouts: p.stdouts_buf_vec(),
        inputs: p.inputs_buf_vec(),
        outputs: p.outputs_buf_vec(),
        ast: ast::AssignmentList { assigns },
    }
}
