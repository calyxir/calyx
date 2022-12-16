#![allow(clippy::upper_case_acronyms)]

//! Parser for Calyx programs.
use super::ast::{self, BitNum, Control, GuardComp as GC, GuardExpr, NumType};
use crate::errors::{self, CalyxResult, Span};
use crate::ir;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest_consume::{match_nodes, Error, Parser};
use std::convert::TryInto;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::rc::Rc;

type ParseResult<T> = Result<T, Error<Rule>>;
type PortDef = ir::PortDef<ir::Width>;
type ComponentDef = ast::ComponentDef;

/// Data associated with parsing the file.
#[derive(Clone)]
struct UserData {
    /// Input to the parser
    pub input: Rc<str>,
    /// Path of the file
    pub file: Rc<str>,
}

// user data is the input program so that we can create ir::Id's
// that have a reference to the input string
type Node<'i> = pest_consume::Node<'i, Rule, UserData>;

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("syntax.pest");

// Define the precedence of binary operations. We use `lazy_static` so that
// this is only ever constructed once.
lazy_static::lazy_static! {
    static ref PRATT: PrattParser<Rule> =
    PrattParser::new()
        .op(Op::infix(Rule::guard_or, Assoc::Left))
        .op(Op::infix(Rule::guard_and, Assoc::Left));
}

#[derive(Parser)]
#[grammar = "frontend/syntax.pest"]
pub struct CalyxParser;

impl CalyxParser {
    /// Parse a Calyx program into an AST representation.
    pub fn parse_file(path: &Path) -> CalyxResult<ast::NamespaceDef> {
        let time = std::time::Instant::now();
        let content = &fs::read(path).map_err(|err| {
            errors::Error::invalid_file(format!(
                "Failed to read {}: {err}",
                path.to_string_lossy(),
            ))
        })?;
        let string_content = std::str::from_utf8(content)?;
        let user_data = UserData {
            input: Rc::from(string_content),
            file: Rc::from(path.to_string_lossy()),
        };
        let inputs = CalyxParser::parse_with_userdata(
            Rule::file,
            string_content,
            user_data,
        )
        .map_err(|e| e.with_path(&path.to_string_lossy()))?;
        let input = inputs.single()?;
        let out = CalyxParser::file(input)?;
        log::info!(
            "Parsed `{}` in {}ms",
            path.to_string_lossy(),
            time.elapsed().as_millis()
        );
        Ok(out)
    }

    pub fn parse<R: Read>(mut r: R) -> CalyxResult<ast::NamespaceDef> {
        let mut buf = String::new();
        r.read_to_string(&mut buf).map_err(|err| {
            errors::Error::invalid_file(format!(
                "Failed to parse buffer: {err}",
            ))
        })?;
        let user_data = UserData {
            input: Rc::from(buf.as_ref()),
            file: Rc::from("<stdin>"),
        };
        let inputs =
            CalyxParser::parse_with_userdata(Rule::file, &buf, user_data)?;
        let input = inputs.single()?;
        Ok(CalyxParser::file(input)?)
    }

    fn get_span(node: &Node) -> Rc<Span> {
        let ud = node.user_data();
        Rc::new(Span::new(
            node.as_span(),
            Rc::clone(&ud.file),
            Rc::clone(&ud.input),
        ))
    }

    #[allow(clippy::result_large_err)]
    fn guard_expr_helper(
        ud: UserData,
        pairs: pest::iterators::Pairs<Rule>,
    ) -> ParseResult<Box<GuardExpr>> {
        PRATT
            .map_primary(|primary| match primary.as_rule() {
                Rule::term => {
                    Self::term(Node::new_with_user_data(primary, ud.clone()))
                        .map(Box::new)
                }
                x => unreachable!("Unexpected rule {:?} for guard_expr", x),
            })
            .map_infix(|lhs, op, rhs| {
                Ok(match op.as_rule() {
                    Rule::guard_or => Box::new(GuardExpr::Or(lhs?, rhs?)),
                    Rule::guard_and => Box::new(GuardExpr::And(lhs?, rhs?)),
                    _ => unreachable!(),
                })
            })
            .parse(pairs)
    }
}

#[allow(clippy::large_enum_variant)]
enum ExtOrComp {
    Ext((String, Vec<ir::Primitive>)),
    Comp(ComponentDef),
}

#[pest_consume::parser]
impl CalyxParser {
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn semi(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn comb(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn reference(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    // ================ Literals =====================
    fn identifier(input: Node) -> ParseResult<ir::Id> {
        let span = Self::get_span(&input);
        Ok(ir::Id::new(input.as_str(), Some(span)))
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
        #[allow(clippy::from_str_radix_10)]
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
        let ud = input.user_data();
        let input_ref = Rc::clone(&ud.input);
        let file_ref = Rc::clone(&ud.file);
        let num = match_nodes!(
            input.clone().into_children();
            [bitwidth(width), decimal(val)] => BitNum {
                    width,
                    num_type: NumType::Decimal,
                    val,
                    span: Some(Span::new(
                        input.as_span(),
                        file_ref,
                        input_ref,
                    )),
                },
            [bitwidth(width), hex(val)] => BitNum {
                    width,
                    num_type: NumType::Hex,
                    val,
                    span: Some(Span::new(
                        input.as_span(),
                        file_ref,
                        input_ref,
                    )),
                },
            [bitwidth(width), octal(val)] => BitNum {
                    width,
                    num_type: NumType::Octal,
                    val,
                    span: Some(Span::new(
                        input.as_span(),
                        file_ref,
                        input_ref,
                    )),
                },
            [bitwidth(width), binary(val)] => BitNum {
                    width,
                    num_type: NumType::Binary,
                    val,
                    span: Some(Span::new(
                        input.as_span(),
                        file_ref,
                        input_ref,
                    )),
                },

        );

        // the below cast is safe since the width must be less than 64 for
        // the given literal to be unrepresentable
        if num.width == 0
            || (num.width < 64 && u64::pow(2, num.width as u32) <= num.val)
        {
            let lit_str = match num.num_type {
                NumType::Binary => format!("{:b}", num.val),
                NumType::Decimal => format!("{}", num.val),
                NumType::Octal => format!("{:o}", num.val),
                NumType::Hex => format!("{:x}", num.val),
            };
            let bit_plural = if num.width == 1 { "bit" } else { "bits" };
            Err(input.error(format!(
                "Cannot represent given literal '{}' in {} {}",
                lit_str, num.width, bit_plural
            )))
        } else {
            Ok(num)
        }
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
        match_nodes!(
            input.clone().into_children();
            [attribute(kvs)..] => kvs.collect::<Vec<_>>().try_into().map_err(|e| input.error(format!("{:?}", e)))
        )
    }
    fn name_with_attribute(
        input: Node,
    ) -> ParseResult<(ir::Id, ir::Attributes)> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), attributes(attrs)] => (name, attrs),
            [identifier(name)] => (name, ir::Attributes::default()),
        ))
    }

    fn attr_val(input: Node) -> ParseResult<u64> {
        Ok(match_nodes!(
            input.into_children();
            [bitwidth(num)] => num
        ))
    }

    fn at_attribute(input: Node) -> ParseResult<(String, u64)> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(key), attr_val(num)] => (key.id.as_str().to_owned(), num),
            [identifier(key)] => (key.id.as_str().to_owned(), 1)
        ))
    }

    fn at_attributes(input: Node) -> ParseResult<ir::Attributes> {
        match_nodes!(
            input.clone().into_children();
            [at_attribute(kvs)..] => kvs.collect::<Vec<_>>().try_into().map_err(|e| input.error(format!("{:?}", e)))
        )
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

    fn inputs(input: Node) -> ParseResult<Vec<PortDef>> {
        Ok(match_nodes!(
            input.into_children();
            [io_port(ins)..] => {
                ins.map(|(name, width, attributes)| ir::PortDef {
                    name, width, direction: ir::Direction::Input, attributes
                }).collect()
            }
        ))
    }

    fn outputs(input: Node) -> ParseResult<Vec<PortDef>> {
        Ok(match_nodes!(
            input.into_children();
            [io_port(outs)..] => {
                outs.map(|(name, width, attributes)| ir::PortDef {
                    name, width, direction: ir::Direction::Output, attributes
                }).collect()
            }
        ))
    }

    fn signature(input: Node) -> ParseResult<Vec<PortDef>> {
        Ok(match_nodes!(
            input.into_children();
            // NOTE(rachit): We expect the signature to be extended to have `go`,
            // `done`, `reset,`, and `clk`.
            [] => Vec::with_capacity(4),
            [inputs(ins)] =>  ins ,
            [outputs(outs)] =>  outs ,
            [inputs(ins), outputs(outs)] => {
                ins.into_iter().chain(outs.into_iter()).collect()
            },
        ))
    }

    // ==============Primitives=====================
    fn sig_with_params(
        input: Node,
    ) -> ParseResult<(Vec<ir::Id>, Vec<PortDef>)> {
        Ok(match_nodes!(
            input.into_children();
            [params(p), signature(s)] => (p, s),
            [signature(s)] => (vec![], s),
        ))
    }
    fn primitive(input: Node) -> ParseResult<ir::Primitive> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [name_with_attribute((name, attrs)), sig_with_params((p, s))] => ir::Primitive {
                name,
                params: p,
                signature: s,
                attributes: attrs.add_span(span),
                is_comb: false,
            },
            [comb(_), name_with_attribute((name, attrs)), sig_with_params((p, s))] => ir::Primitive {
                name,
                params: p,
                signature: s,
                attributes: attrs.add_span(span),
                is_comb: true,
            },
        ))
    }

    // ================ Cells =====================
    fn cell_without_semi(input: Node) -> ParseResult<ast::Cell> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), reference(_), identifier(id), identifier(prim), args(args)] =>
            ast::Cell::from(id, prim, args, attrs.add_span(span),true),
            [at_attributes(attrs), identifier(id), identifier(prim), args(args)] =>
            ast::Cell::from(id, prim, args, attrs.add_span(span),false)
        ))
    }

    fn cell(input: Node) -> ParseResult<ast::Cell> {
        match_nodes!(
            input.clone().into_children();
            [cell_without_semi(_)] =>
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

    #[allow(clippy::upper_case_acronyms)]
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
            [bad_num(_)] => unreachable!("bad_num returned non-error result"),
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
            [expr(l), guard_eq(_), expr(r)] => GuardExpr::CompOp(GC::Eq, l, r),
            [expr(l), guard_neq(_), expr(r)] => GuardExpr::CompOp(GC::Neq, l, r),
            [expr(l), guard_geq(_), expr(r)] => GuardExpr::CompOp(GC::Geq, l, r),
            [expr(l), guard_leq(_), expr(r)] => GuardExpr::CompOp(GC::Leq, l, r),
            [expr(l), guard_gt(_), expr(r)] =>  GuardExpr::CompOp(GC::Gt, l, r),
            [expr(l), guard_lt(_), expr(r)] =>  GuardExpr::CompOp(GC::Lt, l, r),
        ))
    }

    fn guard_not(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn guard_expr(input: Node) -> ParseResult<Box<GuardExpr>> {
        let ud = input.user_data().clone();
        Self::guard_expr_helper(ud, input.into_pair().into_inner())
    }

    fn term(input: Node) -> ParseResult<ast::GuardExpr> {
        Ok(match_nodes!(
            input.into_children();
            [guard_expr(guard)] => *guard,
            [cmp_expr(e)] => e,
            [expr(e)] => ast::GuardExpr::Atom(e),
            [guard_not(_), expr(e)] => {
                ast::GuardExpr::Not(Box::new(ast::GuardExpr::Atom(e)))
            },
            [guard_not(_), cmp_expr(e)] => {
                ast::GuardExpr::Not(Box::new(e))
            },
            [guard_not(_), guard_expr(e)] => {
                ast::GuardExpr::Not(e)
            },
            [guard_not(_), expr(e)] =>
                ast::GuardExpr::Not(Box::new(ast::GuardExpr::Atom(e)))
        ))
    }

    fn switch_stmt(input: Node) -> ParseResult<ast::Guard> {
        Ok(match_nodes!(
            input.into_children();
            [guard_expr(guard), expr(expr)] => ast::Guard { guard: Some(*guard), expr },
        ))
    }

    fn wire(input: Node) -> ParseResult<ast::Wire> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), LHS(dest), expr(expr)] => ast::Wire {
                src: ast::Guard { guard: None, expr },
                dest,
                attributes: attrs.add_span(span),
            },
            [at_attributes(attrs), LHS(dest), switch_stmt(src)] => ast::Wire {
                src,
                dest,
                attributes: attrs.add_span(span),
            }
        ))
    }

    fn group(input: Node) -> ParseResult<ast::Group> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [name_with_attribute((name, attrs)), wire(wire)..] => ast::Group {
                name,
                attributes: attrs.add_span(span),
                wires: wire.collect(),
                is_comb: false,
            },
            [comb(_), name_with_attribute((name, attrs)), wire(wire)..] => ast::Group {
                name,
                attributes: attrs.add_span(span),
                wires: wire.collect(),
                is_comb: true,
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
    fn invoke_arg(input: Node) -> ParseResult<(ir::Id, ast::Atom)> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), port(p)] => (name, ast::Atom::Port(p)),
            [identifier(name), num_lit(bn)] => (name, ast::Atom::Num(bn))

        ))
    }

    fn invoke_args(input: Node) -> ParseResult<Vec<(ir::Id, ast::Atom)>> {
        Ok(match_nodes!(
            input.into_children();
            [invoke_arg(args)..] => args.collect()
        ))
    }

    fn invoke_ref_arg(input: Node) -> ParseResult<(ir::Id, ir::Id)> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(outcell), identifier(incell)] => (outcell, incell)
        ))
    }

    fn invoke_ref_args(input: Node) -> ParseResult<Vec<(ir::Id, ir::Id)>> {
        Ok(match_nodes!(
            input.into_children();
            [invoke_ref_arg(args)..] => args.collect(),
            [] => Vec::new()
        ))
    }

    fn invoke(input: Node) -> ParseResult<ast::Control> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), identifier(comp), invoke_ref_args(cells),invoke_args(inputs), invoke_args(outputs)] =>
                ast::Control::Invoke {
                    comp,
                    inputs,
                    outputs,
                    attributes: attrs.add_span(span),
                    comb_group: None,
                    ref_cells: cells
                },
            [at_attributes(attrs), identifier(comp), invoke_ref_args(cells),invoke_args(inputs), invoke_args(outputs), identifier(group)] =>
                ast::Control::Invoke {
                    comp,
                    inputs,
                    outputs,
                    attributes: attrs.add_span(span),
                    comb_group: Some(group),
                    ref_cells: cells
                },

        ))
    }

    fn empty(input: Node) -> ParseResult<ast::Control> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs)] => ast::Control::Empty {
                attributes: attrs.add_span(span)
            }
        ))
    }

    fn enable(input: Node) -> ParseResult<ast::Control> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), identifier(name)] => ast::Control::Enable {
                comp: name,
                attributes: attrs.add_span(span)
            }
        ))
    }

    fn seq(input: Node) -> ParseResult<ast::Control> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), stmt(stmt)..] => ast::Control::Seq {
                stmts: stmt.collect(),
                attributes: attrs.add_span(span),
            }
        ))
    }

    fn par(input: Node) -> ParseResult<ast::Control> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), stmt(stmt)..] => ast::Control::Par {
                stmts: stmt.collect(),
                attributes: attrs.add_span(span),
            }
        ))
    }

    fn port_with(input: Node) -> ParseResult<(ast::Port, Option<ir::Id>)> {
        Ok(match_nodes!(
            input.into_children();
            [port(port), identifier(cond)] => (port, Some(cond)),
            [port(port)] => (port, None),
        ))
    }

    fn if_stmt(input: Node) -> ParseResult<ast::Control> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), port_with((port, cond)), block(stmt)] => ast::Control::If {
                port,
                cond,
                tbranch: Box::new(stmt),
                fbranch: Box::new(ast::Control::Empty { attributes: ir::Attributes::default() }),
                attributes: attrs.add_span(span),
            },
            [at_attributes(attrs), port_with((port, cond)), block(tbranch), block(fbranch)] =>
                ast::Control::If {
                    port,
                    cond,
                    tbranch: Box::new(tbranch),
                    fbranch: Box::new(fbranch),
                    attributes: attrs.add_span(span),
                },
            [at_attributes(attrs), port_with((port, cond)), block(tbranch), if_stmt(fbranch)] =>
                ast::Control::If {
                    port,
                    cond,
                    tbranch: Box::new(tbranch),
                    fbranch: Box::new(fbranch),
                    attributes: attrs.add_span(span),
                },

        ))
    }

    fn while_stmt(input: Node) -> ParseResult<ast::Control> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [at_attributes(attrs), port_with((port, cond)), block(stmt)] => ast::Control::While {
                port,
                cond,
                body: Box::new(stmt),
                attributes: attrs.add_span(span),
            }
        ))
    }

    fn stmt(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [enable(data)] => data,
            [empty(data)] => data,
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
            [] => ast::Control::empty()
        ))
    }

    fn component(input: Node) -> ParseResult<ComponentDef> {
        let span = Self::get_span(&input);
        match_nodes!(
        input.clone().into_children();
        [
            comb(_),
            name_with_attribute((name, attributes)),
            signature(sig),
            cells(cells),
            connections(connections)
        ] => {
            let (continuous_assignments, groups) = connections;
            let sig = sig.into_iter().map(|ir::PortDef { name, width, direction, attributes }| {
                if let ir::Width::Const { value } = width {
                    Ok(ir::PortDef {
                        name,
                        width: value,
                        direction,
                        attributes
                    })
                } else {
                    Err(input.error("Components cannot use parameters"))
                }
            }).collect::<Result<_, _>>()?;
            Ok(ComponentDef {
                name,
                signature: sig,
                cells,
                groups,
                continuous_assignments,
                control: Control::empty(),
                attributes: attributes.add_span(span),
                is_comb: true,
            })
        },
        [
            name_with_attribute((name, attributes)),
            signature(sig),
            cells(cells),
            connections(connections),
            control(control)
        ] => {
            let (continuous_assignments, groups) = connections;
            let sig = sig.into_iter().map(|ir::PortDef { name, width, direction, attributes }| {
                if let ir::Width::Const { value } = width {
                    Ok(ir::PortDef {
                        name,
                        width: value,
                        direction,
                        attributes
                    })
                } else {
                    Err(input.error("Components cannot use parameters"))
                }
            }).collect::<Result<_, _>>()?;
            Ok(ComponentDef {
                name,
                signature: sig,
                cells,
                groups,
                continuous_assignments,
                control,
                attributes: attributes.add_span(span),
                is_comb: false,
            })
        })
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

    fn externs_and_comps(
        input: Node,
    ) -> ParseResult<impl Iterator<Item = ExtOrComp>> {
        Ok(match_nodes!(input.into_children();
            [extern_or_component(e)..] => e
        ))
    }

    fn any_char(input: Node) -> ParseResult<String> {
        Ok(input.as_str().into())
    }

    fn metadata_char(input: Node) -> ParseResult<String> {
        Ok(match_nodes!(input.into_children();
            [any_char(c)] => c,
        ))
    }

    fn metadata(input: Node) -> ParseResult<String> {
        Ok(match_nodes!(input.into_children();
            [metadata_char(c)..] => c.collect::<String>().trim().into()
        ))
    }

    fn file(input: Node) -> ParseResult<ast::NamespaceDef> {
        Ok(match_nodes!(
            input.into_children();
            // There really seems to be no straightforward way to resolve this
            // duplication
            [imports(imports), externs_and_comps(mixed), metadata(m), EOI(_)] => {
                let mut namespace =
                    ast::NamespaceDef {
                        imports,
                        components: Vec::new(),
                        externs: Vec::new(),
                        metadata: if m != *"" { Some(m) } else { None }
                    };
                for m in mixed {
                    match m {
                        ExtOrComp::Ext(ext) => namespace.externs.push(ext),
                        ExtOrComp::Comp(comp) => namespace.components.push(comp),
                    }
                }
                namespace
            },
            [imports(imports), externs_and_comps(mixed), EOI(_)] => {
                let mut namespace =
                    ast::NamespaceDef {
                        imports,
                        components: Vec::new(),
                        externs: Vec::new(),
                        metadata: None
                    };
                for m in mixed {
                    match m {
                        ExtOrComp::Ext(ext) => namespace.externs.push(ext),
                        ExtOrComp::Comp(comp) => namespace.components.push(comp),
                    }
                }
                namespace
            },

        ))
    }
}
