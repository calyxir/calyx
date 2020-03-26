use crate::backend::traits::Emitable;
use crate::backend::verilog::control::get_all_used;
use crate::lang::{
    ast, colors, component, pretty_print::parens, structure,
    structure::NodeData,
};
use bumpalo::Bump;
use petgraph::graph::NodeIndex;
use pretty::termcolor::ColorSpec;
use pretty::RcDoc as D;

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
            .append(colors::comment(D::text("// Structure wire declarations")))
            .append(D::line())
            .append(wire_declarations(&comp))
            .append(D::line())
            .append(D::line())
            .append(colors::comment(D::text("// Valid wire declarations")))
            .append(D::line())
            .append(valid_declarations(&arena, &comp))
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
        colors::keyword(D::text("logic"))
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

fn valid_declarations<'a>(
    arena: &'a Bump,
    comp: &component::Component,
) -> D<'a, ColorSpec> {
    let all = get_all_used(&arena, &comp.control);
    let docs = all.iter().map(|id| {
        let name = format!("{}$valid", id.as_ref());
        colors::keyword(D::text("logic"))
            .append(D::space())
            .append(colors::ident(D::text(name)))
            .append(";")
    });
    D::intersperse(docs, D::line())
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
    let valid_wire = format!("{}$valid", name.as_ref());
    let valid = D::text(".")
        .append(colors::port(D::text("valid")))
        .append(parens(colors::ident(D::text(valid_wire))));

    D::intersperse(incoming.chain(outgoing), D::text(",").append(D::line()))
        .append(",")
        .append(D::line())
        .append(valid)
}
