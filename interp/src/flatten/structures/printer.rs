use calyx_ir::PortComp;

use crate::flatten::flat_ir::{
    identifier::{CanonicalIdentifier, IdMap},
    wires::guards::Guard,
};

use super::super::flat_ir::prelude::*;
use super::super::text_utils;
use super::context::Context;

pub struct Printer<'a> {
    ctx: &'a Context,
}

impl<'a> Printer<'a> {
    pub fn new(ctx: &'a Context) -> Self {
        Self { ctx }
    }

    #[inline]
    fn string_table(&self) -> &IdMap {
        &self.ctx.secondary.string_table
    }

    pub fn print_group(&self, group: GroupIdx, parent: ComponentRef) {
        println!(
            "{}",
            text_utils::indent(
                &format!(
                    "Group: {}",
                    self.ctx.secondary[self.ctx.primary[group].name()]
                ),
                1
            )
        );
        for assign in self.ctx.primary[group].assignments.iter() {
            println!(
                "{}",
                text_utils::indent(self.print_assignment(parent, assign), 2)
            );
        }
    }

    pub fn print_comb_group(&self, group: CombGroupIdx, parent: ComponentRef) {
        println!(
            "{}",
            text_utils::indent(
                &format!(
                    "Comb Group: {}",
                    self.ctx.secondary[self.ctx.primary[group].name()]
                ),
                1
            )
        );
        for assign in self.ctx.primary[group].assignments.iter() {
            println!(
                "{}",
                text_utils::indent(self.print_assignment(parent, assign), 2)
            );
        }
    }

    pub fn print_component(&self, idx: ComponentRef) {
        println!(
            "Component: {}",
            self.ctx.resolve_id(self.ctx.secondary[idx].name)
        );
        for x in self.ctx.secondary[idx].definitions.groups() {
            self.print_group(x, idx)
        }

        for x in self.ctx.secondary[idx].definitions.comb_groups() {
            self.print_comb_group(x, idx)
        }
        println!();
        println!("{}", text_utils::indent("Control:", 1));
        if let Some(ctrl) = self.ctx.primary[idx].control {
            println!("{}", self.format_control(idx, ctrl, 2));
        }
    }

    pub fn print_program(&self) {
        for idx in self.ctx.primary.components.keys() {
            self.print_component(idx);
            println!()
        }
    }

    #[inline]
    pub fn lookup_id_from_port(
        &self,
        comp: ComponentRef,
        target: PortRef,
    ) -> CanonicalIdentifier {
        let port = self.ctx.lookup_port_definition(comp, target);
        let parent = self.ctx.find_parent_cell(comp, target);

        match (port, parent) {
            (PortDefinitionRef::Local(l), ParentIdx::Component(c)) => CanonicalIdentifier::interface_port( self.ctx.secondary[c].name, self.ctx.secondary[l]),
            (PortDefinitionRef::Local(l), ParentIdx::Cell(c)) => CanonicalIdentifier::cell_port( self.ctx.secondary[c].name(), self.ctx.secondary[l]),
            (PortDefinitionRef::Local(l), ParentIdx::Group(g)) => CanonicalIdentifier::group_port( self.ctx.primary[g].name(), self.ctx.secondary[l]),
            (PortDefinitionRef::Ref(rp), ParentIdx::RefCell(rc)) => CanonicalIdentifier::cell_port( self.ctx.secondary[rc].name(), self.ctx.secondary[rp]),
            _ => unreachable!("Inconsistent port definition and parent. This should never happen"),
        }
    }

    pub fn format_control(
        &self,
        parent: ComponentRef,
        control: ControlIdx,
        indent: usize,
    ) -> String {
        match &self.ctx.primary[control] {
            ControlNode::Empty(_) => String::new(),
            ControlNode::Enable(e) => text_utils::indent(
                self.ctx.secondary[self.ctx.primary[e.group()].name()].clone()
                    + ";",
                indent,
            ),

            // TODO Griffin: refactor into shared function rather than copy-paste?
            ControlNode::Seq(s) => {
                let mut seq = text_utils::indent("seq {\n", indent);
                for stmt in s.stms() {
                    let child = self.format_control(parent, *stmt, indent + 1);
                    seq += &child;
                    seq += "\n";
                }
                seq += &text_utils::indent("}", indent);
                seq
            }
            ControlNode::Par(p) => {
                let mut par = text_utils::indent("par {\n", indent);
                for stmt in p.stms() {
                    let child = self.format_control(parent, *stmt, indent + 1);
                    par += &child;
                    par += "\n";
                }
                par += &text_utils::indent("}", indent);
                par
            }
            ControlNode::If(i) => {
                let cond = self.lookup_id_from_port(parent, i.cond_port());
                let mut out = text_utils::indent(
                    format!("if {} ", cond.format_name(self.string_table())),
                    indent,
                );
                if let Some(grp) = i.cond_group() {
                    out += &format!(
                        "with {} ",
                        self.ctx.secondary[self.ctx.primary[grp].name()]
                    );
                }
                out += "{\n";

                let t_branch =
                    self.format_control(parent, i.tbranch(), indent + 1);
                let f_branch =
                    self.format_control(parent, i.fbranch(), indent + 1);

                out += &t_branch;
                out += "\n";
                out += &text_utils::indent("}", indent);

                if !f_branch.is_empty() {
                    out += &format!(" else {{\n{}\n", f_branch);
                    out += &(text_utils::indent("}\n", indent));
                }

                out
            }
            ControlNode::While(w) => {
                let cond = self.lookup_id_from_port(parent, w.cond_port());
                let mut out = text_utils::indent(
                    format!("while {} ", cond.format_name(self.string_table())),
                    indent,
                );
                if let Some(grp) = w.cond_group() {
                    out += &format!(
                        "with {} ",
                        self.ctx.secondary[self.ctx.primary[grp].name()]
                    );
                }
                out += "{\n";

                let body = self.format_control(parent, w.body(), indent + 1);
                out += &(body + "\n");
                out += &text_utils::indent("}", indent);

                out
            }
            ControlNode::Invoke(_) => {
                text_utils::indent("<<<<INVOKE PLACEHOLDER>>>>", indent)
            }
        }
    }

    pub fn format_guard(
        &self,
        parent: ComponentRef,
        guard: GuardIdx,
    ) -> String {
        fn op_to_str(op: &PortComp) -> String {
            match op {
                PortComp::Eq => String::from("=="),
                PortComp::Neq => String::from("!="),
                PortComp::Gt => String::from(">"),
                PortComp::Lt => String::from("<"),
                PortComp::Geq => String::from(">="),
                PortComp::Leq => String::from("<="),
            }
        }

        match &self.ctx.primary.guards[guard] {
            Guard::True => String::new(),
            Guard::Or(l, r) => {
                let l = self.format_guard(parent, *l);
                let r = self.format_guard(parent, *r);
                format!("({} | {})", l, r)
            }
            Guard::And(l, r) => {
                let l = self.format_guard(parent, *l);
                let r = self.format_guard(parent, *r);
                format!("({} & {})", l, r)
            }
            Guard::Not(n) => {
                let n = self.format_guard(parent, *n);
                format!("!{}", n)
            }
            Guard::Comp(op, l, r) => {
                let l = self.lookup_id_from_port(parent, *l);
                let r = self.lookup_id_from_port(parent, *r);
                format!(
                    "{} {} {}",
                    l.format_name(&self.ctx.secondary.string_table),
                    op_to_str(op),
                    r.format_name(&self.ctx.secondary.string_table)
                )
            }
            Guard::Port(p) => {
                let p = self.lookup_id_from_port(parent, *p);
                p.format_name(&self.ctx.secondary.string_table)
            }
        }
    }

    pub fn print_assignment(
        &self,
        parent_comp: ComponentRef,
        target: AssignmentIdx,
    ) -> String {
        let assign = &self.ctx.primary.assignments[target];
        let dst = self.lookup_id_from_port(parent_comp, assign.dst);
        let src = self.lookup_id_from_port(parent_comp, assign.src);
        let guard = self.format_guard(parent_comp, assign.guard);
        let guard = if guard.is_empty() {
            guard
        } else {
            format!("{} ? ", guard)
        };

        format!(
            "{} = {}{}",
            dst.format_name(self.string_table()),
            guard,
            src.format_name(self.string_table())
        )
    }
}
