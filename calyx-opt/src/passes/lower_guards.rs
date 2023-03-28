use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir, RRC};
use ir::Nothing;

/// Lowers guards into a purely structural representation. After this pass,
/// all guards are guaranteed to be either [ir::Guard::True] or [ir::Guard::Port].
#[derive(Default)]
pub struct LowerGuards;

impl Named for LowerGuards {
    fn name() -> &'static str {
        "lower-guards"
    }

    fn description() -> &'static str {
        "lower guards to a purely structural representation"
    }
}

fn guard_to_prim(guard: &ir::Guard<ir::Nothing>) -> Option<String> {
    let var_name = match guard {
        ir::Guard::Or(..) => "or",
        ir::Guard::And(..) => "and",
        ir::Guard::CompOp(op, _, _) => match op {
            ir::PortComp::Eq => "eq",
            ir::PortComp::Neq => "neq",
            ir::PortComp::Gt => "gt",
            ir::PortComp::Lt => "lt",
            ir::PortComp::Geq => "ge",
            ir::PortComp::Leq => "le",
        },
        ir::Guard::True | ir::Guard::Not(_) | ir::Guard::Port(_) => {
            return None;
        }
        ir::Guard::Info(_) => {
            panic!("Guards Shouldn't Take Info at this Point")
        }
    };
    Some(var_name.to_string())
}

fn lower_guard(
    guard: ir::Guard<Nothing>,
    assigns: &mut Vec<ir::Assignment<Nothing>>,
    builder: &mut ir::Builder,
) -> RRC<ir::Port> {
    let maybe_prim = guard_to_prim(&guard);
    match guard {
        ir::Guard::And(l, r) | ir::Guard::Or(l, r) => {
            let l_low = lower_guard(*l, assigns, builder);
            let r_low = lower_guard(*r, assigns, builder);

            let prim = maybe_prim.unwrap();
            let prim_name = format!("std_{}", prim);
            let prim_cell =
                builder.add_primitive(prim, prim_name, &[l_low.borrow().width]);
            let prim = prim_cell.borrow();

            assigns.push(builder.build_assignment(
                prim.get("left"),
                l_low,
                ir::Guard::True,
            ));
            assigns.push(builder.build_assignment(
                prim.get("right"),
                r_low,
                ir::Guard::True,
            ));
            prim.get("out")
        }

        ir::Guard::CompOp(_, l, r) => {
            let prim = maybe_prim.unwrap();
            let prim_name = format!("std_{}", prim);
            let prim_cell =
                builder.add_primitive(prim, prim_name, &[l.borrow().width]);
            let prim = prim_cell.borrow();

            assigns.push(builder.build_assignment(
                prim.get("left"),
                l,
                ir::Guard::True,
            ));
            assigns.push(builder.build_assignment(
                prim.get("right"),
                r,
                ir::Guard::True,
            ));
            prim.get("out")
        }
        ir::Guard::Not(g) => {
            let g_low = lower_guard(*g, assigns, builder);
            let not_prim = builder.add_primitive(
                "not",
                "std_not",
                &[g_low.borrow().width],
            );
            let not = not_prim.borrow();
            assigns.push(builder.build_assignment(
                not.get("in"),
                g_low,
                ir::Guard::True,
            ));
            not.get("out")
        }
        ir::Guard::True => builder.add_constant(1, 1).borrow().get("out"),
        ir::Guard::Port(p) => p,
        ir::Guard::Info(_) => panic!("shouldn't have info ports at this point"),
    }
}

fn lower_assigns(
    assigns: Vec<ir::Assignment<Nothing>>,
    builder: &mut ir::Builder,
) -> Vec<ir::Assignment<Nothing>> {
    let mut new_assigns = Vec::with_capacity(assigns.len() * 2);
    for mut assign in assigns {
        let g = std::mem::take(&mut assign.guard);
        let mut assigns = vec![];
        let port = lower_guard(*g, &mut assigns, builder);
        assign.guard = Box::new(port.into());
        new_assigns.extend(assigns);
        new_assigns.push(assign);
    }
    new_assigns
}

impl Visitor for LowerGuards {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);

        // Transform continuous assignments
        let conts: Vec<_> =
            builder.component.continuous_assignments.drain(..).collect();
        let new_conts = lower_assigns(conts, &mut builder);
        builder.component.continuous_assignments = new_conts;

        // Transform group assignments
        let groups = builder
            .component
            .get_groups_mut()
            .drain()
            .map(|group| {
                let assigns =
                    group.borrow_mut().assignments.drain(..).collect();
                let new_assigns = lower_assigns(assigns, &mut builder);
                group.borrow_mut().assignments = new_assigns;
                group
            })
            .into();
        builder.component.set_groups(groups);

        /*let static_groups = builder
            .component
            .get_static_groups_mut()
            .drain()
            .map(|group| {
                let assigns =
                    group.borrow_mut().assignments.drain(..).collect();
                let new_assigns = lower_assigns(assigns, &mut builder);
                group.borrow_mut().assignments = new_assigns;
                group
            })
            .into();
        builder.component.set_static_groups(static_groups);*/

        // Transform comb group assignments
        let comb_groups = builder
            .component
            .comb_groups
            .drain()
            .map(|group| {
                let assigns =
                    group.borrow_mut().assignments.drain(..).collect();
                let new_assigns = lower_assigns(assigns, &mut builder);
                group.borrow_mut().assignments = new_assigns;
                group
            })
            .into();
        builder.component.comb_groups = comb_groups;

        Ok(Action::Stop)
    }
}
