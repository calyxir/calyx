use crate::backend::traits::Backend;
use crate::context;
use crate::errors;
use crate::lang::ast;

pub struct RtlBackend {}

impl Backend for RtlBackend {
    fn validate(prog: &context::Context) -> Result<(), errors::Error> {
        prog.definitions_map(|_name, comp| {
            use ast::Control;
            match &comp.control {
                Control::Seq { data } => {
                    for con in &data.stmts {
                        match con {
                            Control::Enable { .. } => (),
                            _ => return Err(errors::Error::MalformedControl),
                        }
                    }
                    Ok(())
                }
                _ => Err(errors::Error::MalformedControl),
            }
        })
    }

    fn to_string(_: &context::Context) -> std::string::String {
        "unimplemented backend!\n".to_string()
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
// //        Wire Declaration Functions
// //==========================================
// fn wire_declarations(c: &Context) -> RcDoc<'_> {
//     let wire_names = c
//         .toplevel
//         .get_wires()
//         .into_iter()
//         .unique_by(|wire| &wire.src)
//         .filter_map(|wire| wire_string(&wire, c));
//     RcDoc::intersperse(wire_names, RcDoc::line())
// }

// fn wire_string<'a>(wire: &'a Wire, c: &Context) -> Option<RcDoc<'a>> {
//     None
//     // let width = Context::port_width(&wire.src, &c.toplevel, c);
//     // match &wire.src {
//     //     Port::Comp { .. } => Some(
//     //         RcDoc::text("logic")
//     //             .append(RcDoc::space())
//     //             .append(bit_width(width))
//     //             .append(port_wire_id(&wire.src))
//     //             .append(RcDoc::text(";")),
//     //     ),
//     //     Port::This { .. } => None,
//     // }
// }

// /**
//  * Generates a string wirename for the provided Port object
//  */
// pub fn port_wire_id(p: &Port) -> RcDoc<'_> {
//     match p {
//         Port::Comp { component, port } => RcDoc::text(component)
//             .append(RcDoc::text("_"))
//             .append(RcDoc::text(port)),
//         Port::This { port } => RcDoc::text(port),
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
