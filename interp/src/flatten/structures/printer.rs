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

    fn string_table(&self) -> &IdMap {
        &self.ctx.secondary.string_table
    }

    pub fn print_program(&self) {
        for (idx, _comp) in self.ctx.primary.components.iter() {
            println!(
                "Component: {}",
                self.ctx.resolve_id(self.ctx.secondary[idx].name)
            );
            for x in self.ctx.secondary.comp_aux_info[idx].definitions.groups()
            {
                println!(
                    "{}",
                    text_utils::indent(
                        &format!(
                            "Group: {}",
                            self.ctx.secondary[self.ctx.primary[x].name()]
                        ),
                        1
                    )
                );
                for assign in self.ctx.primary.groups[x].assignments.iter() {
                    println!(
                        "{}",
                        text_utils::indent(
                            self.print_assignment(idx, assign),
                            2
                        )
                    );
                }
            }
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
                format!("(!{})", n)
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
