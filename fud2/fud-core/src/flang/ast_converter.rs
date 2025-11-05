use cranelift_entity::PrimaryMap;
use std::{collections::HashMap, ops};

use crate::{
    exec::{IO, OpRef, Operation},
    flang::ast::{
        Assignment, AssignmentList, Op, Visitable, Visitor, VisitorResult,
    },
};

pub fn steps_to_ast(
    plan: &Vec<(OpRef, Vec<IO>, Vec<IO>)>,
    ops: &PrimaryMap<OpRef, Operation>,
) -> AssignmentList {
    let mut ast = AssignmentList { assigns: vec![] };
    for step in plan {
        let vars = step
            .1
            .iter()
            .map(|v| match v {
                IO::StdIO(utf8_path_buf) => utf8_path_buf,
                IO::File(utf8_path_buf) => utf8_path_buf,
            })
            .cloned()
            .collect();
        let args = step
            .2
            .iter()
            .map(|v| match v {
                IO::StdIO(utf8_path_buf) => utf8_path_buf,
                IO::File(utf8_path_buf) => utf8_path_buf,
            })
            .cloned()
            .collect();

        let fun = Op {
            name: ops[step.0].name.clone(),
            args,
        };

        let assignment = Assignment { vars, value: fun };
        ast.assigns.push(assignment);
    }

    ast
}

/// A struct to convert a flang AST into the steps of a `Plan`.
struct ASTToStepList {
    step_list: Vec<(OpRef, Vec<IO>, Vec<IO>)>,
    name_to_op_ref: HashMap<String, OpRef>,
}

impl ASTToStepList {
    fn from_ops(ops: &PrimaryMap<OpRef, Operation>) -> Self {
        let name_to_op_ref =
            ops.iter().map(|(k, v)| (v.name.clone(), k)).collect();
        ASTToStepList {
            step_list: vec![],
            name_to_op_ref,
        }
    }

    fn step_list_from_ast(
        mut self,
        ast: &AssignmentList,
    ) -> Vec<(OpRef, Vec<IO>, Vec<IO>)> {
        let _ = ast.visit(&mut self);
        self.step_list
    }
}

impl Visitor for ASTToStepList {
    type Result = ops::ControlFlow<()>;

    fn visit_assignment(&mut self, a: &Assignment) -> Self::Result {
        let vars = a.vars.iter().map(|s| IO::File(s.clone())).collect();
        let args = a.value.args.iter().map(|s| IO::File(s.clone())).collect();
        let op_ref = self.name_to_op_ref[&a.value.name];

        self.step_list.push((op_ref, vars, args));
        Self::Result::output()
    }
}

/// Given a flang AST and a set of ops, returns the steps of a `Plan` which the flang AST
/// represents.
pub fn ast_to_steps(
    ast: &AssignmentList,
    ops: &PrimaryMap<OpRef, Operation>,
) -> Vec<(OpRef, Vec<IO>, Vec<IO>)> {
    ASTToStepList::from_ops(ops).step_list_from_ast(ast)
}
