use crate::backend::traits::{Backend, Emitable};
use crate::errors;
use crate::lang::pretty_print::{brackets, display, parens};
use crate::lang::{
    ast, ast::Control, colors, component, context, structure,
    structure::NodeData,
};
use bumpalo::Bump;
use itertools::Itertools;
use petgraph::graph::NodeIndex;
use pretty::termcolor::ColorSpec;
use pretty::RcDoc as D;
use std::io::Write;

pub struct VerilogBackend {}

impl Backend for VerilogBackend {
    fn name() -> &'static str {
        "verilog"
    }

    fn validate(ctx: &context::Context) -> Result<(), errors::Error> {
        let prog: ast::NamespaceDef = ctx.clone().into();
        for comp in &prog.components {
            match &comp.control {
                Control::Seq { data } => {
                    for con in &data.stmts {
                        match con {
                            Control::Enable { .. } => (),
                            _ => return Err(errors::Error::MalformedControl),
                        }
                    }
                }
                Control::Enable { .. } => (),
                _ => return Err(errors::Error::MalformedControl),
            }
        }
        Ok(())
    }

    fn emit<W: Write>(
        ctx: &context::Context,
        file: W,
    ) -> Result<(), errors::Error> {
        let prog: ast::NamespaceDef = ctx.clone().into();

        // build Vec of tuples first so that `comps` lifetime is longer than
        // `docs` lifetime
        let comps: Vec<(&ast::ComponentDef, component::Component)> = prog
            .components
            .iter()
            .map(|cd| (cd, ctx.get_component(&cd.name).unwrap()))
            .collect();

        let mut arena = Bump::new();
        let docs = comps.iter().map(|(cd, comp)| cd.doc(&arena, &comp));
        display(
            D::intersperse(docs, D::line().append(D::line())),
            Some(file),
        );
        arena.reset();
        Ok(())
    }
}

impl Emitable for ast::ComponentDef {
    fn doc<'a>(
        &self,
        arena: &'a Bump,
        comp: &component::Component,
    ) -> D<'a, ColorSpec> {
        let structure = D::nil()
            .append(D::space())
            .append(self.name.to_string())
            .append(D::line())
            .append(parens(
                D::line()
                    .append(self.signature.doc(&arena, &comp))
                    .nest(4)
                    .append(D::line()),
            ))
            .append(";")
            .append(D::line())
            .append(D::line())
            .append(colors::comment(D::text("// Wire declarations")))
            .append(D::line())
            .append(wire_declarations(&comp))
            .append(D::line())
            .append(D::line())
            .append(colors::comment(D::text("// Subcomponent Instances")))
            .append(D::line())
            .append(subcomponent_instances(&comp));
        let control = self.control.doc(&arena, &comp);
        let inner = structure
            .append(D::line())
            .append(D::line())
            .append(control);

        colors::comment(D::text("// Component Signature"))
            .append(D::line())
            .append(colors::define(D::text("module")))
            .append(inner.nest(2))
            .append(D::line())
            .append(colors::define(D::text("endmodule")))
            .append(D::space())
            .append(colors::comment(D::text(format!(
                "// end {}",
                self.name.to_string()
            ))))
    }
}

impl Emitable for ast::Signature {
    fn doc<'a>(
        &self,
        arena: &'a Bump,
        comp: &component::Component,
    ) -> D<'a, ColorSpec> {
        let inputs = self.inputs.iter().map(|pd| {
            colors::port(D::text("input"))
                .append(D::space())
                .append(pd.doc(&arena, &comp))
        });
        let outputs = self.outputs.iter().map(|pd| {
            colors::port(D::text("output"))
                .append(D::space())
                .append(pd.doc(&arena, &comp))
        });
        D::intersperse(inputs.chain(outputs), D::text(",").append(D::line()))
    }
}

impl Emitable for ast::Portdef {
    fn doc<'a>(
        &self,
        _arena: &'a Bump,
        _ctx: &component::Component,
    ) -> D<'a, ColorSpec> {
        // XXX(sam) why would we not use wires?
        colors::keyword(D::text("wire"))
            .append(D::space())
            .append(bit_width(self.width))
            .append(D::space())
            .append(self.name.to_string())
    }
}

//==========================================
//        Wire Declaration Functions
//==========================================
fn wire_declarations<'a>(comp: &component::Component) -> D<'a, ColorSpec> {
    // declare a wire for each instance, not input or output
    // because they already have wires defined
    // in the module signature. We only make a wire
    // once for each instance output. This is because
    // Verilog wires do not correspond exactly with Futil wires.
    let structure: &structure::StructureGraph = &comp.structure;
    let wires =
        structure
            .instances()
            .filter_map(|(idx, data)| match data {
                structure::NodeData::Instance {
                    name, signature, ..
                } => Some(signature.outputs.into_iter().filter_map(
                    move |portdef| wire_string(portdef, structure, idx, &name),
                )),
                _ => None,
            })
            .flatten();
    D::intersperse(wires, D::line())
}

fn wire_string<'a>(
    portdef: ast::Portdef,
    structure: &structure::StructureGraph,
    idx: NodeIndex,
    name: &ast::Id,
) -> Option<D<'a, ColorSpec>> {
    // build comment of the destinations of each wire declaration
    let dests: Vec<D<ColorSpec>> = structure
        .connected_to(idx, portdef.name.to_string())
        .map(|(node, edge)| {
            D::text(node.get_name().to_string())
                .append(" @ ")
                .append(edge.dest.to_string())
        })
        .collect();
    if !dests.is_empty() {
        let dest_comment = colors::comment(
            D::text("// ").append(D::intersperse(dests, D::text(", "))),
        );
        let wire_name =
            format!("{}${}", &name.to_string(), &portdef.name.to_string());
        Some(
            colors::keyword(D::text("wire"))
                .append(D::space())
                .append(bit_width(portdef.width))
                .append(colors::ident(D::text(wire_name)))
                .append(";")
                .append(D::space())
                .append(dest_comment),
        )
    } else {
        None
    }
}

pub fn bit_width<'a>(width: u64) -> D<'a, ColorSpec> {
    use std::cmp::Ordering;
    match width.cmp(&1) {
        Ordering::Less => panic!("Invalid bit width!"),
        Ordering::Equal => D::nil(),
        Ordering::Greater => {
            D::text(format!("[{}:0]", width - 1)).append(D::space())
        }
    }
}

//==========================================
//        Subcomponent Instance Functions
//==========================================
fn subcomponent_instances<'a>(comp: &component::Component) -> D<'a, ColorSpec> {
    use crate::lang::structure::NodeData;
    let subs = comp.structure.instances().filter_map(|(idx, data)| {
        if let NodeData::Instance {
            name,
            structure,
            signature,
        } = data
        {
            let doc = subcomponent_sig(&name, &structure)
                .append(D::space())
                .append(parens(
                    D::line()
                        .append(signature_connections(
                            &name, &signature, &comp, idx,
                        ))
                        .nest(4)
                        .append(D::line()),
                ))
                .append(";");
            Some(doc)
        } else {
            None
        }
    });
    D::intersperse(subs, D::line().append(D::line()))
}

fn subcomponent_sig<'a>(
    id: &ast::Id,
    structure: &ast::Structure,
) -> D<'a, ColorSpec> {
    use ast::Structure;
    let (name, params): (&ast::Id, &[u64]) = match structure {
        Structure::Decl { data } => (&data.component, &[]),
        Structure::Std { data } => (&data.instance.name, &data.instance.params),
        Structure::Wire { .. } => {
            panic!("Shouldn't have a wire in the structure graph")
        }
    };

    colors::ident(D::text(name.to_string()))
        .append(D::line())
        .append("#")
        .append(parens(
            D::intersperse(
                params.iter().map(|param| D::text(param.to_string())),
                D::text(",").append(D::line()),
            )
            .group(),
        ))
        .append(D::line())
        .append(id.to_string())
        .group()
}

fn signature_connections<'a>(
    name: &ast::Id,
    sig: &ast::Signature,
    comp: &component::Component,
    idx: NodeIndex,
) -> D<'a, ColorSpec> {
    let incoming = sig
        .inputs
        .iter()
        .map(|portdef| {
            comp.structure
                .connected_from(idx, portdef.name.to_string())
                .map(move |(src, edge)| {
                    let wire_name = match src {
                        NodeData::Input(pd) | NodeData::Output(pd) => {
                            pd.name.to_string()
                        }
                        NodeData::Instance { name, .. } => {
                            format!("{}${}", name.to_string(), &edge.src)
                        }
                    };
                    D::text(".")
                        .append(colors::port(D::text(portdef.name.to_string())))
                        .append(parens(colors::ident(D::text(wire_name))))
                })
        })
        .flatten();
    let outgoing = sig.outputs.iter().filter_map(|portdef| {
        if comp
            .structure
            .connected_to(idx, portdef.name.to_string())
            .count()
            > 0
        {
            let wire_name =
                format!("{}${}", &name.to_string(), &portdef.name.to_string());
            Some(
                D::text(".")
                    .append(colors::port(D::text(portdef.name.to_string())))
                    .append(parens(colors::ident(D::text(wire_name)))),
            )
        } else {
            None
        }
    });

    D::intersperse(incoming.chain(outgoing), D::text(",").append(D::line()))
}

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

fn necessary_bits(control: &ast::Control) -> u64 {
    let state_num = match control {
        Control::Seq { data } => data.stmts.len(),
        Control::Enable { .. } => 1,
        _ => panic!("Should have been caught by validation check"),
    };
    (state_num as f64).log2().ceil() as u64 - 1
}

fn state_variables<'a>(bits: u64) -> D<'a, ColorSpec> {
    colors::keyword(D::text("logic"))
        .append(D::space())
        .append(brackets(D::text(bits.to_string()).append(":0")))
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
                .append("(reset)")
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
    bits: u64,
    control: &ast::Control,
) -> D<'a, ColorSpec> {
    let all = get_all_used(&arena, control);
    let states = match control {
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
            D::intersperse(doc, D::line())
        }
        _ => D::nil(),
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
                .append(colors::keyword(D::text("endcase")))
                .nest(2),
        )
        .append(D::line())
        .append(colors::keyword(D::text("end")))
}

fn fsm_output_state(all_ids: &[ast::Id], enable: ast::Enable) -> D<ColorSpec> {
    let docs = all_ids.iter().map(|id| {
        if enable.comps.contains(id) {
            D::text(id.as_ref()).append(D::space()).append("= 1;")
        } else {
            D::text(id.as_ref()).append(D::space()).append("= 0;")
        }
    });
    D::intersperse(docs, D::line())
}

fn get_all_used<'a>(arena: &'a Bump, control: &ast::Control) -> &'a [ast::Id] {
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
