use std::{collections::HashMap, ops, str::FromStr};

use camino::Utf8PathBuf;
use cranelift_entity::PrimaryMap;

use crate::{
    exec::{IO, OpRef, Operation},
    plan_files::ast::{
        Assignment, AssignmentList, Function, Visitable, Visitor, VisitorResult,
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
                IO::StdIO(utf8_path_buf) => utf8_path_buf.to_string(),
                IO::File(utf8_path_buf) => utf8_path_buf.to_string(),
            })
            .collect();
        let args = step
            .2
            .iter()
            .map(|v| match v {
                IO::StdIO(utf8_path_buf) => utf8_path_buf.to_string(),
                IO::File(utf8_path_buf) => utf8_path_buf.to_string(),
            })
            .collect();

        let fun = Function {
            name: ops[step.0].name.clone(),
            args,
        };

        let assignment = Assignment { vars, value: fun };
        ast.assigns.push(assignment);
    }

    ast
}

pub struct ASTToStepList {
    step_list: Vec<(OpRef, Vec<IO>, Vec<IO>)>,
    name_to_op_ref: HashMap<String, OpRef>,
}

impl ASTToStepList {
    pub fn from_ops(ops: &PrimaryMap<OpRef, Operation>) -> Self {
        let name_to_op_ref =
            ops.iter().map(|(k, v)| (v.name.clone(), k)).collect();
        ASTToStepList {
            step_list: vec![],
            name_to_op_ref,
        }
    }

    pub fn step_list_from_ast(
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
        let vars = a
            .vars
            .iter()
            .map(|s| IO::File(Utf8PathBuf::from_str(s).unwrap()))
            .collect();
        let args = a
            .value
            .args
            .iter()
            .map(|s| IO::File(Utf8PathBuf::from_str(s).unwrap()))
            .collect();
        let op_ref = self.name_to_op_ref[&a.value.name];

        self.step_list.push((op_ref, vars, args));
        Self::Result::output()
    }
}
