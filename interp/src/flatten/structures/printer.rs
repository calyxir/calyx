use calyx_ir::PortComp;

use crate::flatten::flat_ir::{
    cell_prototype::{CellPrototype, LiteralOrPrimitive},
    identifier::{CanonicalIdentifier, IdMap},
    wires::guards::Guard,
};

use super::super::flat_ir::prelude::*;
use super::super::text_utils;
use super::context::Context;

use std::fmt::Write;

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

    pub fn print_group(&self, group: GroupIdx, parent: ComponentIdx) {
        println!(
            "{}",
            text_utils::indent(
                format!(
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

    pub fn print_comb_group(&self, group: CombGroupIdx, parent: ComponentIdx) {
        println!(
            "{}",
            text_utils::indent(
                format!(
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

    pub fn print_component(&self, idx: ComponentIdx) {
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
        if !self.ctx.primary[idx].continuous_assignments.is_empty() {
            println!("{}", text_utils::indent("Continuous Assignments:", 1));
            for assign in self.ctx.primary[idx].continuous_assignments.iter() {
                println!(
                    "{}",
                    text_utils::indent(self.print_assignment(idx, assign), 2)
                );
            }
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

    fn lookup_cell_prototype(
        &self,
        parent: ComponentIdx,
        cell: CellRef,
    ) -> &CellPrototype {
        match cell {
            CellRef::Local(l) => {
                &self.ctx.secondary
                    [self.ctx.secondary[parent].cell_offset_map[l]]
                    .prototype
            }
            CellRef::Ref(r) => {
                &self.ctx.secondary
                    [self.ctx.secondary[parent].ref_cell_offset_map[r]]
                    .prototype
            }
        }
    }

    fn lookup_cell_id(
        &self,
        parent: ComponentIdx,
        cell: CellRef,
    ) -> Identifier {
        match cell {
            CellRef::Local(l) => {
                self.ctx.secondary
                    [self.ctx.secondary[parent].cell_offset_map[l]]
                    .name
            }
            CellRef::Ref(r) => {
                self.ctx.secondary
                    [self.ctx.secondary[parent].ref_cell_offset_map[r]]
                    .name
            }
        }
    }

    #[inline]
    pub fn lookup_id_from_port(
        &self,
        comp: ComponentIdx,
        target: PortRef,
    ) -> CanonicalIdentifier {
        let port = self.ctx.lookup_port_definition(comp, target);
        let parent = self.ctx.find_parent_cell(comp, target);

        match (port, parent) {
            (PortDefinitionRef::Local(l), ParentIdx::Component(c)) => CanonicalIdentifier::interface_port( self.ctx.secondary[c].name, self.ctx.secondary[l].name),
            (PortDefinitionRef::Local(l), ParentIdx::Cell(c)) => {
                if let CellPrototype::Constant { value, width, c_type }= &self.ctx.secondary[c].prototype {
                    match c_type {
                        LiteralOrPrimitive::Literal => CanonicalIdentifier::literal((*width).into(), *value),
                        LiteralOrPrimitive::Primitive => CanonicalIdentifier::cell_port( self.ctx.secondary[c].name, self.ctx.secondary[l].name),
                    }
                } else {
                    CanonicalIdentifier::cell_port( self.ctx.secondary[c].name, self.ctx.secondary[l].name)
                }
            },
            (PortDefinitionRef::Local(l), ParentIdx::Group(g)) => CanonicalIdentifier::group_port( self.ctx.primary[g].name(), self.ctx.secondary[l].name),
            (PortDefinitionRef::Ref(rp), ParentIdx::RefCell(rc)) => CanonicalIdentifier::cell_port( self.ctx.secondary[rc].name, self.ctx.secondary[rp]),
            _ => unreachable!("Inconsistent port definition and parent. This should never happen"),
        }
    }

    pub fn format_control(
        &self,
        parent: ComponentIdx,
        control: ControlIdx,
        indent: usize,
    ) -> String {
        match &self.ctx.primary[control] {
            ControlNode::Empty(_) => String::new(),
            ControlNode::Enable(e) => text_utils::indent(
                format!(
                    "{};     ({:?})",
                    self.ctx.secondary[self.ctx.primary[e.group()].name()]
                        .clone(),
                    control
                ),
                indent,
            ),

            // TODO Griffin: refactor into shared function rather than copy-paste?
            ControlNode::Seq(s) => {
                let mut seq = text_utils::indent(
                    format!("seq {{  ({:?})\n", control),
                    indent,
                );
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
            ControlNode::Invoke(i) => {
                let invoked_name =
                    &self.ctx.secondary[self.lookup_cell_id(parent, i.cell)];

                let mut out = format!("invoke {invoked_name}");

                if !i.ref_cells.is_empty() {
                    let ref_cells = self.format_invoke_ref_cell_list(i, parent);
                    out += &format!("[{}]", ref_cells);
                }
                let inputs = self.format_invoke_port_lists(&i.inputs, parent);
                let outputs = self.format_invoke_port_lists(&i.outputs, parent);

                out += &format!("({inputs})({outputs})");

                if let Some(grp) = i.comb_group {
                    out += &format!(
                        " with {}",
                        self.ctx.secondary[self.ctx.primary[grp].name()]
                    );
                }

                out += ";";

                text_utils::indent(out, indent)
            }
        }
    }

    fn format_invoke_port_lists(
        &self,
        ports: &[(PortRef, PortRef)],
        parent: ComponentIdx,
    ) -> String {
        let mut out = String::new();
        for (dst, src) in ports {
            let dst = *self.lookup_id_from_port(parent, *dst).name().expect("destination for a ref cell is a literal. This should never happen");
            let src = self.lookup_id_from_port(parent, *src);
            write!(
                out,
                "{}={}, ",
                self.ctx.secondary[dst],
                src.format_name(self.string_table())
            )
            .unwrap();
        }
        // remove trailing ", "
        if out.ends_with(", ") {
            out.pop();
            out.pop();
        }

        out
    }

    fn format_invoke_ref_cell_list(
        &self,
        invoke: &Invoke,
        parent: ComponentIdx,
    ) -> String {
        let mut out = String::new();
        let invoked_cell = self
            .lookup_cell_prototype(parent, invoke.cell)
            .as_component()
            .expect("invoked a non-component with ref cells");

        let invoked_comp_info = &self.ctx.secondary[*invoked_cell];

        for (dst, src) in &invoke.ref_cells {
            let src = self.lookup_cell_id(parent, *src);

            let dst = invoked_comp_info.ref_cell_offset_map[*dst];
            let dst = self.ctx.secondary[dst].name;

            let src = &self.ctx.secondary[src];
            let dst = &self.ctx.secondary[dst];

            write!(out, "{dst}={src}, ").unwrap();
        }

        // remove trailing ", "
        if out.ends_with(", ") {
            out.pop();
            out.pop();
        }

        out
    }

    pub fn format_guard(
        &self,
        parent: ComponentIdx,
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
        parent_comp: ComponentIdx,
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
            "{} = {}{};",
            dst.format_name(self.string_table()),
            guard,
            src.format_name(self.string_table())
        )
    }
}
