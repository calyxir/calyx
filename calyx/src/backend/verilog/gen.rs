use crate::backend::traits::{Backend, Emitable};
use crate::errors;
use crate::lang::pretty_print::display;
use crate::lang::{
    ast, ast::Control, colors, colors::ColorHelper, component, context,
    pretty_print::PrettyHelper, structure, structure::EdgeData,
    structure::NodeData,
};
use bumpalo::Bump;
use petgraph::graph::NodeIndex;
use pretty::termcolor::ColorSpec;
use pretty::RcDoc as D;
use std::io::Write;

/// Implements a simple Verilog backend. The backend
/// only accepts Futil programs with control that is
/// a top level `seq` with only `enable`s as children:
/// ```
/// (seq (enable A B)
///      (enable B C)
///       ...)
/// ```
/// or control that is just a single `enable`.
/// ```
/// (enable A B)
/// ```
pub struct VerilogBackend {}

impl Backend for VerilogBackend {
    fn name() -> &'static str {
        "verilog"
    }

    fn validate(ctx: &context::Context) -> Result<(), errors::Error> {
        let prog: ast::NamespaceDef = ctx.clone().into();
        for comp in &prog.components {
            match &comp.control {
                Control::Enable { .. } | Control::Empty { .. } => (),
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
            .append(
                D::line()
                    .append(self.signature.doc(&arena, &comp))
                    .nest(4)
                    .append(D::line())
                    .parens(),
            )
            .append(";")
            .append(D::line())
            .append(D::line())
            .append(colors::comment(D::text("// Structure wire declarations")))
            .append(D::line())
            .append(wire_declarations(&comp))
            .append(D::line())
            .append(D::line())
            // .append(colors::comment(D::text("// Valid wire declarations")))
            // .append(D::line())
            // .append(valid_declarations(&arena, &comp))
            // .append(D::line())
            // .append(D::line())
            .append(colors::comment(D::text("// Subcomponent Instances")))
            .append(D::line())
            .append(subcomponent_instances(&comp));
        // let control = self.control.doc(&arena, &comp);
        let inner = structure;
        // .append(D::line())
        // .append(D::line())
        // .append(control);

        colors::comment(D::text("// Component Signature"))
            .append(D::line())
            .append(D::text("module").define_color())
            .append(inner.nest(2))
            .append(D::line())
            .append(D::text("endmodule").define_color())
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
            D::text("input")
                .port_color()
                .append(D::space())
                .append(pd.doc(&arena, &comp))
        });
        let outputs = self.outputs.iter().map(|pd| {
            D::text("output")
                .port_color()
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
        D::text("logic")
            .keyword_color()
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
    let wires = structure
        .instances()
        .filter_map(|(idx, data)| match data {
            structure::NodeData::Instance {
                name, signature, ..
            } => Some(
                signature
                    .outputs
                    .into_iter()
                    .filter_map(move |portdef| {
                        wire_string(portdef, structure, idx, &name)
                    })
                    .collect::<Vec<_>>(),
            ),
            NodeData::Input(portdef) => Some(
                structure
                    .connected_to(idx, portdef.name.to_string())
                    .filter_map(|(node, edge)| match node {
                        NodeData::Output(_) => Some(alias(edge.clone())),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten();
    D::intersperse(wires, D::line())
}

// fn valid_declarations<'a>(
//     arena: &'a Bump,
//     comp: &component::Component,
// ) -> D<'a, ColorSpec> {
//     let all = get_all_used(&arena, &comp.control);
//     let docs = all.iter().map(|id| {
//         let name = format!("{}$valid", id.as_ref());
//         colors::keyword(D::text("logic"))
//             .append(D::space())
//             .append(colors::ident(D::text(name)))
//             .append(";")
//     });
//     D::intersperse(docs, D::line())
// }

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
            D::text("wire")
                .keyword_color()
                .append(D::space())
                .append(bit_width(portdef.width))
                .append(D::text(wire_name).ident_color())
                .append(";")
                .append(D::space())
                .append(dest_comment),
        )
    } else {
        None
    }
}

fn alias<'a>(edge: EdgeData) -> D<'a, ColorSpec> {
    D::text("assign")
        .keyword_color()
        .append(D::space())
        .append(edge.dest)
        .append(" = ")
        .append(edge.src)
        .append(";")
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
    let subs = comp.structure.instances().filter_map(|(idx, data)| {
        if let NodeData::Instance {
            name,
            structure,
            signature,
        } = data
        {
            let doc = subcomponent_sig(&name, &structure)
                .append(D::space())
                .append(
                    D::line()
                        .append(signature_connections(
                            &name, &signature, &comp, idx,
                        ))
                        .nest(4)
                        .append(D::line())
                        .parens(),
                )
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

    D::text(name.to_string())
        .ident_color()
        .append(D::line())
        .append("#")
        .append(
            D::intersperse(
                params.iter().map(|param| D::text(param.to_string())),
                D::text(",").append(D::line()),
            )
            .group()
            .parens(),
        )
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
                        .append(
                            (D::text(portdef.name.to_string())).port_color(),
                        )
                        .append(D::text(wire_name).ident_color().parens())
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
                    .append(D::text(portdef.name.to_string()).port_color())
                    .append(D::text(wire_name).ident_color().parens()),
            )
        } else {
            None
        }
    });
    // let valid_wire = format!("{}$valid", name.as_ref());
    // let valid = D::text(".")
    //     .append(colors::port(D::text("valid")))
    //     .append(parens(colors::ident(D::text(valid_wire))));

    D::intersperse(incoming.chain(outgoing), D::text(",").append(D::line()))
    // .append(",")
    // .append(D::line())
    // .append(valid)
}
