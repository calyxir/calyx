use crate::backend::traits::{Backend, Emitable};
use crate::context;
use crate::errors;
use crate::lang::pretty_print::parens;
use crate::lang::{ast, component, structure};
use petgraph::graph::NodeIndex;
use pretty::RcDoc as D;
use std::rc::Rc;

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
                _ => return Err(errors::Error::MalformedControl),
            }
        }
        Ok(())
    }

    fn to_string(ctx: &context::Context) -> String {
        let mut w = Vec::new();
        let prog: ast::NamespaceDef = ctx.clone().into();
        // let x = prog
        //     .components
        //     .into_iter()
        //     .map(|c| (c, ctx.get_component(&c.name).unwrap()));
        let mut x: Vec<(ast::ComponentDef, component::Component)> = vec![];
        for cd in prog.components.iter() {
            x.push((cd.clone(), ctx.get_component(&cd.name).unwrap()))
        }
        let docs = x.iter().map(|(cd, comp)| {
            cd.doc(&comp)
            // c.doc(&comp)
        });
        let doc = D::intersperse(docs, D::line());
        doc.render(100, &mut w).unwrap();
        String::from_utf8(w).unwrap()
    }
}

impl Emitable for ast::ComponentDef {
    fn doc<'a>(&'a self, comp: &'a component::Component) -> D<'a> {
        D::text("// Component Signature")
            .append(D::line())
            .append(D::text("module"))
            .append(D::space())
            .append(D::text(&self.name))
            .append(D::line())
            .append(parens(
                D::line()
                    .append(self.signature.doc(&comp))
                    .nest(4)
                    .append(D::line()),
            ))
            .append(D::text(";"))
            .append(D::line())
            .append(D::line())
            .append(D::text("// Wire declarations"))
            .append(D::line())
            .append(wire_declarations(&self.structure, &comp))
            .append(D::line())
            .append(D::line())
            .append(D::text("// Subcomponent Instances"))
            .append(D::line())
            .append(subcomponent_instances(&comp))
            .append(D::line())
            .append(D::text("endmodule"))
            .append(D::space())
            .append(D::text(format!("// end {}", &self.name)))
    }
}

impl Emitable for ast::Signature {
    fn doc<'a>(&'a self, comp: &'a component::Component) -> D<'a> {
        // XXX(sam) do we need bitwidths?
        let inputs = self.inputs.iter().map(|pd| {
            D::text("input").append(D::space()).append(pd.doc(&comp))
        });
        let outputs = self.outputs.iter().map(|pd| {
            D::text("output").append(D::space()).append(pd.doc(&comp))
        });
        D::intersperse(inputs.chain(outputs), D::text(",").append(D::line()))
    }
}

impl Emitable for ast::Portdef {
    fn doc(&self, _ctx: &component::Component) -> D {
        // XXX(Sam) why don't we use wires?
        D::text("wire").append(D::space()).append(&self.name)
    }
}

//==========================================
//        Wire Declaration Functions
//==========================================
fn wire_declarations<'a>(
    _structure: &'a [ast::Structure],
    comp: &'a component::Component,
) -> D<'a> {
    let structure: &structure::StructureGraph = &comp.structure;
    let wires: Vec<D> = structure
        .instances()
        .filter_map(|(idx, data)| match data {
            structure::NodeData::Instance {
                name, signature, ..
            } => {
                let wires =
                    signature.outputs.into_iter().filter_map(move |portdef| {
                        let dests: Vec<D> = structure
                            .connected_to(idx, portdef.name.to_string())
                            .map(|(node, edge)| {
                                D::text(node.get_name().to_string())
                                    .append(D::text(" @ "))
                                    .append(D::text(&edge.dest))
                            })
                            .collect();
                        if dests.is_empty() {
                            None
                        } else {
                            let dest_comment = D::text("// ")
                                .append(D::intersperse(dests, D::text(", ")));
                            let wire_name =
                                format!("{}${}", &name, &portdef.name);
                            Some(
                                D::text("wire")
                                    .append(D::space())
                                    .append(bit_width(portdef.width))
                                    .append(D::text(wire_name))
                                    .append(D::space())
                                    .append(dest_comment),
                            )
                        }
                    });
                Some(wires)
            }
            _ => None,
        })
        .flatten()
        .collect();
    D::intersperse(wires, D::line())
}

fn wire_string<'a>(
    wire: &'a ast::Wire,
    comp: &component::Component,
) -> Option<D<'a>> {
    let src_node = comp.structure.get_node_from_port(&wire.src).unwrap();
    let dest_node = comp.structure.get_node_from_port(&wire.dest).unwrap();

    let width = comp
        .structure
        .get_wire_width(
            src_node,
            wire.src.port_name(),
            dest_node,
            wire.dest.port_name(),
        )
        .unwrap();
    match &wire.src {
        ast::Port::Comp { .. } => Some(
            D::text("wire")
                .append(D::space())
                .append(bit_width(width))
                // .append(port_wire_id(&wire))
                .append(D::text(";")),
        ),
        ast::Port::This { .. } => None,
    }
}

pub fn bit_width<'a>(width: u64) -> D<'a> {
    use std::cmp::Ordering;
    match width.cmp(&1) {
        Ordering::Less => panic!("Invalid bit width!"),
        Ordering::Equal => D::nil(),
        Ordering::Greater => {
            D::text(format!("[{}:0]", width - 1)).append(D::space())
        }
    }
}

//  Generates a string wirename for the provided Port object
// pub fn port_wire_id(src: &structure::NodeData) -> D {
//     // src.
//     // let port_doc: fn(&'a ast::Port) -> D<'a> = |p| match p {
//     //     ast::Port::Comp { component, port } => D::text(component)
//     //         .append(D::text("_"))
//     //         .append(D::text(port)),
//     //     ast::Port::This { port } => D::text(port),
//     // };
//     // port_doc(&wire.src)
//     //     .append(D::text("$"))
//     //     .append(port_doc(&wire.dest))
// }

//==========================================
//        Subcomponent Instance Functions
//==========================================
fn subcomponent_instances<'a>(comp: &component::Component) -> D<'a> {
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
                ));
            Some(doc)
        } else {
            None
        }
    });
    D::intersperse(subs, D::line().append(D::line()))
}

fn subcomponent_sig<'a>(id: &ast::Id, structure: &ast::Structure) -> D<'a> {
    use ast::Structure;
    let (name, params): (&str, &[u64]) = match structure {
        Structure::Decl { data } => (&data.component, &[]),
        Structure::Std { data } => (&data.instance.name, &data.instance.params),
        Structure::Wire { .. } => {
            panic!("Shouldn't have a wire in the structure graph")
        }
    };

    D::text(name.to_string())
        .append(D::line())
        .append(D::text("#"))
        .append(parens(
            D::intersperse(
                params.iter().map(|param| D::text(param.to_string())),
                D::text(",").append(D::line()),
            )
            .group(),
        ))
        .append(D::line())
        .append(D::text(id.to_string()))
        .group()
}

fn signature_connections<'a>(
    name: &ast::Id,
    sig: &ast::Signature,
    comp: &component::Component,
    idx: NodeIndex,
) -> D<'a> {
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
                        .append(portdef.name.to_string())
                        .append(parens(D::text(wire_name)))
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
                    .append(portdef.name.to_string())
                    .append(parens(D::text(wire_name))),
            )
        } else {
            None
        }
        // let doc = comp
        //     .structure
        //     .connected_to(idx, portdef.name.to_string())
        //     .map(move |(dest_node, edge)| {
        //                             // .append(D::text(edge_data.src.to_string()))
        //         // .append(parens(D::text(edge_data.dest.to_string())))
        //     });
    });

    // let inputs = sig
    //     .inputs
    //     .iter()
    //     .chain(sig.outputs.iter())
    //     .map(|pd| D::text(".").append(pd.name.to_string()));
    D::intersperse(incoming.chain(outgoing), D::text(",").append(D::line()))
}

// Intermediate data structures for string formatting
// #[derive(Clone, Debug)]
// pub struct RtlInst<'a> {
//     pub comp_name: &'a String,
//     pub id: &'a String,
//     pub params: Vec<u64>,
//     pub ports: HashMap<&'a String, String>, // Maps Port names to wires
// }

// fn instances(c: &Context) -> RcDoc<'_> {
//     let decls = c
//         .toplevel
//         .get_decl()
//         .into_iter()
//         .map(|decl| component_to_inst(&decl, c))
//         .map(inst_to_string);
//     let prims = c
//         .toplevel
//         .get_std()
//         .into_iter()
//         .map(|prim| prim_to_inst(&prim, c))
//         .map(inst_to_string);
//     RcDoc::intersperse(decls, RcDoc::line().append(RcDoc::line()))
//         .append(RcDoc::line())
//         .append(RcDoc::intersperse(
//             prims,
//             RcDoc::line().append(RcDoc::line()),
//         ))
// }

// fn component_to_inst<'a>(inst: &'a Decl, c: &'a Context) -> RtlInst<'a> {
//     let comp = c.definitions.get(&inst.component).unwrap();
//     let wires = c.toplevel.get_wires();
//     let mut port_map: HashMap<&String, String> = HashMap::new();
//     for w in wires {
//         if let Port::Comp { component, port } = &w.src {
//             if *component == inst.name {
//                 // Note that all port_wire_ids are currently based off the source
//                 port_map.insert(port, pretty_print(port_wire_id(&w.src)));
//             }
//         }
//         if let Port::Comp { component, port } = &w.dest {
//             if component.clone() == inst.name {
//                 // Note that all port_wire_ids are currently based off the source
//                 port_map.insert(port, pretty_print(port_wire_id(&w.src)));
//             }
//         }
//     }
//     // Fill up any remaining ports with empty string
//     for Portdef { name, .. } in comp
//         .signature
//         .inputs
//         .iter()
//         .chain(comp.signature.outputs.iter())
//     {
//         if !port_map.contains_key(name) {
//             port_map.insert(name, "".to_string());
//         }
//     }
//     RtlInst {
//         comp_name: &comp.name,
//         id: &inst.name,
//         params: vec![],
//         ports: port_map,
//     }
// }

// fn prim_to_inst<'a>(inst: &'a Std, c: &'a Context) -> RtlInst<'a> {
//     let prim = c.library.get(&inst.instance.name).unwrap();
//     let wires = c.toplevel.get_wires();
//     let mut port_map: HashMap<&String, String> = HashMap::new();
//     for w in wires {
//         if let Port::Comp { component, port } = &w.src {
//             if component.clone() == inst.name {
//                 // Note that all port_wire_ids are currently based off the source
//                 port_map.insert(port, pretty_print(port_wire_id(&w.src)));
//             }
//         }
//         if let Port::Comp { component, port } = &w.dest {
//             if component.clone() == inst.name {
//                 // Note that all port_wire_ids are currently based off the source
//                 port_map.insert(port, pretty_print(port_wire_id(&w.src)));
//             }
//         }
//     }
//     // Fill up any remaining ports with empty string
//     for ParamPortdef { name, .. } in
//         prim.signature.inputs().chain(prim.signature.outputs())
//     {
//         if !port_map.contains_key(name) {
//             port_map.insert(name, "".to_string());
//         }
//     }
//     RtlInst {
//         comp_name: &prim.name,
//         id: &inst.name,
//         params: inst.instance.params.clone(),
//         ports: port_map,
//     }
// }

// pub fn inst_to_string(inst: RtlInst) -> RcDoc<'_> {
//     let ports = inst.ports.into_iter().map(|(port, wire)| {
//         RcDoc::text(".")
//             .append(RcDoc::text(port))
//             .append(RcDoc::text("("))
//             .append(RcDoc::text(wire))
//             .append(RcDoc::text(")"))
//     });
//     let params = inst.params.iter().map(|p| RcDoc::text(p.to_string()));
//     RcDoc::text(inst.comp_name.clone())
//         .append(RcDoc::space())
//         .append(RcDoc::text("#("))
//         .append(RcDoc::intersperse(
//             params,
//             RcDoc::text(",").append(RcDoc::space()).group(),
//         ))
//         .append(RcDoc::text(")"))
//         .append(RcDoc::space())
//         .append(RcDoc::text(inst.id.clone()))
//         .append(RcDoc::space())
//         .append(RcDoc::text("("))
//         .append(
//             RcDoc::line()
//                 .append(RcDoc::intersperse(
//                     ports,
//                     RcDoc::text(",").append(RcDoc::line()),
//                 ))
//                 .nest(4),
//         )
//         .append(RcDoc::line())
//         .append(RcDoc::text(");"))
// }
