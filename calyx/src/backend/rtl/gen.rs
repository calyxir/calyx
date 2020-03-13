use crate::backend::traits::{Backend, Emitable};
use crate::context;
use crate::errors;
use crate::lang::{ast, component};
use pretty::RcDoc as R;

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

    fn to_string(ctx: &context::Context) -> std::string::String {
        let mut w = Vec::new();
        let prog: ast::NamespaceDef = ctx.clone().into();
        let docs = prog.components.iter().map(|c| {
            let comp = ctx.get_component(&c.name).unwrap();
            c.doc(&comp)
        });
        let doc = R::intersperse(docs, R::line());
        doc.render(100, &mut w).unwrap();
        String::from_utf8(w).unwrap()
    }
}

impl Emitable for ast::ComponentDef {
    fn doc(&self, comp: &component::Component) -> R {
        use crate::lang::pretty_print::parens;
        R::text("// Component Signature")
            .append(R::line())
            .append(R::text("module"))
            .append(R::space())
            .append(R::text(&self.name))
            .append(R::line())
            .append(parens(
                R::line()
                    .append(self.signature.doc(&comp))
                    .nest(4)
                    .append(R::line()),
            ))
            .append(R::text(";"))
            .append(R::line())
            .append(R::line())
            .append(R::text("// Wire declarations"))
            .append(R::line())
            .append(wire_declarations(&self.structure, &comp))
            // .append(RcDoc::line())
            // .append(RcDoc::line())
            // .append(RcDoc::text("// Subcomponent Instances"))
            // .append(RcDoc::line())
            // .append(instances(c))
            // .append(RcDoc::line())
            .append(R::line())
            .append(R::text("endmodule"))
    }
}

impl Emitable for ast::Signature {
    fn doc(&self, comp: &component::Component) -> R {
        let inputs = self.inputs.iter().map(|pd| {
            R::text("input").append(R::space()).append(pd.doc(&comp))
        });
        let outputs = self.outputs.iter().map(|pd| {
            R::text("output").append(R::space()).append(pd.doc(&comp))
        });
        R::intersperse(inputs.chain(outputs), R::text(",").append(R::line()))
    }
}

impl Emitable for ast::Portdef {
    fn doc(&self, ctx: &component::Component) -> R {
        // XXX(Sam) why don't we use wires?
        R::text("wire").append(R::space()).append(&self.name)
    }
}

// //==========================================
// //        Wire Declaration Functions
// //==========================================
fn wire_declarations<'a>(
    structure: &'a [ast::Structure],
    comp: &component::Component,
) -> R<'a> {
    let wire_names = structure
        .iter()
        // .unique_by(|wire| &wire.src) why is this here?
        .filter_map(|st| {
            if let ast::Structure::Wire { data } = st {
                wire_string(&data, comp)
            } else {
                None
            }
        });
    R::intersperse(wire_names, R::line())
}

// XXX(sam) get rid of use of unwrap
fn wire_string<'a>(
    wire: &'a ast::Wire,
    comp: &component::Component,
) -> Option<R<'a>> {
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
            R::text("wire")
                .append(R::space())
                .append(bit_width(width))
                .append(port_wire_id(&wire.src))
                .append(R::text(";")),
        ),
        ast::Port::This { .. } => None,
    }
}

pub fn bit_width<'a>(width: u64) -> R<'a> {
    if width < 1 {
        panic!("Invalid bit width!");
    } else if width == 1 {
        R::nil()
    } else {
        R::text(format!("[{}:0]", width - 1)).append(R::space())
    }
}

///  Generates a string wirename for the provided Port object
pub fn port_wire_id(p: &ast::Port) -> R {
    match p {
        ast::Port::Comp { component, port } => R::text(component)
            .append(R::text("_"))
            .append(R::text(port)),
        ast::Port::This { port } => R::text(port),
    }
}

// fn pretty_print(doc: RcDoc) -> String {
//     let mut w = Vec::new();
//     doc.render(80, &mut w).unwrap();
//     String::from_utf8(w).unwrap()
// }

// #[allow(unused)]
// pub fn to_verilog(c: &Context) -> String {
//     let doc: RcDoc = RcDoc::text("// Component Signature")
//         .append(RcDoc::line())
//         .append(RcDoc::text("module"))
//         .append(RcDoc::space())
//         .append(RcDoc::text(c.toplevel.name.clone()))
//         .append(RcDoc::line())
//         .append(RcDoc::text("("))
//         .append(component_io(&c.toplevel).nest(4))
//         .append(RcDoc::line())
//         .append(RcDoc::text(");"))
//         .append(RcDoc::line())
//         .append(RcDoc::line())
//         .append(RcDoc::text("// Wire declarations"))
//         .append(RcDoc::line())
//         .append(wire_declarations(c))
//         .append(RcDoc::line())
//         .append(RcDoc::line())
//         .append(RcDoc::text("// Subcomponent Instances"))
//         .append(RcDoc::line())
//         .append(instances(c))
//         .append(RcDoc::line())
//         .append(RcDoc::line())
//         .append(RcDoc::text("endmodule"));
//     pretty_print(doc)
// }

// //==========================================
// //        Component I/O Functions
// //==========================================

// /**
//  * Returns a string with the list of all of a component's i/o pins
//  */
// #[allow(unused)]
// pub fn component_io(c: &ComponentDef) -> RcDoc<'_> {
//     let mut inputs = c
//         .signature
//         .inputs
//         .iter()
//         .map(|pd| in_port(pd.width, &pd.name));
//     let mut outputs = c
//         .signature
//         .outputs
//         .iter()
//         .map(|pd| out_port(pd.width, &pd.name));
//     RcDoc::line().append(RcDoc::intersperse(
//         inputs.chain(outputs),
//         RcDoc::text(",").append(RcDoc::line()),
//     ))
// }

// pub fn in_port(width: u64, name: &str) -> RcDoc<'_> {
//     RcDoc::text("input")
//         .append(RcDoc::space())
//         .append(RcDoc::text("logic"))
//         .append(RcDoc::space())
//         .append(bit_width(width))
//         .append(RcDoc::text(name))
// }

// pub fn out_port(width: u64, name: &str) -> RcDoc<'_> {
//     RcDoc::text("output")
//         .append(RcDoc::space())
//         .append(RcDoc::text("logic"))
//         .append(RcDoc::space())
//         .append(bit_width(width))
//         .append(RcDoc::text(name))
// }

// pub fn bit_width<'a>(width: u64) -> RcDoc<'a> {
//     if width < 1 {
//         panic!("Invalid bit width!");
//     } else if width == 1 {
//         RcDoc::text("".to_string())
//     } else {
//         RcDoc::text(format!("[{}:0]", width - 1)).append(RcDoc::space())
//     }
// }
// //==========================================
// //        Subcomponent Instance Functions
// //==========================================
// // Intermediate data structures for string formatting
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
