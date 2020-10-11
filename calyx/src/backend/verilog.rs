use crate::backend::traits::{Backend, Emitable};
use crate::errors::{Error, Result};
use crate::frontend::{
    colors,
    colors::ColorHelper,
    pretty_print::{display, PrettyHelper},
};
use crate::lang::library::ast as lib;
use crate::lang::structure::Node;
use crate::{
    lang::{
        ast,
        ast::{Atom, Cell, Control, GuardExpr, Port},
        component, context,
        structure::{DataDirection, EdgeData, NodeData},
    },
    utils::OutputFile,
};
use itertools::Itertools;
use lib::Implementation;
use petgraph::graph::NodeIndex;
use pretty::termcolor::ColorSpec;
use pretty::RcDoc;
use std::cmp::Ordering;

type D<'a> = RcDoc<'a, ColorSpec>;

/// Implements a simple Verilog backend. The backend
/// only accepts Futil programs with no control and no groups.
pub struct VerilogBackend {}

/// Checks to make sure that there are no holes being
/// used in a guard.
fn validate_guard(guard: &GuardExpr) -> bool {
    match guard {
        GuardExpr::And(bs) => bs.iter().all(|b| validate_guard(b)),
        GuardExpr::Or(bs) => bs.iter().all(|b| validate_guard(b)),
        GuardExpr::Eq(left, right) => {
            validate_guard(left) && validate_guard(right)
        }
        GuardExpr::Neq(left, right) => {
            validate_guard(left) && validate_guard(right)
        }
        GuardExpr::Gt(left, right) => {
            validate_guard(left) && validate_guard(right)
        }
        GuardExpr::Lt(left, right) => {
            validate_guard(left) && validate_guard(right)
        }
        GuardExpr::Geq(left, right) => {
            validate_guard(left) && validate_guard(right)
        }
        GuardExpr::Leq(left, right) => {
            validate_guard(left) && validate_guard(right)
        }
        GuardExpr::Not(inner) => validate_guard(inner),
        GuardExpr::Atom(Atom::Port(p)) => {
            matches!(p, Port::Comp { .. } | Port::This { .. })
        }
        GuardExpr::Atom(Atom::Num(_)) => true,
    }
}

/// Returns `Ok` if there are no groups defined.
fn validate_structure(comp: &component::Component) -> Result<()> {
    let valid = comp.structure.edge_idx().all(|idx| {
        let edge = &comp.structure.get_edge(idx);
        edge.guard
            .as_ref()
            .map(|g| validate_guard(g))
            .unwrap_or(true)
            && edge.group.is_none()
    });
    if valid {
        Ok(())
    } else {
        Err(Error::MalformedStructure(
            "Groups / Holes can not be turned into Verilog".to_string(),
        ))
    }
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

    fn emit(ctx: &context::Context, file: OutputFile) -> Result<()> {
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
            .map(|(cd, comp)| cd.doc(&ctx, &comp))
            .collect::<Result<Vec<_>>>()?;
        let prims = primitive_implemenations(&prog, ctx)?;
        display(
            colors::comment(D::text(
                // XXX(sam) hack to deal with incorrect array index sizes
                "/* verilator lint_off WIDTH */",
            ))
            .append(D::line())
            .append(prims)
            .append(D::line())
            .append(D::line())
            .append(D::intersperse(docs, D::line())),
            file,
            ctx,
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
    fn doc<'a>(
        &self,
        ctx: &context::Context,
        comp: &component::Component,
    ) -> Result<D<'a>> {
        let memory_init_doc = if ctx.verilator_mode {
            colors::comment(D::text("// Memory initialization / finalization "))
                .append(D::line())
                .append(memory_init(&comp))
                .append(D::line())
                .append(D::line())
        } else {
            D::nil()
        };

        let structure = D::nil()
            .append(D::space())
            .append(self.name.to_string())
            .append(self.signature.doc(&ctx, &comp)?)
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
            .append(subcomponent_instances(&comp))
            .append(D::line())
            .append(D::line())
            .append(memory_init_doc)
            .append(colors::comment(D::text("// Input / output connections")))
            .append(D::line())
            .append(connections(&comp));
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
        ctx: &context::Context,
        comp: &component::Component,
    ) -> Result<D<'a>> {
        let mut inputs = self
            .inputs
            .iter()
            .map(|pd| {
                Ok(D::text("input")
                    .keyword_color()
                    .append(D::space())
                    .append(pd.doc(&ctx, &comp)?))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut outputs = self
            .outputs
            .iter()
            .map(|pd| {
                Ok(D::text("output")
                    .keyword_color()
                    .append(D::space())
                    .append(pd.doc(&ctx, &comp)?))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut ports = vec![];
        ports.append(&mut inputs);
        ports.append(&mut outputs);
        let doc =
            D::intersperse(ports.into_iter(), D::text(",").append(D::line()));
        Ok(D::space()
            .append(D::line().append(doc).nest(4).append(D::line()).parens()))
    }
}

impl Emitable for ast::Portdef {
    fn doc<'a>(
        &self,
        _: &context::Context,
        _comp: &component::Component,
    ) -> Result<D<'a>> {
        Ok(D::text("logic")
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
    let wires = comp
        .structure
        .component_iterator()
        // filter for cells because we don't need to declare wires for ports
        .filter_map(|(_idx, node)| match &node.data {
            NodeData::Cell(_) => Some(node),
            _ => None,
        })
        // extract name, portdef from input / output of signature
        .map(|node| {
            node.signature
                .inputs
                .iter()
                .map(move |pd| (&node.name, pd))
                .chain(
                    node.signature
                        .outputs
                        .iter()
                        .map(move |pd| (&node.name, pd)),
                )
        })
        .flatten()
        // XXX(sam), definitely could use `test` here
        .map(|(name, portdef)| {
            Ok(D::text("logic")
                .keyword_color()
                .append(D::space())
                .append(bitwidth(portdef.width)?)
                .append(format!(
                    "{}_{}",
                    name.to_string(),
                    portdef.name.to_string()
                ))
                .append(";"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(D::intersperse(wires, D::line()))
}

/// Generates a Verilog identifier for a (Node, String).
///  * NodeData::Cell(..) => name_port
///  * NodeData::Port => port
///  * NodeData::Hole => impossible!
///  * NodeData::Constant({width: w, value: v}) => w'dv
fn wire_id_from_node<'a>(node: &Node, port: String) -> D<'a> {
    match &node.data {
        NodeData::Cell(..) => {
            D::text(format!("{}_{}", node.name.to_string(), port))
        }
        NodeData::ThisPort => D::text(port),
        NodeData::Hole(name) => {
            unreachable!(format!("Structure has a hole: {}", name.id))
        }
        NodeData::Constant(n) => D::text(format!("{}'d{}", n.width, n.val)),
    }
}

/// Tracks the context in the guards to only generate parens when inside an
/// operator with stronger binding.
#[derive(Debug, Eq, PartialEq)]
enum ParenCtx {
    Op,
    Not,
    And,
    Or,
}
impl Ord for ParenCtx {
    fn cmp(&self, other: &Self) -> Ordering {
        use ParenCtx as P;
        match (self, other) {
            (P::Not, _) => Ordering::Greater,
            (P::Op, P::Not) => Ordering::Less,
            (P::Op, _) => Ordering::Greater,
            (P::And, P::Op) | (P::And, P::Not) => Ordering::Less,
            (P::And, _) => Ordering::Greater,
            (P::Or, _) => Ordering::Less,
        }
    }
}
impl PartialOrd for ParenCtx {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
/// Converts a guarded edge into a Verilog string
fn guard<'a>(expr: &GuardExpr, ctx: ParenCtx) -> D<'a> {
    use ParenCtx as P;
    match expr {
        GuardExpr::Atom(a) => atom(a),
        GuardExpr::Not(a) => {
            let ret = D::text(expr.op_str()).append(guard(a, P::Not));
            if ctx > P::Not {
                ret.parens()
            } else {
                ret
            }
        }
        GuardExpr::And(bs) => {
            let ret = D::intersperse(
                bs.iter().map(|b| guard(b, P::And)).collect::<Vec<_>>(),
                D::text(" & "),
            );
            if ctx > P::And {
                ret.parens()
            } else {
                ret
            }
        }
        GuardExpr::Or(bs) => {
            let ret = D::intersperse(
                bs.iter().map(|b| guard(b, P::Or)).collect::<Vec<_>>(),
                D::text(" | "),
            );
            if ctx > P::Or {
                ret.parens()
            } else {
                ret
            }
        }
        GuardExpr::Eq(a, b)
        | GuardExpr::Neq(a, b)
        | GuardExpr::Gt(a, b)
        | GuardExpr::Lt(a, b)
        | GuardExpr::Geq(a, b)
        | GuardExpr::Leq(a, b) => {
            let ret = D::nil().append(
                guard(a, P::Op)
                    .append(format!(" {} ", expr.op_str()))
                    .append(guard(b, P::Op)),
            );
            if ctx > P::Op {
                ret.parens()
            } else {
                ret
            }
        }
    }
}

/// Converts ast::Atom to a verilog string
fn atom<'a>(atom: &Atom) -> D<'a> {
    match atom {
        Atom::Port(p) => match p {
            Port::Comp { component, port } => D::text(format!(
                "{}_{}",
                component.to_string(),
                port.to_string()
            )),
            Port::This { port } => D::text(port.to_string()),
            Port::Hole { .. } => unreachable!(
                "Holes should be caught in the backend validation."
            ),
        },
        Atom::Num(n) => D::text(format!("{}'d{}", n.width, n.val)),
    }
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

/// Get all the assignments to a given (node, port) pair.
fn get_all_edges(
    comp: &component::Component,
    node: NodeIndex,
    port: String,
) -> (String, Vec<(EdgeData, &Node)>) {
    // collect all edges writing into this node and port
    let edges = comp
        .structure
        .edge_idx()
        .with_direction(DataDirection::Write)
        .with_node(node)
        .with_port(port.clone())
        .map(|idx| {
            (
                comp.structure.get_edge(idx).clone(),
                comp.structure.get_node(comp.structure.endpoints(idx).0),
            )
        })
        .collect::<Vec<_>>();

    (port, edges)
}

/// Generate a sequence of ternary assignments into the (node, port) using
/// edges. Generated code looks like:
/// node.port = g1 ? n1.p1 : g2 ? n2.p2 ...
fn gen_assigns<'a>(
    node: &Node,
    port: String,
    edges: Vec<(EdgeData, &Node)>,
) -> D<'a> {
    let unguarded_drivers = edges
        .iter()
        .filter(|(ed, _)| {
            ed.guard.is_none() || ed.guard.as_ref().unwrap().provably_true()
        })
        .count();

    // Error if there is more than one unguarded driver.
    if unguarded_drivers > 1 {
        panic!(
            "Multiple unguarded drivers for {}.{}",
            node.name.to_string(),
            port
        );
    }

    // Error if there is an unguarded driver along with other guarded drivers.
    if unguarded_drivers == 1 && edges.len() > 1 {
        panic!(
            "{}.{} driven by both unguarded and guarded drivers",
            node.name.to_string(),
            port
        );
    }

    if unguarded_drivers == 1 {
        let (el, src_node) = &edges[0];
        let dest = wire_id_from_node(node, port);
        dest.append(" = ")
            .append(wire_id_from_node(src_node, el.src.port_name().to_string()))
            .append(";")
    } else {
        let pre = wire_id_from_node(&node, port).append(" = ");
        let default = D::line()
            .nest(2)
            .append(pre.clone().append("'0").append(";"))
            .append(D::line());
        edges
            .iter()
            // Sort by the destination port names. This is required for
            // deterministic outputs.
            .sorted_by(|e1, e2| Ord::cmp(&e1.0.src, &e2.0.src))
            .fold(default, |acc, (el, node)| {
                let cond = D::text("if ").keyword_color().append(
                    el.guard
                        .as_ref()
                        .map(|g| guard(&g, ParenCtx::Not).parens())
                        .unwrap_or_else(D::nil),
                );
                let assign = pre
                    .clone()
                    .append(wire_id_from_node(
                        &node,
                        el.src.port_name().to_string(),
                    ))
                    .append(";");
                cond.append(D::line().nest(2).append(assign))
                    .append(D::line())
                    .append(D::text("else ").keyword_color())
                    .append(acc)
            })
    }
}

//==========================================
//        Connection Functions
//==========================================
/// Generate wire connections
fn connections<'a>(comp: &component::Component) -> D<'a> {
    let doc = comp
        .structure
        .component_iterator()
        // for every component
        .map(|(idx, node)| {
            node.signature
                .inputs
                .iter()
                // get all the edges writing into a port
                .map(|portdef| {
                    get_all_edges(&comp, idx, portdef.name.to_string())
                })
                // remove empty edges
                .filter(|(_, edges)| !edges.is_empty())
                .map(|(port, edges)| gen_assigns(node, port, edges))
                .collect::<Vec<_>>()
        })
        .flatten();

    D::text("always_comb begin")
        .append(D::line().append(D::intersperse(doc, D::line())).nest(2))
        .append(D::line())
        .append(D::text("end"))
}

//==========================================
//        Subcomponent Instance Functions
//==========================================
/// Generate Verilog for each subcomponent instanstiation and
/// wire up all the ports.
fn subcomponent_instances<'a>(comp: &component::Component) -> D<'a> {
    let doc = comp
        .structure
        .component_iterator()
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
    let all = sig.inputs.iter().chain(sig.outputs.iter()).map(|portdef| {
        // if portdef is named `clk`, wire up `clk`
        if &portdef.name == "clk" {
            D::text(".").append("clk").append(D::text("clk").parens())
        } else {
            D::text(".")
                .append(D::text(portdef.name.to_string()))
                .append(
                    D::text(format!(
                        "{}_{}",
                        comp.structure.get_node(idx).name.to_string(),
                        portdef.name.to_string()
                    ))
                    .parens(),
                )
        }
    });

    D::intersperse(all, D::text(",").append(D::line()))
}

//==========================================
//        Memory init functions
//==========================================
// Generates code of the form:
// ```
// import "DPI-C" function string futil_getenv (input string env_var);
// string DATA;
// initial begin
//   DATA = futil_getenv("DATA");
//   $display("DATA: %s", DATA);
//   $readmemh({DATA, "/<mem_name>.out"}, <mem_name>.mem);
//   ...
// end
// final begin
//   $writememh({DATA, "/<mem_name>.out"}, <mem_name>.mem);
// end
// ```
fn memory_init<'a>(comp: &component::Component) -> D<'a> {
    // Import futil helper library.
    const IMPORT_STMT: &str =
        "import \"DPI-C\" function string futil_getenv (input string env_var);";
    const DATA_DECL: &str = "string DATA;";
    const DATA_GET: &str = "DATA = futil_getenv(\"DATA\");";
    const DATA_DISP: &str =
        "$fdisplay(2, \"DATA (path to meminit files): %s\", DATA);";

    let initial_block = D::text("initial begin")
        .append(D::line())
        .append(
            (D::text(DATA_GET)
                .append(D::line())
                .append(DATA_DISP)
                .append(memory_load_store("$readmemh", "dat", &comp)))
            .nest(4),
        )
        .append(D::line())
        .append("end");

    let final_block = D::text("final begin")
        .append(memory_load_store("$writememh", "out", &comp).nest(4))
        .append(D::line())
        .append("end");

    D::text(IMPORT_STMT)
        .append(D::line())
        .append(DATA_DECL)
        .append(D::line())
        .append(D::space())
        .append(initial_block)
        .append(D::line())
        .append(D::line())
        .append(D::space())
        .append(final_block)
}

fn memory_load_store<'a>(
    mem_f: &'static str,
    ext: &'static str,
    comp: &component::Component,
) -> D<'a> {
    let doc = comp
        .structure
        .component_iterator()
        .filter(|(_, node)| {
            if let NodeData::Cell(Cell::Prim { data }) = &node.data {
                data.instance.name.to_string().contains("mem")
            } else {
                false
            }
        })
        .map(|(_, node)| {
            D::text(mem_f)
                .append(
                    D::text(format!(
                        "{{ DATA, \"/{}.{}\" }}",
                        node.name.to_string(),
                        ext
                    ))
                    .append(",")
                    .append(D::space())
                    .append(format!("{}.mem", node.name.to_string()))
                    .parens(),
                )
                .append(";")
        });

    D::line().append(D::intersperse(doc, D::line()))
}
