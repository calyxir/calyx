use crate::backend::traits::Emitable;
use crate::lang::{
    ast, ast::Control, colors, component, pretty_print::brackets,
};
use bumpalo::Bump;
use itertools::Itertools;
use pretty::termcolor::ColorSpec;
use pretty::RcDoc as D;

//==========================================
//        Control Generation
//==========================================
impl Emitable for ast::Control {
    fn doc<'a>(
        &self,
        arena: &'a Bump,
        comp: &component::Component,
    ) -> D<'a, ColorSpec> {
        let bits = necessary_bits(&comp.control);
        state_variables(bits)
            .append(D::line())
            .append(D::line())
            .append(state_transition())
            .append(D::line())
            .append(D::line())
            .append(increment_state())
            .append(D::line())
            .append(D::line())
            .append(seq_fsm(&arena, bits, &comp.control))
    }
}

fn necessary_bits(control: &ast::Control) -> u32 {
    let state_num = match control {
        Control::Seq { data } => data.stmts.len(),
        Control::Enable { .. } => 1,
        _ => panic!("Should have been caught by validation check"),
    };
    (state_num as f32).log2().ceil() as u32
}

fn state_variables<'a>(bits: u32) -> D<'a, ColorSpec> {
    colors::keyword(D::text("logic"))
        .append(D::space())
        .append(brackets(D::text((bits - 1).to_string()).append(":0")))
        .append(D::space())
        .append("state, next_state;")
}

fn state_transition<'a>() -> D<'a, ColorSpec> {
    colors::comment(D::text("// state transition (counter)"))
        .append(D::line())
        .append(colors::define(D::text("always_ff")))
        .append(D::space())
        .append("@(posedge clk)")
        .append(D::space())
        .append(colors::keyword(D::text("begin")))
        .append(
            D::line()
                .append(colors::keyword(D::text("if")))
                .append(D::space())
                .append("(!valid)")
                .append(D::line().append("state <= 0;").nest(2))
                .append(D::line())
                .append(colors::keyword(D::text("else")))
                .append(D::line().append("state <= next_state;").nest(2))
                .nest(2),
        )
        .append(D::line())
        .append(colors::keyword(D::text("end")))
}

fn increment_state<'a>() -> D<'a, ColorSpec> {
    colors::comment(D::text("// next state logic"))
        .append(D::line())
        .append(colors::define(D::text("always_comb")))
        .append(D::space())
        .append(colors::keyword(D::text("begin")))
        .append(D::line().append(D::text("next_state = state + 1;")).nest(2))
        .append(D::line())
        .append(colors::keyword(D::text("end")))
}

fn seq_fsm<'a>(
    arena: &'a Bump,
    bits: u32,
    control: &ast::Control,
) -> D<'a, ColorSpec> {
    let all = get_all_used(&arena, control);
    let (num_states, states) = match control {
        Control::Seq { data } => {
            let doc =
                data.stmts.iter().enumerate().map(|(i, stmt)| match stmt {
                    Control::Enable { data } => {
                        D::text(format!("{}'d{}:", bits, i))
                            .append(D::space())
                            .append(colors::keyword(D::text("begin")))
                            .append(
                                D::line()
                                    .append(fsm_output_state(
                                        &all,
                                        data.clone(),
                                    ))
                                    .nest(2),
                            )
                            .append(D::line())
                            .append(colors::keyword(D::text("end")))
                    }
                    _ => D::nil(),
                });
            (data.stmts.len(), D::intersperse(doc, D::line()))
        }
        _ => (0, D::nil()),
    };

    let default = if (num_states as u32) < 2u32.pow(bits) {
        colors::keyword(D::text("default"))
            .append(":")
            .append(D::space())
            .append(colors::keyword(D::text("begin")))
            .append(D::space())
            .append(colors::keyword(D::text("end")))
    } else {
        colors::comment(D::text("// all cases covered"))
    };

    colors::comment(D::text("// sequential fsm"))
        .append(D::line())
        .append(colors::define(D::text("always_comb")))
        .append(D::space())
        .append(colors::keyword(D::text("begin")))
        .append(
            D::line()
                .append(colors::keyword(D::text("case")))
                .append(D::space())
                .append("(state)")
                .append(D::line().append(states).nest(2))
                .append(D::line())
                .append(default)
                .append(D::line())
                .append(colors::keyword(D::text("endcase")))
                .nest(2),
        )
        .append(D::line())
        .append(colors::keyword(D::text("end")))
}

fn fsm_output_state(all_ids: &[ast::Id], enable: ast::Enable) -> D<ColorSpec> {
    let docs = all_ids.iter().map(|id| {
        let name = format!("{}$valid", id.as_ref());
        let doc = D::text(name).append(D::space());
        if enable.comps.contains(id) {
            doc.append("= 1;")
        } else {
            doc.append("= 0;")
        }
    });
    D::intersperse(docs, D::line())
}

pub fn get_all_used<'a>(
    arena: &'a Bump,
    control: &ast::Control,
) -> &'a [ast::Id] {
    let comps = match control {
        Control::Enable { data } => data.comps.clone(),
        Control::Seq { data } => data
            .stmts
            .iter()
            .map(|stmt| {
                if let Control::Enable { data } = stmt {
                    data.comps.clone()
                } else {
                    vec![]
                }
            })
            .flatten()
            .unique()
            .collect(),
        _ => vec![],
    };
    arena.alloc(comps)
}
