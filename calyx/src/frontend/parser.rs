//! Parser for FuTIL programs.
use super::ast::{self, BitNum, NumType};
use crate::errors::{self, FutilResult, Span};
use crate::ir;
use pest::prec_climber::{Assoc, Operator, PrecClimber};
use pest_consume::{match_nodes, Error, Parser};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::rc::Rc;

type ParseResult<T> = Result<T, Error<Rule>>;
// user data is the input program so that we can create ir::Id's
// that have a reference to the input string
type Node<'i> = pest_consume::Node<'i, Rule, Rc<str>>;

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("futil_syntax.pest");

// Define the precedence of binary operations. We use `lazy_static` so that
// this is only ever constructed once.
lazy_static::lazy_static! {
    static ref PRECCLIMBER: PrecClimber<Rule> = PrecClimber::new(
        vec![
            // loosest binding
            Operator::new(Rule::guard_or, Assoc::Left),
            Operator::new(Rule::guard_and, Assoc::Left),
            // tighest binding
        ]
    );
}

#[derive(Parser)]
#[grammar = "frontend/futil_syntax.pest"]
pub struct FutilParser;

impl FutilParser {
    /// Parse a FuTIL program into an AST representation.
    pub fn parse_file(path: &PathBuf) -> FutilResult<ast::NamespaceDef> {
        let content = &fs::read(path).map_err(|err| {
            errors::Error::InvalidFile(format!(
                "Failed to read {}: {}",
                path.to_string_lossy(),
                err.to_string()
            ))
        })?;
        let string_content = std::str::from_utf8(content)?;
        let inputs = FutilParser::parse_with_userdata(
            Rule::file,
            string_content,
            Rc::from(string_content),
        )
        .map_err(|e| e.with_path(&path.to_string_lossy()))?;
        let input = inputs.single()?;
        Ok(FutilParser::file(input)?)
    }

    pub fn parse<R: Read>(mut r: R) -> FutilResult<ast::NamespaceDef> {
        let mut buf = String::new();
        r.read_to_string(&mut buf).map_err(|err| {
            errors::Error::InvalidFile(format!(
                "Failed to parse buffer: {}",
                err.to_string()
            ))
        })?;
        let inputs = FutilParser::parse_with_userdata(
            Rule::file,
            &buf,
            Rc::from(buf.as_str()),
        )?;
        let input = inputs.single()?;
        Ok(FutilParser::file(input)?)
    }
}

#[allow(clippy::large_enum_variant)]
enum ExtOrComp {
    Ext((String, Vec<ir::Primitive>)),
    Comp(ast::ComponentDef),
}

#[pest_consume::parser]
impl FutilParser {
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn semi(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    // ================ Literals =====================
    fn identifier(input: Node) -> ParseResult<ir::Id> {
        Ok(ir::Id::new(
            input.as_str(),
            Some(Span::new(input.as_span(), Rc::clone(input.user_data()))),
        ))
    }

    fn bitwidth(input: Node) -> ParseResult<u64> {
        input
            .as_str()
            .parse::<u64>()
            .map_err(|_| input.error("Expected valid bitwidth"))
    }

    fn bad_num(input: Node) -> ParseResult<u64> {
        Err(input.error("Expected number with bitwidth (like 32'd10)."))
    }

    fn hex(input: Node) -> ParseResult<u64> {
        u64::from_str_radix(input.as_str(), 16)
            .map_err(|_| input.error("Expected hexadecimal number"))
    }
    fn decimal(input: Node) -> ParseResult<u64> {
        u64::from_str_radix(input.as_str(), 10)
            .map_err(|_| input.error("Expected decimal number"))
    }
    fn octal(input: Node) -> ParseResult<u64> {
        u64::from_str_radix(input.as_str(), 8)
            .map_err(|_| input.error("Expected octal number"))
    }
    fn binary(input: Node) -> ParseResult<u64> {
        u64::from_str_radix(input.as_str(), 2)
            .map_err(|_| input.error("Expected binary number"))
    }

    fn num_lit(input: Node) -> ParseResult<BitNum> {
        Ok(match_nodes!(
            input.clone().into_children();
            [bitwidth(width), decimal(val)] => BitNum {
                    width,
                    num_type: NumType::Decimal,
                    val,
                    span: Some(Span::new(
                        input.as_span(),
                        Rc::clone(input.user_data()),
                    )),
                },
            [bitwidth(width), hex(val)] => BitNum {
                    width,
                    num_type: NumType::Hex,
                    val,
                    span: Some(Span::new(
                        input.as_span(),
                        Rc::clone(input.user_data()),
                    )),
                },
            [bitwidth(width), octal(val)] => BitNum {
                    width,
                    num_type: NumType::Octal,
                    val,
                    span: Some(Span::new(
                        input.as_span(),
                        Rc::clone(input.user_data()),
                    )),
                },
            [bitwidth(width), binary(val)] => BitNum {
                    width,
                    num_type: NumType::Binary,
                    val,
                    span: Some(Span::new(
                        input.as_span(),
                        Rc::clone(input.user_data()),
                    )),
                },

        ))
    }

    fn char(input: Node) -> ParseResult<&str> {
        Ok(input.as_str())
    }

    fn string_lit(input: Node) -> ParseResult<String> {
        Ok(match_nodes!(
            input.into_children();
            [char(c)..] => c.collect::<Vec<_>>().join("")
        ))
    }

    // ================ Attributes =====================
    fn attribute(input: Node) -> ParseResult<(String, u64)> {
        Ok(match_nodes!(
            input.into_children();
            [string_lit(key), bitwidth(num)] => (key, num)
        ))
    }

    fn attributes(input: Node) -> ParseResult<ir::Attributes> {
        Ok(match_nodes!(
            input.into_children();
            [attribute(kvs)..] => kvs.collect::<Vec<_>>().into()
        ))
    }

    fn at_attribute(input: Node) -> ParseResult<(String, u64)> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(key), bitwidth(num)] => (key.id, num)
        ))
    }

    fn at_attributes(input: Node) -> ParseResult<ir::Attributes> {
        Ok(match_nodes!(
            input.into_children();
            [at_attribute(kvs)..] => kvs.collect::<Vec<_>>().into()
        ))
    }

    // ================ Signature =====================
    fn params(input: Node) -> ParseResult<Vec<ir::Id>> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(id)..] => id.collect()
        ))
    }

    fn args(input: Node) -> ParseResult<Vec<u64>> {
        Ok(match_nodes!(
            input.into_children();
            [bitwidth(bw)..] => bw.collect(),
            [] => vec![]
        ))
    }

    fn io_port(
        input: Node,
    ) -> ParseResult<(ir::Id, ir::Width, ir::Attributes)> {
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), identifier(id), bitwidth(value)] =>
                (id, ir::Width::Const { value }, attrs),
            [at_attributes(attrs), identifier(id), identifier(value)] =>
                (id, ir::Width::Param { value }, attrs)
        ))
    }

    fn inputs(input: Node) -> ParseResult<Vec<ir::PortDef>> {
        Ok(match_nodes!(
            input.into_children();
            [io_port(ins)..] => {
                ins.map(|(name, width, attributes)| ir::PortDef {
                    name, width, direction: ir::Direction::Input, attributes
                }).collect()
            }
        ))
    }

    fn outputs(input: Node) -> ParseResult<Vec<ir::PortDef>> {
        Ok(match_nodes!(
            input.into_children();
            [io_port(outs)..] => {
                outs.map(|(name, width, attributes)| ir::PortDef {
                    name, width, direction: ir::Direction::Output, attributes
                }).collect()
            }
        ))
    }

    fn signature(input: Node) -> ParseResult<Vec<ir::PortDef>> {
        Ok(match_nodes!(
            input.into_children();
            // XXX(rachit): We expect the signature to be extended to have `go`,
            // `done`, and `clk`.
            [] => Vec::with_capacity(3),
            [inputs(ins)] => { ins },
            [outputs(outs)] => { outs },
            [inputs(ins), outputs(outs)] => {
                ins.into_iter().chain(outs.into_iter()).collect()
            },
        ))
    }

    // ==============PortDeftives =====================
    fn primitive(input: Node) -> ParseResult<ir::Primitive> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), attributes(attrs), params(p), signature(s)] => ir::Primitive {
                name,
                params: p,
                signature: s,
                attributes: attrs,
            },
            [identifier(name), attributes(attrs), signature(s)] => ir::Primitive {
                name,
                params: Vec::with_capacity(0),
                signature: s,
                attributes: attrs,
            },
            [identifier(name), params(p), signature(s)] => ir::Primitive {
                name,
                params: p,
                signature: s,
                attributes: ir::Attributes::default()
            },
            [identifier(name), signature(s)] => ir::Primitive {
                name,
                params: Vec::with_capacity(0),
                signature: s,
                attributes: ir::Attributes::default()
            }
        ))
    }

    // ================ Cells =====================
    fn cell_without_semi(input: Node) -> ParseResult<ast::Cell> {
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), identifier(id), identifier(prim), args(args)] =>
            ast::Cell::from(id, prim, args, attrs)
        ))
    }

    fn cell(input: Node) -> ParseResult<ast::Cell> {
        match_nodes!(
            input.clone().into_children();
            [cell_without_semi(node)] =>
                Err(input.error("Declaration is missing `;`")),
            [cell_without_semi(node), semi(_)] => Ok(node),
        )
    }

    fn cells(input: Node) -> ParseResult<Vec<ast::Cell>> {
        Ok(match_nodes!(
                input.into_children();
                [cell(cells)..] => cells.collect()
        ))
    }

    // ================ Wires =====================
    fn port(input: Node) -> ParseResult<ast::Port> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(component), identifier(port)] =>
                ast::Port::Comp { component, port },
            [identifier(port)] => ast::Port::This { port }
        ))
    }

    fn hole(input: Node) -> ParseResult<ast::Port> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(group), identifier(name)] => ast::Port::Hole { group, name }
        ))
    }

    fn LHS(input: Node) -> ParseResult<ast::Port> {
        Ok(match_nodes!(
            input.into_children();
            [port(port)] => port,
            [hole(hole)] => hole
        ))
    }

    fn expr(input: Node) -> ParseResult<ast::Atom> {
        match_nodes!(
            input.into_children();
            [LHS(port)] => Ok(ast::Atom::Port(port)),
            [num_lit(num)] => Ok(ast::Atom::Num(num)),
            [bad_num(num)] => unreachable!("bad_num returned non-error result"),
        )
    }

    fn guard_eq(_input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn guard_neq(_input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn guard_leq(_input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn guard_geq(_input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn guard_lt(_input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn guard_gt(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn cmp_expr(input: Node) -> ParseResult<ast::GuardExpr> {
        Ok(match_nodes!(
            input.into_children();
            [expr(l), guard_eq(_), expr(r)] => ast::GuardExpr::Eq(l, r),
            [expr(l), guard_neq(_), expr(r)] => ast::GuardExpr::Neq(l, r),
            [expr(l), guard_geq(_), expr(r)] => ast::GuardExpr::Geq(l, r),
            [expr(l), guard_leq(_), expr(r)] => ast::GuardExpr::Leq(l, r),
            [expr(l), guard_gt(_), expr(r)] =>  ast::GuardExpr::Gt(l, r),
            [expr(l), guard_lt(_), expr(r)] =>  ast::GuardExpr::Lt(l, r),
        ))
    }

    fn guard_not(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    #[prec_climb(term, PRECCLIMBER)]
    fn guard_expr(
        l: ast::GuardExpr,
        op: Node,
        r: ast::GuardExpr,
    ) -> ParseResult<ast::GuardExpr> {
        match op.as_rule() {
            Rule::guard_or => Ok(ast::GuardExpr::Or(Box::new(l), Box::new(r))),
            Rule::guard_and => {
                Ok(ast::GuardExpr::And(Box::new(l), Box::new(r)))
            }
            _ => unreachable!(),
        }
    }

    fn term(input: Node) -> ParseResult<ast::GuardExpr> {
        Ok(match_nodes!(
            input.into_children();
            [guard_expr(guard)] => guard,
            [cmp_expr(e)] => e,
            [expr(e)] => ast::GuardExpr::Atom(e),
            [guard_not(_), expr(e)] => {
                ast::GuardExpr::Not(Box::new(ast::GuardExpr::Atom(e)))
            },
            [guard_not(_), cmp_expr(e)] => {
                ast::GuardExpr::Not(Box::new(e))
            },
            [guard_not(_), guard_expr(e)] => {
                ast::GuardExpr::Not(Box::new(e))
            },
            [guard_not(_), expr(e)] =>
                ast::GuardExpr::Not(Box::new(ast::GuardExpr::Atom(e)))
        ))
    }

    fn switch_stmt(input: Node) -> ParseResult<ast::Guard> {
        Ok(match_nodes!(
            input.into_children();
            [guard_expr(guard), expr(expr)] => ast::Guard { guard: Some(guard), expr },
        ))
    }

    fn wire(input: Node) -> ParseResult<ast::Wire> {
        Ok(match_nodes!(
            input.into_children();
            [LHS(dest), expr(expr)] => ast::Wire {
                src: ast::Guard { guard: None, expr },
                dest
            },
            [LHS(dest), switch_stmt(src)] => ast::Wire {
                src,
                dest
            }
        ))
    }

    fn group(input: Node) -> ParseResult<ast::Group> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), attributes(attrs), wire(wire)..] => ast::Group {
                name,
                attributes: attrs,
                wires: wire.collect()
            },
            [identifier(name), wire(wire)..] => ast::Group {
                name,
                attributes: ir::Attributes::default(),
                wires: wire.collect()
            }
        ))
    }

    fn connections(
        input: Node,
    ) -> ParseResult<(Vec<ast::Wire>, Vec<ast::Group>)> {
        let mut wires = Vec::new();
        let mut groups = Vec::new();
        for node in input.into_children() {
            match node.as_rule() {
                Rule::wire => wires.push(Self::wire(node)?),
                Rule::group => groups.push(Self::group(node)?),
                _ => unreachable!(),
            }
        }
        Ok((wires, groups))
    }

    // ================ Control program =====================
    fn invoke_arg(input: Node) -> ParseResult<(ir::Id, ast::Port)> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), port(p)] => (name, p)
        ))
    }

    fn invoke_args(input: Node) -> ParseResult<Vec<(ir::Id, ast::Port)>> {
        Ok(match_nodes!(
            input.into_children();
            [invoke_arg(args)..] => args.collect()
        ))
    }

    fn invoke(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), identifier(comp), invoke_args(inputs), invoke_args(outputs)] =>
                ast::Control::Invoke {
                    comp,
                    inputs,
                    outputs,
                    attributes: attrs
                }
        ))
    }

    fn enable(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), identifier(name)] => ast::Control::Enable {
                comp: name,
                attributes: attrs
            }
        ))
    }

    fn seq(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), stmt(stmt)..] => ast::Control::Seq {
                stmts: stmt.collect(),
                attributes: attrs,
            }
        ))
    }

    fn par(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), stmt(stmt)..] => ast::Control::Par {
                stmts: stmt.collect(),
                attributes: attrs,
            }
        ))
    }

    fn if_stmt(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), port(port), identifier(cond), block(stmt)] => ast::Control::If {
                port,
                cond,
                tbranch: Box::new(stmt),
                fbranch: Box::new(ast::Control::Empty{}),
                attributes: attrs,
            },
            [at_attributes(attrs), port(port), identifier(cond), block(tbranch), block(fbranch)] =>
                ast::Control::If {
                    port,
                    cond,
                    tbranch: Box::new(tbranch),
                    fbranch: Box::new(fbranch),
                    attributes: attrs,
                },
            [at_attributes(attrs), port(port), identifier(cond), block(tbranch), if_stmt(fbranch)] =>
                ast::Control::If {
                    port,
                    cond,
                    tbranch: Box::new(tbranch),
                    fbranch: Box::new(fbranch),
                    attributes: attrs,
                },

        ))
    }

    fn while_stmt(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), port(port), identifier(cond), block(stmt)] => ast::Control::While {
                port,
                cond,
                body: Box::new(stmt),
                attributes: attrs,
            }
        ))
    }

    fn stmt(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [enable(data)] => data,
            [invoke(data)] => data,
            [seq(data)] => data,
            [par(data)] => data,
            [if_stmt(data)] => data,
            [while_stmt(data)] => data,
        ))
    }

    fn block(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [stmt(stmt)] => stmt,
            [stmts_without_block(_)] => unreachable!()
        ))
    }

    fn stmts_without_block(input: Node) -> ParseResult<ast::Control> {
        match_nodes!(
            input.clone().into_children();
            [stmt(_)..] => Err(
                input.error("Sequence of control statements should be enclosed in `seq` or `par`."))
        )
    }

    fn control(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [block(stmt)] => stmt,
            [] => ast::Control::Empty{}
        ))
    }

    fn component(input: Node) -> ParseResult<ast::ComponentDef> {
        Ok(match_nodes!(
            input.into_children();
            [
                identifier(id),
                signature(sig),
                cells(cells),
                connections(connections),
                control(control)
            ] => {
                let (continuous_assignments, groups) = connections;
                ast::ComponentDef {
                    name: id,
                    signature: sig,
                    cells,
                    groups,
                    continuous_assignments,
                    control,
                    attributes: ir::Attributes::default()
                }
            },
            [
                identifier(id),
                attributes(attributes),
                signature(sig),
                cells(cells),
                connections(connections),
                control(control)
            ] => {
                let (continuous_assignments, groups) = connections;
                ast::ComponentDef {
                    name: id,
                    signature: sig,
                    cells,
                    groups,
                    continuous_assignments,
                    control,
                    attributes,
                }
        }))
    }

    fn imports(input: Node) -> ParseResult<Vec<String>> {
        Ok(match_nodes!(
            input.into_children();
            [string_lit(path)..] => path.collect()
        ))
    }

    fn ext(input: Node) -> ParseResult<(String, Vec<ir::Primitive>)> {
        Ok(match_nodes!(
            input.into_children();
            [string_lit(file), primitive(prims)..] => (file, prims.collect())
        ))
    }

    fn extern_or_component(input: Node) -> ParseResult<ExtOrComp> {
        Ok(match_nodes!(
            input.into_children();
            [component(comp)] => ExtOrComp::Comp(comp),
            [ext(ext)] => ExtOrComp::Ext(ext)
        ))
    }

    fn file(input: Node) -> ParseResult<ast::NamespaceDef> {
        Ok(match_nodes!(
            input.into_children();
            [imports(imports), extern_or_component(mixed).., EOI] => {
                let mut namespace =
                    ast::NamespaceDef {
                        imports,
                        components: Vec::new(),
                        externs: Vec::new(),
                    };
                for m in mixed {
                    match m {
                        ExtOrComp::Ext(ext) => namespace.externs.push(ext),
                        ExtOrComp::Comp(comp) => namespace.components.push(comp),
                    }
                }
                namespace
            }
        ))
    }
}
