use super::group_passthrough::GroupPassthrough;
use super::interfacing::Interfacing;
use crate::backend::traits::{Backend, Emitable};
use crate::errors::Error;
use crate::lang::library::ast as lib;
use crate::lang::pretty_print::{display, PrettyHelper};
use crate::lang::structure::Node;
use crate::lang::{
    ast, ast::Control, ast::Structure, colors, colors::ColorHelper, component,
    context, structure, structure::EdgeData, structure::NodeData,
};
use crate::passes::visitor::Visitor;
use itertools::Itertools;
use lib::Implementation;
use petgraph::graph::NodeIndex;
use pretty::termcolor::ColorSpec;
use pretty::RcDoc as D;
use std::cmp::Ordering;
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

/// Returns `Ok` if every input on each subcomponent has at most
/// one incoming wire. This is to ensure that we never generate
/// Verilog that has two drivers for a single input.
fn validate_structure(comp: &component::Component) -> Result<(), Error> {
    let has_multiple_inputs = comp
        .structure
        .nodes()
        // get all incoming ports
        .map(|(idx, node)| node.in_ports().map(move |pd| (idx, pd)))
        .flatten()
        // find if port had > 1 incoming edge
        .any(|(idx, port)| {
            comp.structure
                .incoming_to_port(idx, port.to_string())
                .count()
                > 1
        });
    if !has_multiple_inputs {
        Ok(())
    } else {
        Err(Error::MalformedStructure(
            "A port had multiple inputs.".to_string(),
        ))
    }
}

/// Returns `Ok` if the control for `comp` is either a single `enable`
/// or `empty`.
fn validate_control(comp: &component::Component) -> Result<(), Error> {
    match &comp.control {
        Control::Enable { .. } => Ok(()),
        _ => Err(Error::MalformedControl(
            "Must be a single enable statement".to_string(),
        )),
    }
}

impl Backend for VerilogBackend {
    fn name() -> &'static str {
        "verilog"
    }

    fn validate(ctx: &context::Context) -> Result<(), Error> {
        ctx.definitions_iter(|_, comp| {
            validate_structure(comp)?;
            validate_control(comp)
        })
    }

    fn emit<W: Write>(ctx: &context::Context, file: W) -> Result<(), Error> {
        // add interfacing to main module
        Interfacing::do_pass_default(&ctx)?;
        GroupPassthrough::do_pass_default(&ctx)?;

        use crate::lang::pretty_print::PrettyPrint;
        ctx.pretty_print();

        let prog: ast::NamespaceDef = ctx.clone().into();

        // build Vec of tuples first so that `comps` lifetime is longer than
        // `docs` lifetime
        let comps: Vec<(&ast::ComponentDef, component::Component)> = prog
            .components
            .iter()
            .map(|cd| (cd, ctx.get_component(&cd.name).unwrap()))
            .collect();

        let docs = comps
            .iter()
            .map(|(cd, comp)| cd.doc(&comp))
            .collect::<Result<Vec<_>, _>>()?;
        let prims = primitive_implemenations(&prog, ctx)?;
        display(
            colors::comment(D::text("/* verilator lint_off PINMISSING */"))
                .append(D::line())
                .append(prims)
                .append(D::line())
                .append(D::line())
                .append(D::intersperse(docs, D::line())),
            Some(file),
        );
        Ok(())
    }
}

/// Collects all of the Verilog implementations specified in the library
/// file.
fn primitive_implemenations<'a>(
    prog: &ast::NamespaceDef,
    context: &context::Context,
) -> Result<D<'a, ColorSpec>, Error> {
    let docs = prog
        .components
        .iter()
        .map(|c| c.structure.iter())
        .flatten()
        .filter_map(|s| match s {
            Structure::Std { data } => Some(&data.instance.name),
            _ => None,
        })
        .unique()
        .map(|name| {
            context.library_context.definitions[&name]
                .implementation
                .iter()
                .find_map(|im| match im {
                    Implementation::Verilog { data } => {
                        Some(D::text(data.code.to_string()))
                    }
                })
                .ok_or_else(|| {
                    Error::MissingImplementation("Verilog", name.clone())
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(D::intersperse(docs, D::line().append(D::line())))
}

impl Emitable for ast::ComponentDef {
    fn doc<'a>(
        &self,
        comp: &component::Component,
    ) -> Result<D<'a, ColorSpec>, Error> {
        let structure = D::nil()
            .append(D::space())
            .append(self.name.to_string())
            .append(D::line())
            .append(
                D::line()
                    .append(self.signature.doc(&comp)?)
                    .nest(4)
                    .append(D::line())
                    .parens(),
            )
            .append(";")
            .append(D::line())
            .append(D::line())
            .append(colors::comment(D::text("// Structure wire declarations")))
            .append(D::line())
            .append(wire_declarations(&comp)?)
            .append(D::line())
            .append(D::line())
            .append(colors::comment(D::text("// Subcomponent Instances")))
            .append(D::line())
            .append(subcomponent_instances(&comp));
        let inner = structure;

        Ok(colors::comment(D::text("// Component Signature"))
            .append(D::line())
            .append(D::text("module").define_color())
            .append(inner.nest(2))
            .append(D::line())
            .append(D::text("endmodule").define_color())
            .append(D::space())
            .append(colors::comment(D::text(format!(
                "// end {}",
                self.name.to_string()
            )))))
    }
}

impl Emitable for ast::Signature {
    fn doc<'a>(
        &self,
        comp: &component::Component,
    ) -> Result<D<'a, ColorSpec>, Error> {
        let inputs = self
            .inputs
            .iter()
            .map(|pd| {
                Ok(D::text("input")
                    .port_color()
                    .append(D::space())
                    .append(pd.doc(&comp)?))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let outputs = self
            .outputs
            .iter()
            .map(|pd| {
                Ok(D::text("output")
                    .port_color()
                    .append(D::space())
                    .append(pd.doc(&comp)?))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(D::intersperse(
            inputs.into_iter().chain(outputs.into_iter()),
            D::text(",").append(D::line()),
        ))
    }
}

impl Emitable for ast::Portdef {
    fn doc<'a>(
        &self,
        _ctx: &component::Component,
    ) -> Result<D<'a, ColorSpec>, Error> {
        // XXX(sam) why would we use logic over wires?
        Ok(D::text("wire")
            .keyword_color()
            .append(D::space())
            .append(bit_width(self.width)?)
            .append(D::space())
            .append(self.name.to_string()))
    }
}

//==========================================
//        Wire Declaration Functions
//==========================================
/// Generate all the wire declarations for `comp`
fn wire_declarations<'a>(
    comp: &component::Component,
) -> Result<D<'a, ColorSpec>, Error> {
    // declare a wire for each instance, not input or output
    // because they already have wires defined
    // in the module signature.

    // We only make a wire once for each instance
    // output. This is because Verilog wires do not
    // correspond exactly with Futil wires.
    let structure: &structure::StructureGraph = &comp.structure;
    let wires = structure
        .nodes()
        .filter_map(|(idx, node)| {
            match node.data {
                NodeData::Port => None,
                NodeData::Instance(..) => Some(
                    structure
                        .outgoing_from_node(idx)
                        .map(|(_n, e)| {
                            D::text("wire")
                                .keyword_color()
                                .append(D::space())
                                .append(bit_width(e.width).unwrap())
                                .append(
                                    D::text(format!(
                                        "{}${}",
                                        node.name.as_ref(),
                                        e.src
                                    ))
                                    .ident_color(),
                                )
                                .append(";")
                        })
                        .collect::<Vec<_>>(),
                ),
                NodeData::Group(..) => {
                    let out =
                        structure.outgoing_from_node(idx).map(|(_n, e)| {
                            D::text("wire")
                                .keyword_color()
                                .append(D::space())
                                .append(bit_width(e.width).unwrap())
                                .append(
                                    D::text(format!(
                                        "{}${}-out",
                                        node.name.as_ref(),
                                        e.src
                                    ))
                                    .ident_color(),
                                )
                                .append(";")
                        });
                    let incoming =
                        structure.incoming_to_node(idx).map(|(_n, e)| {
                            D::text("wire")
                                .keyword_color()
                                .append(D::space())
                                .append(bit_width(e.width).unwrap())
                                .append(
                                    D::text(format!(
                                        "{}${}-in",
                                        node.name.as_ref(),
                                        e.src
                                    ))
                                    .ident_color(),
                                )
                                .append(";")
                        });
                    Some(out.chain(incoming).collect::<Vec<_>>())
                }
            }
            // .collect::<Result<Vec<_>, Error>>()
        })
        .flatten();
    Ok(D::intersperse(wires, D::line()))
}

/// Generates a verilog wire declaration for the provided
/// node and port. Returns `None` if the port is never used.
fn wire_string<'a>(
    portdef: &ast::Portdef,
    structure: &structure::StructureGraph,
    idx: NodeIndex,
    name: &ast::Id,
) -> Result<Option<D<'a, ColorSpec>>, Error> {
    // don't want to declare a wire for ports, they already have wires declared
    if let NodeData::Port = structure.get(idx).data {
        return Ok(None);
    };

    // build comment of the destinations of each wire declaration
    let dests: Vec<D<ColorSpec>> = structure
        .outgoing_from_port(idx, portdef.name.to_string())
        .filter_map(|(node, edge)| match node.data {
            NodeData::Instance(..) | NodeData::Group(..) => Some(
                D::text(node.name.to_string())
                    .append(D::text(" @ "))
                    .append(edge.dest.to_string()),
            ),
            NodeData::Port => None,
        })
        .collect();

    // only declare the wire if it is actually used
    if !dests.is_empty() {
        let dest_comment = colors::comment(
            D::text("// ").append(D::intersperse(dests, D::text(", "))),
        );
        let wire_name =
            format!("{}${}", &name.to_string(), &portdef.name.to_string());
        Ok(Some(
            D::text("wire")
                .keyword_color()
                .append(D::space())
                .append(bit_width(portdef.width)?)
                .append(D::text(wire_name).ident_color())
                .append(";")
                .append(D::space())
                .append(dest_comment),
        ))
    } else {
        Ok(None)
    }
}

/// Uses Verilog assign to connect the two ends of `edge`.
fn alias<'a>(src_string: String, dest_string: String) -> D<'a, ColorSpec> {
    D::text("assign")
        .keyword_color()
        .append(D::space())
        .append(src_string)
        .append(" = ")
        .append(dest_string)
        .append(";")
}

/// Turn u64 into a formatted Verilog bitwidth specifier.
pub fn bit_width<'a>(width: u64) -> Result<D<'a, ColorSpec>, Error> {
    match width.cmp(&1) {
        Ordering::Less => {
            Err(Error::Misc(format!("{} is an impossible bitwidth", width)))
        }
        Ordering::Equal => Ok(D::nil()),
        Ordering::Greater => {
            Ok(D::text(format!("[{}:0]", width - 1)).append(D::space()))
        }
    }
}

//==========================================
//        Subcomponent Instance Functions
//==========================================
/// Generate Verilog for each subcomponent instanstiation and
/// wire up all the ports.
fn subcomponent_instances<'a>(comp: &component::Component) -> D<'a, ColorSpec> {
    let subs = comp.structure.nodes().map(|(idx, node)| match node.data {
        NodeData::Instance(structure) => {
            subcomponent_sig(&node.name, &structure)
                .append(D::space())
                .append(
                    D::line()
                        .append(signature_connections(
                            &node.name,
                            &node.signature,
                            &comp,
                            idx,
                        ))
                        .nest(4)
                        .append(D::line())
                        .parens(),
                )
                .append(";")
        }
        NodeData::Group(..) => {
            let doc =
                comp.structure.outgoing_from_node(idx).filter_map(|(n, e)| {
                    match n.data {
                        NodeData::Port => None,
                        NodeData::Group(..) | NodeData::Instance(..) => {
                            Some(alias(
                                format!("{}${}", node.name.as_ref(), e.src),
                                format!("{}${}", n.name.as_ref(), e.dest),
                            ))
                        }
                    }
                });
            colors::comment(D::text(format!(
                "// Group {}",
                node.name.to_string()
            )))
            .append(D::line())
            .append(D::intersperse(doc, D::line()))
        }
        NodeData::Port => {
            let doc = comp.structure.outgoing_from_node(idx).map(|(n, e)| {
                match n.data {
                    NodeData::Port => {
                        alias(e.src.to_string(), e.dest.to_string())
                    }
                    NodeData::Group(..) | NodeData::Instance(..) => alias(
                        e.src.to_string(),
                        format!("{}${}", n.name.as_ref(), e.dest),
                    ),
                }
            });
            colors::comment(D::text("// Port connections".to_string()))
                .append(D::line())
                .append(D::intersperse(doc, D::line()))
        }
    });
    D::intersperse(subs, D::line().append(D::line()))
}

/// Generates just the Verilog instanstiation code, but none
/// of the connections.
fn subcomponent_sig<'a>(
    id: &ast::Id,
    structure: &ast::Structure,
) -> D<'a, ColorSpec> {
    let (name, params): (&ast::Id, &[u64]) = match structure {
        Structure::Decl { data } => (&data.component, &[]),
        Structure::Std { data } => (&data.instance.name, &data.instance.params),
        Structure::Wire { .. } => {
            panic!("Shouldn't have a wire in the structure graph")
        }
        Structure::Group { .. } => {
            unimplemented!("Code generation for groups.")
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

/// Generates Verilog for connection ports to wires.
fn signature_connections<'a>(
    name: &ast::Id,
    sig: &ast::Signature,
    comp: &component::Component,
    idx: NodeIndex,
) -> D<'a, ColorSpec> {
    // wire up all the incoming edges
    let incoming = sig
        .inputs
        .iter()
        .map(|portdef| {
            comp.structure
                .incoming_to_port(idx, portdef.name.to_string())
                .map(move |(src, edge)| {
                    let wire_name = match src.data {
                        NodeData::Port => src.name.to_string(),
                        NodeData::Group(..) | NodeData::Instance(..) => {
                            format!("{}${}", src.name.to_string(), &edge.src)
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

    // we need
    //   x @ out -> y @ in
    //   x @ out -> z @ in
    //   x @ out -> this @ ready
    // to generate the Verilog connections:
    //   .out(x$out)
    //   .out(ready)
    // The second outgoing connection in Futil should not have a
    // statement in Verilog because it would look like
    //   .out(x$out)
    // which is identical to the first Verilog statement and thus redundant.
    // That is what the `unique` is doing below.
    let outgoing = sig
        .outputs
        .iter()
        .map(|portdef| {
            comp.structure
                .outgoing_from_port(idx, portdef.name.to_string())
                .map(move |(dest, edge)| {
                    let wire_name = match dest.data {
                        NodeData::Port => dest.name.to_string(),
                        NodeData::Group(..) | NodeData::Instance(..) => {
                            format!("{}${}", dest.name.to_string(), &edge.dest)
                        }
                    };
                    // return tuple so that we can check uniqueness
                    (portdef.name.to_string(), wire_name)
                })
                // call unique so that we only get connection per outgoing wire
                .unique()
                .map(|(port, wire)| {
                    D::text(".")
                        .append((D::text(port)).port_color())
                        .append(D::text(wire).ident_color().parens())
                })
        })
        .flatten();

    D::intersperse(incoming.chain(outgoing), D::text(",").append(D::line()))
}
