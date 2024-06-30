// i tried and couldn't find a crate for this -- if you know of one, lmk

use std::{cell::RefCell, fmt::Write, rc::Rc};

trait MakefileEmittable {
    fn emit(&self) -> String;
}

enum AssignmentKind {
    Overwrite,
    Underwrite,
    Append,
}

impl MakefileEmittable for AssignmentKind {
    fn emit(&self) -> String {
        match &self {
            Self::Overwrite => "=",
            Self::Underwrite => "?=",
            Self::Append => "+=",
        }
        .to_string()
    }
}

type MakefileEmittableRef = Box<dyn MakefileEmittable>;

struct Assignment {
    kind: AssignmentKind,
    name: String,
    value: String,
}

impl Assignment {
    pub fn new<S: AsRef<str>, T: AsRef<str>>(
        kind: AssignmentKind,
        name: S,
        value: T,
    ) -> Self {
        Self {
            kind,
            name: name.as_ref().to_string(),
            value: value.as_ref().to_string(),
        }
    }
}

impl MakefileEmittable for Assignment {
    fn emit(&self) -> String {
        format!("{} {} {}", self.name, self.kind.emit(), self.value)
    }
}

struct Comment {
    text: String,
}

impl Comment {
    pub fn new<S: AsRef<str>>(text: S) -> Self {
        Self {
            text: text.as_ref().to_string(),
        }
    }
}

impl MakefileEmittable for Comment {
    fn emit(&self) -> String {
        self.text
            .lines()
            .map(|line| format!("# {}", line))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

struct Newline;

impl MakefileEmittable for Newline {
    fn emit(&self) -> String {
        String::from("\n")
    }
}

struct Rule {
    is_phony: bool,
    target: String,
    dependencies: Vec<String>,
    commands: Vec<String>,
}

impl Rule {
    fn new<S: AsRef<str>>(target: S) -> Self {
        Self {
            is_phony: false,
            target: target.as_ref().to_string(),
            dependencies: vec![],
            commands: vec![],
        }
    }
}

impl MakefileEmittable for Rule {
    fn emit(&self) -> String {
        let mut result = String::new();
        if self.is_phony {
            writeln!(&mut result, ".PHONY: {}", self.target).unwrap();
        }
        writeln!(
            &mut result,
            "{}: {}",
            self.target,
            self.dependencies.join(" ")
        )
        .unwrap();
        for (i, command) in self.commands.iter().enumerate() {
            if i > 0 {
                result.push('\n');
            }
            write!(&mut result, "\t{}", command.replace("\n", " ")).unwrap();
        }
        result
    }
}

#[derive(Clone)]
pub struct RuleRef {
    rule: Rc<RefCell<Rule>>,
}

impl RuleRef {
    fn new<S: AsRef<str>>(target: S) -> Self {
        RuleRef {
            rule: Rc::new(RefCell::new(Rule::new(target))),
        }
    }

    pub fn set_phony(&self) {
        self.rule.borrow_mut().is_phony = true;
    }

    pub fn add_dep<S: AsRef<str>>(&self, dep: S) {
        self.rule
            .borrow_mut()
            .dependencies
            .push(dep.as_ref().to_string());
    }

    pub fn add_cmd<S: AsRef<str>>(&self, cmd: S) {
        self.rule
            .borrow_mut()
            .commands
            .push(cmd.as_ref().to_string());
    }

    pub fn phony(self) -> Self {
        self.set_phony();
        self
    }

    pub fn dep<S: AsRef<str>>(self, dep: S) -> Self {
        self.add_dep(dep);
        self
    }

    pub fn cmd<S: AsRef<str>>(self, cmd: S) -> Self {
        self.add_cmd(cmd);
        self
    }
}

impl MakefileEmittable for RuleRef {
    fn emit(&self) -> String {
        self.rule.borrow().emit()
    }
}

struct Include {
    path_expr: String,
}

impl Include {
    fn new<S: AsRef<str>>(path_expr: S) -> Self {
        Self {
            path_expr: path_expr.as_ref().to_string(),
        }
    }
}

impl MakefileEmittable for Include {
    fn emit(&self) -> String {
        format!("include {}", self.path_expr)
    }
}

pub struct Makefile {
    contents: Vec<MakefileEmittableRef>,
}

impl Makefile {
    pub fn new() -> Self {
        Self { contents: vec![] }
    }

    pub fn comment<S: AsRef<str>>(&mut self, text: S) {
        self.add(Box::new(Comment::new(text)));
    }

    pub fn newline(&mut self) {
        self.add(Box::new(Newline));
    }

    pub fn assign<S: AsRef<str>, T: AsRef<str>>(&mut self, name: S, value: T) {
        self.add(Box::new(Assignment::new(
            AssignmentKind::Overwrite,
            name,
            value,
        )));
    }

    pub fn assign_without_overwrite<S: AsRef<str>, T: AsRef<str>>(
        &mut self,
        name: S,
        value: T,
    ) {
        self.add(Box::new(Assignment::new(
            AssignmentKind::Underwrite,
            name,
            value,
        )));
    }

    pub fn append<S: AsRef<str>, T: AsRef<str>>(&mut self, name: S, value: T) {
        self.add(Box::new(Assignment::new(
            AssignmentKind::Append,
            name,
            value,
        )));
    }

    pub fn rule<S: AsRef<str>>(&mut self, target: S) -> RuleRef {
        let rule = RuleRef::new(target);
        self.add(Box::new(rule.clone()));
        rule
    }

    pub fn include<S: AsRef<str>>(&mut self, path_expr: S) {
        self.add(Box::new(Include::new(path_expr)));
    }

    pub fn build(&self) -> String {
        self.contents
            .iter()
            .map(|e| e.emit())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn add(&mut self, e: Box<dyn MakefileEmittable>) {
        self.contents.push(e);
    }
}
