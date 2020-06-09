use crate::backend::traits::{Backend, Emitable};
use crate::errors::{Error, Result};
use crate::frontend::{
    colors,
    colors::ColorHelper,
    pretty_print::{display, PrettyHelper},
};
use crate::lang::library::ast as lib;
use crate::lang::{
    ast,
    ast::{Cell, Control, Port},
    component, context,
    structure::EdgeData,
    structure::NodeData,
};
use itertools::Itertools;
use lib::Implementation;
use petgraph::graph::NodeIndex;
use pretty::termcolor::ColorSpec;
use pretty::RcDoc;
use std::cmp::Ordering;
use std::io::Write;

type D<'a> = RcDoc<'a, ColorSpec>;

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
fn validate_structure(comp: &component::Component) -> Result<()> {
    // let has_multiple_inputs = comp
    //     .structure
    //     .instances()
    //     // get all incoming ports
    //     .map(|(idx, node)| node.in_ports().map(move |pd| (idx, pd)))
    //     .flatten()
    //     // find if port had > 1 incoming edge
    //     .any(|(idx, port)| {
    //         comp.structure
    //             .connected_incoming(idx, port.to_string())
    //             .count()
    //             > 1
    //     });
    // if !has_multiple_inputs {
    //     Ok(())
    // } else {
    //     Err(Error::MalformedStructure(
    //         "A port had multiple inputs.".to_string(),
    //     ))
    // }
    Ok(())
}

/// Returns `Ok` if the control for `comp` is either a single `enable`
/// or `empty`.
fn validate_control(comp: &component::Component) -> Result<()> {
    match &comp.control {
        Control::Empty { .. } => Ok(()),
        _ => Err(Error::MalformedControl(
            "Must either be a single enable or an empty statement".to_string(),
        )),
    }
}

impl Backend for VerilogBackend {
    fn name() -> &'static str {
        "verilog"
    }

    fn validate(ctx: &context::Context) -> Result<()> {
        ctx.definitions_iter(|_, comp| {
            validate_structure(comp)?;
            validate_control(comp)
        })
    }

    fn emit<W: Write>(ctx: &context::Context, file: W) -> Result<()> {
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
            .collect::<Result<Vec<_>>>()?;
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
) -> Result<D<'a>> {
    let docs = prog
        .components
        .iter()
        .map(|c| c.cells.iter())
        .flatten()
        .filter_map(|s| match s {
            Cell::Prim { data } => Some(&data.instance.name),
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
        .collect::<Result<Vec<_>>>()?;
    Ok(D::intersperse(docs, D::line().append(D::line())))
}

impl Emitable for ast::ComponentDef {
    fn doc<'a>(&self, comp: &component::Component) -> Result<D<'a>> {
        let structure = D::nil()
            .append(D::space())
            .append(self.name.to_string())
            .append(self.signature.doc(&comp)?)
            .append(";")
            .append(D::line())
            .append(D::line())
            .append(colors::comment(D::text("// Structure wire declarations")))
            .append(D::line())
            .append(wire_declarations(&comp)?)
            .append(D::line())
            .append(D::line())
            .append(colors::comment(D::text("// Input / output connections")))
            .append(D::line())
            .append(io_connections(&comp))
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
    fn doc<'a>(&self, comp: &component::Component) -> Result<D<'a>> {
        let mut inputs = self
            .inputs
            .iter()
            .map(|pd| {
                Ok(D::text("input")
                    .keyword_color()
                    .append(D::space())
                    .append(pd.doc(&comp)?))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut outputs = self
            .outputs
            .iter()
            .map(|pd| {
                Ok(D::text("output")
                    .keyword_color()
                    .append(D::space())
                    .append(pd.doc(&comp)?))
            })
            .collect::<Result<Vec<_>>>()?;
        inputs.append(&mut outputs);
        if inputs.is_empty() {
            Ok(D::space().append(D::nil().parens()))
        } else {
            let doc = D::intersperse(
                inputs.into_iter().chain(outputs.into_iter()),
                D::text(",").append(D::line()),
            );
            Ok(D::space().append(
                D::line().append(doc).nest(4).append(D::line()).parens(),
            ))
        }
    }
}

impl Emitable for ast::Portdef {
    fn doc<'a>(&self, _ctx: &component::Component) -> Result<D<'a>> {
        // XXX(sam) why would we use logic over wires?
        Ok(D::text("wire")
            .keyword_color()
            .append(D::space())
            .append(bitwidth(self.width)?)
            .append(self.name.to_string()))
    }
}

//==========================================
//        Wire Declaration Functions
//==========================================
/// Generate all the wire declarations for `comp`
fn wire_declarations<'a>(comp: &component::Component) -> Result<D<'a>> {
    let wires: Vec<_> = comp
        .structure
        .edges()
        .unique_by(|(_idx, data)| &data.src)
        .map(|(_idx, data)| {
            Ok(D::text("wire")
                .keyword_color()
                .append(D::space())
                .append(bitwidth(data.width)?)
                .append(wire_id(data))
                .append(";"))
        })
        .collect::<Result<_>>()?;
    Ok(D::intersperse(wires, D::line()))
}

/// Creates the identifier for a wire instance from an &EdgeData
fn wire_id<'a>(data: &EdgeData) -> D<'a> {
    match &data.src {
        Port::This { port } => D::text(port.to_string()),
        Port::Comp { component, port } => {
            D::text(format!("{}_{}", component.to_string(), port.to_string()))
        }
        Port::Hole { .. } => unreachable!(),
    }
}

/// Uses Verilog assign to connect the two ends of `edge`.
fn alias<'a>(edge: &EdgeData) -> D<'a> {
    D::text("assign")
        .keyword_color()
        .append(D::space())
        .append(edge.dest.port_name().to_string())
        .append(" = ")
        .append(wire_id(edge))
        .append(";")
}

/// Turn u64 into a formatted Verilog bitwidth specifier.
pub fn bitwidth<'a>(width: u64) -> Result<D<'a>> {
    match width.cmp(&1) {
        Ordering::Less => unreachable!(),
        Ordering::Equal => Ok(D::nil()),
        Ordering::Greater => {
            Ok(D::text(format!("[{}:0]", width - 1)).append(D::space()))
        }
    }
}

//==========================================
//        IO Connection Functions
//==========================================
fn io_connections<'a>(comp: &component::Component) -> D<'a> {
    let doc = comp
        .structure
        .incoming_to_node(comp.structure.this_idx())
        .map(|(_, edge)| alias(edge));

    D::intersperse(doc, D::line())
}

//==========================================
//        Subcomponent Instance Functions
//==========================================
/// Generate Verilog for each subcomponent instanstiation and
/// wire up all the ports.
fn subcomponent_instances<'a>(comp: &component::Component) -> D<'a> {
    let doc = comp
        .structure
        .nodes()
        .filter_map(|(idx, node)| {
            if let NodeData::Cell(cell) = &node.data {
                Some((node, idx, cell))
            } else {
                None
            }
        })
        .map(|(node, idx, cell)| {
            subcomponent_sig(&node.name, &cell)
                .append(D::space())
                .append(
                    D::line()
                        .append(signature_connections(
                            &node.signature,
                            &comp,
                            idx,
                        ))
                        .nest(4)
                        .append(D::line())
                        .parens(),
                )
                .append(";")
        });
    D::intersperse(doc, D::line().append(D::line()))
}

/// Generates just the Verilog instanstiation code, but none
/// of the connections.
fn subcomponent_sig<'a>(id: &ast::Id, structure: &ast::Cell) -> D<'a> {
    let (name, params): (&ast::Id, &[u64]) = match structure {
        Cell::Decl { data } => (&data.component, &[]),
        Cell::Prim { data } => (&data.instance.name, &data.instance.params),
    };

    D::text(name.to_string())
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
    sig: &ast::Signature,
    comp: &component::Component,
    idx: NodeIndex,
) -> D<'a> {
    // wire up all the incoming edges
    let incoming = sig
        .inputs
        .iter()
        .map(|portdef| {
            comp.structure
                .incoming_to_port(idx, portdef.name.to_string())
                .map(move |(src, edge)| {
                    let wire_name = wire_id(edge);
                    D::text(".")
                        .append(D::text(portdef.name.to_string()))
                        .append(wire_name.parens())
                })
        })
        .flatten();

    // we need
    //   x.out -> y.in
    //   x.out -> z.in
    //   x.out -> this.ready
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
                // find inuse edges that are coming out of this port
                .outgoing_from_port(idx, portdef.name.to_string())
                // call unique so that we only get connection per outgoing wire
                .unique_by(|(_, edge)| &edge.src)
                .map(move |(_, edge)| {
                    D::text(".")
                        .append(D::text(portdef.name.to_string()))
                        .append(wire_id(edge).parens())
                })
        })
        .flatten();

    D::intersperse(incoming.chain(outgoing), D::text(",").append(D::line()))
}
