use crate::backend::traits::{Backend, Emitable};
use crate::context;
use crate::errors;
use crate::lang::pretty_print::{display, parens};
use crate::lang::{ast, colors, component, structure};
use petgraph::graph::NodeIndex;
use pretty::termcolor::ColorSpec;
use pretty::RcDoc as D;

pub struct RtlBackend {}

impl Backend for RtlBackend {
    fn validate(ctx: &context::Context) -> Result<(), errors::Error> {
        let prog: ast::NamespaceDef = ctx.clone().into();
        for comp in &prog.components {
            use ast::Control;
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

    fn emit(ctx: &context::Context) -> Result<(), errors::Error> {
        let prog: ast::NamespaceDef = ctx.clone().into();

        // build Vec of tuples first so that `comps` lifetime is longer than `docs` lifetime
        let comps: Vec<(&ast::ComponentDef, component::Component)> = prog
            .components
            .iter()
            .map(|cd| (cd, ctx.get_component(&cd.name).unwrap()))
            .collect();

        let docs = comps.iter().map(|(cd, comp)| cd.doc(&comp));
        display(D::intersperse(docs, D::line()));
        Ok(())
    }
}

impl Emitable for ast::ComponentDef {
    fn doc<'a>(&'a self, comp: &'a component::Component) -> D<'a, ColorSpec> {
        D::text("// Component Signature")
            .append(D::line())
            .append(colors::define(D::text("module")))
            .append(D::space())
            .append(&self.name)
            .append(D::line())
            .append(parens(
                D::line()
                    .append(self.signature.doc(&comp))
                    .nest(4)
                    .append(D::line()),
            ))
            .append(";")
            .append(D::line())
            .append(D::line())
            .append("// Wire declarations")
            .append(D::line())
            .append(wire_declarations(&comp))
            .append(D::line())
            .append(D::line())
            .append("// Subcomponent Instances")
            .append(D::line())
            .append(subcomponent_instances(&comp))
            .append(D::line())
            .append(colors::define(D::text("endmodule")))
            .append(D::space())
            .append(format!("// end {}", &self.name))
    }
}

impl Emitable for ast::Signature {
    fn doc<'a>(&'a self, comp: &'a component::Component) -> D<'a, ColorSpec> {
        let inputs = self.inputs.iter().map(|pd| {
            colors::port(D::text("input"))
                .append(D::space())
                .append(pd.doc(&comp))
        });
        let outputs = self.outputs.iter().map(|pd| {
            colors::port(D::text("output"))
                .append(D::space())
                .append(pd.doc(&comp))
        });
        D::intersperse(inputs.chain(outputs), D::text(",").append(D::line()))
    }
}

impl Emitable for ast::Portdef {
    fn doc(&self, _ctx: &component::Component) -> D<ColorSpec> {
        // XXX(sam) why would we not use wires?
        colors::keyword(D::text("wire"))
            .append(D::space())
            .append(bit_width(self.width))
            .append(D::space())
            .append(&self.name)
    }
}

//==========================================
//        Wire Declaration Functions
//==========================================
fn wire_declarations(comp: &component::Component) -> D<ColorSpec> {
    // declare a wire for each instance, not input or output because they already have wires defined
    // in the module signature. We only make a wire once for each instance output. This is because
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
    structure: &'a structure::StructureGraph,
    idx: NodeIndex,
    name: &ast::Id,
) -> Option<D<'a, ColorSpec>> {
    // build comment of the destinations of each wire declaration
    let dests: Vec<D<ColorSpec>> = structure
        .connected_to(idx, portdef.name.to_string())
        .map(|(node, edge)| {
            D::text(node.get_name().to_string())
                .append(" @ ")
                .append(&edge.dest)
        })
        .collect();
    if !dests.is_empty() {
        let dest_comment =
            D::text("// ").append(D::intersperse(dests, D::text(", ")));
        let wire_name = format!("{}${}", &name, &portdef.name);
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
    let (name, params): (&str, &[u64]) = match structure {
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
                    let wire_name =
                        format!("{}${}", &src.get_name(), &edge.src);
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
            let wire_name = format!("{}${}", &name, &portdef.name);
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
