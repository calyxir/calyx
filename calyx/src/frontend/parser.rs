//! Parser for FuTIL programs.
use super::ast::{self, BitNum, NumType};
use crate::errors::{self, FutilResult, Span};
use crate::ir;
use pest::prec_climber::{Assoc, Operator, PrecClimber};
use pest_consume::{match_nodes, Error, Parser};
use std::collections::HashMap;
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
            Operator::new(Rule::guard_leq, Assoc::Left),
            Operator::new(Rule::guard_geq, Assoc::Left),
            Operator::new(Rule::guard_lt, Assoc::Left),
            Operator::new(Rule::guard_gt, Assoc::Left),
            Operator::new(Rule::guard_eq, Assoc::Left),
            Operator::new(Rule::guard_neq, Assoc::Left),
            Operator::new(Rule::guard_not, Assoc::Right)
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
        )?;
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

#[pest_consume::parser]
impl FutilParser {
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }

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

    fn signature(input: Node) -> ParseResult<ast::Signature> {
        Ok(match_nodes!(
            input.into_children();
            [io_ports(inputs), signature_return(outputs)] => ast::Signature {
                inputs,
                outputs
            },
            [io_ports(inputs)] => ast::Signature {
                inputs,
                outputs: vec![]
            },
            [signature_return(outputs)] => ast::Signature {
                inputs: vec![],
                outputs
            },
            [] => ast::Signature { inputs: vec![], outputs: vec![] }
        ))
    }

    fn signature_return(input: Node) -> ParseResult<Vec<(ir::Id, u64)>> {
        Ok(match_nodes!(
            input.into_children();
            [io_ports(p)] => p,
            [] => vec![]
        ))
    }

    fn io_port(input: Node) -> ParseResult<(ir::Id, u64)> {
        Ok(match_nodes![
            input.into_children();
            [identifier(id), bitwidth(bw)] => (id, bw)])
    }

    fn io_ports(input: Node) -> ParseResult<Vec<(ir::Id, u64)>> {
        Ok(match_nodes![
            input.into_children();
            [io_port(p)..] => p.collect()])
    }

    fn args(input: Node) -> ParseResult<Vec<u64>> {
        Ok(match_nodes!(
            input.into_children();
            [bitwidth(bw)..] => bw.collect(),
            [] => vec![]
        ))
    }

    fn primitive_cell(input: Node) -> ParseResult<ast::Cell> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(id), identifier(prim), args(args)] =>
            ast::Cell::prim(id, prim, args)
        ))
    }

    fn component_cell(input: Node) -> ParseResult<ast::Cell> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), identifier(component)] =>
                ast::Cell::Decl { name, component }
        ))
    }

    fn cell(input: Node) -> ParseResult<ast::Cell> {
        Ok(match_nodes!(
                input.into_children();
                [primitive_cell(node)] => node,
                [component_cell(node)] => node,
        ))
    }

    fn cells(input: Node) -> ParseResult<Vec<ast::Cell>> {
        Ok(match_nodes!(
                input.into_children();
                [cell(cells)..] => cells.collect()
        ))
    }

    fn port(input: Node) -> ParseResult<ast::Port> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(component), identifier(port)] => ast::Port::Comp { component, port },
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
            Rule::guard_eq => Ok(ast::GuardExpr::Eq(Box::new(l), Box::new(r))),
            Rule::guard_neq => {
                Ok(ast::GuardExpr::Neq(Box::new(l), Box::new(r)))
            }
            Rule::guard_leq => {
                Ok(ast::GuardExpr::Leq(Box::new(l), Box::new(r)))
            }
            Rule::guard_geq => {
                Ok(ast::GuardExpr::Geq(Box::new(l), Box::new(r)))
            }
            Rule::guard_lt => Ok(ast::GuardExpr::Lt(Box::new(l), Box::new(r))),
            Rule::guard_gt => Ok(ast::GuardExpr::Gt(Box::new(l), Box::new(r))),
            Rule::guard_or => Ok(ast::GuardExpr::Or(Box::new(l), Box::new(r))),
            Rule::guard_and => Ok(ast::GuardExpr::And(Box::new(l), Box::new(r))),
            _ => unreachable!(),
        }
    }

    fn term(input: Node) -> ParseResult<ast::GuardExpr> {
        Ok(match_nodes!(
            input.into_children();
            [guard_expr(guard)] => guard,
            [expr(e)] => ast::GuardExpr::Atom(e),
            [guard_not(_), guard_expr(e)] => ast::GuardExpr::Not(Box::new(e)),
            [guard_not(_), expr(e)] => ast::GuardExpr::Not(Box::new(ast::GuardExpr::Atom(e)))
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

    fn key_value(input: Node) -> ParseResult<(String, u64)> {
        Ok(match_nodes!(
            input.into_children();
            [string_lit(key), bitwidth(num)] => (key, num)
        ))
    }

    fn attributes(input: Node) -> ParseResult<HashMap<String, u64>> {
        Ok(match_nodes!(
            input.into_children();
            [key_value(kvs)..] => kvs.collect()
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
                attributes: HashMap::new(),
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
            [identifier(comp), invoke_args(inputs), invoke_args(outputs)] =>
                ast::Control::Invoke { comp, inputs, outputs }
        ))
    }

    fn enable(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name)] => ast::Control::Enable { comp: name }
        ))
    }

    fn seq(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [stmt(stmt)..] => ast::Control::Seq {
                stmts: stmt.collect()
            }
        ))
    }

    fn par(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [stmt(stmt)..] => ast::Control::Par {
                stmts: stmt.collect()
            }
        ))
    }

    fn if_stmt(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [port(port), identifier(cond), block(stmt)] => ast::Control::If {
                port,
                cond,
                tbranch: Box::new(stmt),
                fbranch: Box::new(ast::Control::Empty{})
            },
            [port(port), identifier(cond), block(tbranch), block(fbranch)] =>
                ast::Control::If {
                    port,
                    cond,
                    tbranch: Box::new(tbranch),
                    fbranch: Box::new(fbranch)
                },
            [port(port), identifier(cond), block(tbranch), if_stmt(fbranch)] =>
                ast::Control::If {
                    port,
                    cond,
                    tbranch: Box::new(tbranch),
                    fbranch: Box::new(fbranch)
                },

        ))
    }

    fn while_stmt(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [port(port), identifier(cond), block(stmt)] => ast::Control::While {
                port,
                cond,
                body: Box::new(stmt),
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
            }
        }))
    }

    fn imports(input: Node) -> ParseResult<Vec<String>> {
        Ok(match_nodes!(
            input.into_children();
            [string_lit(path)..] => path.collect()
        ))
    }

    fn file(input: Node) -> ParseResult<ast::NamespaceDef> {
        Ok(match_nodes!(
            input.into_children();
            [imports(imports), component(comps).., EOI] => ast::NamespaceDef {
                libraries: imports,
                components: comps.collect()
            }
        ))
    }
}
