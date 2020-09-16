use crate::errors::{self, Result, Span};
use crate::lang::library::ast as lib;
use crate::lang::{
    ast,
    ast::{BitNum, NumType},
};
use pest::prec_climber::{Assoc, Operator, PrecClimber};
use pest_consume::{match_nodes, Error, Parser};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::rc::Rc;

type ParseResult<T> = std::result::Result<T, Error<Rule>>;
// user data is the input program so that we can create Ast::id's
// that have a reference to the input string
type Node<'i> = pest_consume::Node<'i, Rule, Rc<String>>;

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
    pub fn parse_file(path: &PathBuf) -> Result<ast::NamespaceDef> {
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
            Rc::new(string_content.to_string()),
        )?;
        let input = inputs.single()?;
        Ok(FutilParser::file(input)?)
    }

    pub fn parse<R: Read>(mut r: R) -> Result<ast::NamespaceDef> {
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
            Rc::new(buf.to_string()),
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

    fn identifier(input: Node) -> ParseResult<ast::Id> {
        Ok(ast::Id::new(
            input.as_str(),
            Some(Span::new(input.as_span(), Rc::clone(input.user_data()))),
        ))
    }

    fn bitwidth(input: Node) -> ParseResult<u64> {
        Ok(match input.as_str().parse::<u64>() {
            Ok(x) => x,
            _ => panic!("Unable to parse '{}' as a u64", input.as_str()),
        })
    }

    fn num_lit(input: Node) -> ParseResult<BitNum> {
        let raw = input.as_str();
        if raw.contains("'d") {
            match raw.split("'d").collect::<Vec<_>>().as_slice() {
                [bits, val] => Ok(BitNum {
                    width: bits.parse().unwrap(),
                    num_type: NumType::Decimal,
                    val: val.parse().unwrap(),
                    span: Some(Span::new(
                        input.as_span(),
                        Rc::clone(input.user_data()),
                    )),
                }),
                _ => unreachable!(),
            }
        } else if raw.contains("'b") {
            match raw.split("'b").collect::<Vec<_>>().as_slice() {
                [bits, val] => Ok(BitNum {
                    width: bits.parse().unwrap(),
                    num_type: NumType::Binary,
                    val: u64::from_str_radix(val, 2).unwrap(),
                    span: Some(Span::new(
                        input.as_span(),
                        Rc::clone(input.user_data()),
                    )),
                }),
                _ => unreachable!(),
            }
        } else if raw.contains("'x") {
            match raw.split("'x").collect::<Vec<_>>().as_slice() {
                [bits, val] => Ok(BitNum {
                    width: bits.parse().unwrap(),
                    num_type: NumType::Hex,
                    val: u64::from_str_radix(val, 16).unwrap(),
                    span: Some(Span::new(
                        input.as_span(),
                        Rc::clone(input.user_data()),
                    )),
                }),
                _ => unreachable!(),
            }
        } else if raw.contains("'o") {
            match raw.split("'o").collect::<Vec<_>>().as_slice() {
                [bits, val] => Ok(BitNum {
                    width: bits.parse().unwrap(),
                    num_type: NumType::Octal,
                    val: u64::from_str_radix(val, 8).unwrap(),
                    span: Some(Span::new(
                        input.as_span(),
                        Rc::clone(input.user_data()),
                    )),
                }),
                _ => unreachable!(),
            }
        } else {
            unreachable!()
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

    fn signature(input: Node) -> ParseResult<lib::ParamSignature> {
        Ok(match_nodes!(
            input.into_children();
            [io_ports(inputs), signature_return(outputs)] => lib::ParamSignature {
                inputs,
                outputs
            },
            [io_ports(inputs)] => lib::ParamSignature {
                inputs,
                outputs: vec![]
            },
            [signature_return(outputs)] => lib::ParamSignature {
                inputs: vec![],
                outputs
            },
            [] => lib::ParamSignature { inputs: vec![], outputs: vec![] }
        ))
    }

    fn signature_return(input: Node) -> ParseResult<Vec<lib::ParamPortdef>> {
        Ok(match_nodes!(
            input.into_children();
            [io_ports(p)] => p,
            [] => vec![]
        ))
    }

    fn io_port(input: Node) -> ParseResult<lib::ParamPortdef> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(id), bitwidth(bw)] => lib::ParamPortdef {
                name: id,
                width: lib::Width::Const { value: bw }
            },
            [identifier(id), identifier(param)] => lib::ParamPortdef {
                name: id,
                width: lib::Width::Param { value: param }
            }
        ))
    }

    fn io_ports(input: Node) -> ParseResult<Vec<lib::ParamPortdef>> {
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
            [identifier(id), identifier(name)] =>
                ast::Cell::decl(id, name)
        ))
    }

    fn cells(input: Node) -> ParseResult<Vec<ast::Cell>> {
        input
            .into_children()
            .map(|node| match node.as_rule() {
                Rule::primitive_cell => Self::primitive_cell(node),
                Rule::component_cell => Self::component_cell(node),
                _ => unreachable!(),
            })
            .collect()
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
        Ok(match_nodes!(
            input.into_children();
            [LHS(port)] => ast::Atom::Port(port),
            [num_lit(num)] => ast::Atom::Num(num)
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
            Rule::guard_or => Ok(ast::GuardExpr::Or(vec![l, r])),
            Rule::guard_and => Ok(ast::GuardExpr::And(vec![l, r])),
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

    fn params(input: Node) -> ParseResult<Vec<ast::Id>> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(id)..] => id.collect()
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

    fn connections(input: Node) -> ParseResult<Vec<ast::Connection>> {
        input
            .into_children()
            .map(|node| match node.as_rule() {
                Rule::wire => Ok(ast::Connection::Wire(Self::wire(node)?)),
                Rule::group => Ok(ast::Connection::Group(Self::group(node)?)),
                _ => unreachable!(),
            })
            .collect()
    }

    fn inner_wrap(input: Node) -> ParseResult<String> {
        // remove extra whitespace and indentation
        let mut result = String::new();
        // records the base indentation level
        let mut indent_level: Option<usize> = None;
        for line in input.as_str().lines() {
            // find the first non-empty line
            if !line.is_empty() && indent_level.is_none() {
                indent_level = line.find(|s| !char::is_whitespace(s));
            }

            // if we have already found indent level
            if indent_level.is_some() {
                result += indent_level
                    .map(|pre| {
                        if line.len() > pre {
                            line.split_at(pre).1
                        } else {
                            line
                        }
                    })
                    .unwrap_or(line)
                    .trim_end();
                result += "\n";
            }
        }
        Ok(result.trim_end().to_string())
    }

    fn prim_block(input: Node) -> ParseResult<String> {
        Ok(match_nodes!(
            input.into_children();
            [inner_wrap(text)] => text
        ))
    }

    fn verilog_block(input: Node) -> ParseResult<lib::Verilog> {
        Ok(match_nodes!(
            input.into_children();
            [prim_block(code)] => lib::Verilog { code }
        ))
    }

    fn implementation(input: Node) -> ParseResult<Vec<lib::Implementation>> {
        input
            .into_children()
            .map(|node| {
                Ok(match node.as_rule() {
                    Rule::verilog_block => lib::Implementation::Verilog {
                        data: Self::verilog_block(node)?,
                    },
                    _ => unreachable!(),
                })
            })
            .collect()
    }

    fn enable(input: Node) -> ParseResult<ast::Enable> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name)] => ast::Enable { comp: name }
        ))
    }

    fn seq(input: Node) -> ParseResult<ast::Seq> {
        Ok(match_nodes!(
            input.into_children();
            [stmt(stmt)..] => ast::Seq {
                stmts: stmt.collect()
            }
        ))
    }

    fn par(input: Node) -> ParseResult<ast::Par> {
        Ok(match_nodes!(
            input.into_children();
            [stmt(stmt)..] => ast::Par {
                stmts: stmt.collect()
            }
        ))
    }

    fn if_stmt(input: Node) -> ParseResult<ast::If> {
        Ok(match_nodes!(
            input.into_children();
            [port(port), identifier(cond), stmt(stmt)] => ast::If {
                port,
                cond,
                tbranch: Box::new(stmt),
                fbranch: Box::new(ast::Control::empty())
            },
            [port(port), identifier(cond), stmt(tbranch), stmt(fbranch)] => ast::If {
                port,
                cond,
                tbranch: Box::new(tbranch),
                fbranch: Box::new(fbranch)
            },
            [port(port), identifier(cond), stmt(tbranch), if_stmt(fbranch)] => ast::If {
                port,
                cond,
                tbranch: Box::new(tbranch),
                fbranch: Box::new(ast::Control::If { data: fbranch } )
            },

        ))
    }

    fn while_stmt(input: Node) -> ParseResult<ast::While> {
        Ok(match_nodes!(
            input.into_children();
            [port(port), identifier(cond), stmt(stmt)] => ast::While {
                port,
                cond,
                body: Box::new(stmt),
            }
        ))
    }

    fn stmt(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [enable(data)] => ast::Control::Enable { data },
            [seq(data)] => ast::Control::Seq { data },
            [par(data)] => ast::Control::Par { data },
            [if_stmt(data)] => ast::Control::If { data },
            [while_stmt(data)] => ast::Control::While { data },
        ))
    }

    fn control(input: Node) -> ParseResult<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [stmt(stmt)] => stmt,
            [] => ast::Control::empty()
        ))
    }

    fn component(input: Node) -> ParseResult<ast::ComponentDef> {
        Ok(match_nodes!(
        input.into_children();
        [identifier(id), signature(sig), cells(cells), connections(connections), control(control)] =>
            ast::ComponentDef {
                name: id,
                signature: sig.to_signature().unwrap(),
                cells,
                connections,
                control,
            },
            [identifier(id), cells(cells), connections(connections), control(control)] =>
                ast::ComponentDef {
                    name: id,
                    signature: ast::Signature {
                        inputs: vec![],
                        outputs: vec![]
                    },
                    cells,
                    connections,
                    control,
                },
        ))
    }

    fn primitive(input: Node) -> ParseResult<lib::Primitive> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), attributes(attrs), params(p), signature(s), implementation(i)] => lib::Primitive {
                name,
                params: p,
                signature: s,
                attributes: attrs,
                implementation: i
            },
            [identifier(name), attributes(attrs), signature(s), implementation(i)] => lib::Primitive {
                name,
                params: vec![],
                signature: s,
                attributes: attrs,
                implementation: i
            },
            [identifier(name), params(p), signature(s), implementation(i)] => lib::Primitive {
                name,
                params: p,
                signature: s,
                attributes: HashMap::new(),
                implementation: i
            },
            [identifier(name), signature(s), implementation(i)] => lib::Primitive {
                name,
                params: vec![],
                signature: s,
                attributes: HashMap::new(),
                implementation: i
            }
        ))
    }

    fn definition(input: Node) -> ParseResult<ast::Definition> {
        Ok(match_nodes!(
            input.into_children();
            [component(c)] => ast::Definition::Component(c),
            [primitive(p)] => ast::Definition::Primitive(p)
        ))
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
            [imports(imports), definition(defns).., EOI] => {
                let mut primitives = Vec::new();
                let mut components = Vec::new();
                for def in defns {
                    match def {
                        ast::Definition::Primitive(p) => primitives.push(p),
                        ast::Definition::Component(c) => components.push(c),
                    }
                }
                ast::NamespaceDef {
                    imports: imports,
                    components,
                    primitives
                }
            }
        ))
    }
}
