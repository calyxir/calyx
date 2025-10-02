use std::{collections::HashMap, ops};

use camino::Utf8PathBuf;
use cranelift_entity::PrimaryMap;

use crate::{
    exec::{IO, OpRef, Operation},
    plan_files::ast::{
        Assignment, AssignmentList, Op, Visitable, Visitor, VisitorResult,
    },
};

pub fn ast_from_steps(
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
        &mut self,
        ast: &AssignmentList,
    ) -> Vec<(OpRef, Vec<IO>, Vec<IO>)> {
        self.step_list = vec![];
        let _ = ast.visit(self);
        self.step_list.clone()
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
pub fn ast_to_step_list(
    ast: &AssignmentList,
    ops: &PrimaryMap<OpRef, Operation>,
) -> Vec<(OpRef, Vec<IO>, Vec<IO>)> {
    ASTToStepList::from_ops(ops).step_list_from_ast(ast)
}

#[derive(Default)]
struct ASTToString {
    assigns: Vec<String>,
}

impl ASTToString {
    fn new() -> Self {
        ASTToString { assigns: vec![] }
    }

    fn string_from_ast(&mut self, ast: &AssignmentList) -> String {
        self.assigns = vec![];
        let _ = ast.visit(self);
        self.assigns.join("\n")
    }
}

impl Visitor for ASTToString {
    type Result = ops::ControlFlow<()>;

    fn visit_assignment(&mut self, a: &Assignment) -> Self::Result {
        let var_vec: Vec<String> =
            a.vars.iter().map(Utf8PathBuf::to_string).collect();
        let vars = var_vec.join(", ");
        let arg_vec: Vec<String> =
            a.value.args.iter().map(Utf8PathBuf::to_string).collect();
        let args = arg_vec.join(", ");
        let assign_string = format!("{} = {}({});", vars, a.value.name, args);
        self.assigns.push(assign_string);
        Self::Result::output()
    }
}

/// Returns a pretty printed string from a flang AST. The returned string will be valid flang
/// syntax.
pub fn ast_to_string(ast: &AssignmentList) -> String {
    ASTToString::new().string_from_ast(ast)
}
