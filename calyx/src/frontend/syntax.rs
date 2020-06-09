use crate::errors::{Result, Span};
use crate::lang::{
    ast,
    ast::{BitNum, NumType},
};
use pest_consume::{match_nodes, Error, Parser};
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

type ParseResult<T> = std::result::Result<T, Error<Rule>>;
// user data is the input program so that we can create Ast::id's
// that have a reference to the input string
type Node<'i> = pest_consume::Node<'i, Rule, Rc<String>>;

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("futil_syntax.pest");

#[derive(Parser)]
#[grammar = "frontend/futil_syntax.pest"]
pub struct FutilParser;

impl FutilParser {
    pub fn parse_file(path: &PathBuf) -> Result<ast::NamespaceDef> {
        let content = &fs::read(path)?;
        let string_content = std::str::from_utf8(content)?;
        let inputs = FutilParser::parse_with_userdata(
            Rule::file,
            string_content,
            Rc::new(string_content.to_string()),
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
                    span: Span::new(
                        input.as_span(),
                        Rc::clone(input.user_data()),
                    ),
                }),
                _ => unreachable!(),
            }
        } else if raw.contains("'b") {
            match raw.split("'b").collect::<Vec<_>>().as_slice() {
                [bits, val] => Ok(BitNum {
                    width: bits.parse().unwrap(),
                    num_type: NumType::Binary,
                    val: u64::from_str_radix(val, 2).unwrap(),
                    span: Span::new(
                        input.as_span(),
                        Rc::clone(input.user_data()),
                    ),
                }),
                _ => unreachable!(),
            }
        } else if raw.contains("'x") {
            match raw.split("'x").collect::<Vec<_>>().as_slice() {
                [bits, val] => Ok(BitNum {
                    width: bits.parse().unwrap(),
                    num_type: NumType::Hex,
                    val: u64::from_str_radix(val, 16).unwrap(),
                    span: Span::new(
                        input.as_span(),
                        Rc::clone(input.user_data()),
                    ),
                }),
                _ => unreachable!(),
            }
        } else if raw.contains("'o") {
            match raw.split("'o").collect::<Vec<_>>().as_slice() {
                [bits, val] => Ok(BitNum {
                    width: bits.parse().unwrap(),
                    num_type: NumType::Octal,
                    val: u64::from_str_radix(val, 8).unwrap(),
                    span: Span::new(
                        input.as_span(),
                        Rc::clone(input.user_data()),
                    ),
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

    fn signature(input: Node) -> ParseResult<ast::Signature> {
        Ok(match_nodes!(
            input.into_children();
            [io_ports(ins), io_ports(outs)] => ast::Signature {
                inputs: ins,
                outputs: outs
            },
            [io_ports(ins)] => ast::Signature {
                inputs: ins,
                outputs: vec![]
            }
        ))
    }

    fn io_port(input: Node) -> ParseResult<ast::Portdef> {
        Ok(match_nodes![
            input.into_children();
            [identifier(id), bitwidth(bw)] => ast::Portdef { name: id, width: bw }])
    }

    fn io_ports(input: Node) -> ParseResult<Vec<ast::Portdef>> {
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

    fn comparator(
        input: Node,
    ) -> ParseResult<impl Fn(ast::Atom, ast::Atom) -> ast::GuardExpr> {
        Ok(match input.as_str() {
            "==" => ast::GuardExpr::Eq,
            "!=" => ast::GuardExpr::Neq,
            "<" => ast::GuardExpr::Lt,
            ">" => ast::GuardExpr::Gt,
            "<=" => ast::GuardExpr::Leq,
            ">=" => ast::GuardExpr::Geq,
            _ => unreachable!(),
        })
    }

    fn not_expr(input: Node) -> ParseResult<ast::GuardExpr> {
        Ok(match_nodes!(
            input.into_children();
            [expr(e)] => ast::GuardExpr::Not(e)
        ))
    }

    fn guard_expr(input: Node) -> ParseResult<ast::GuardExpr> {
        Ok(match_nodes!(
            input.into_children();
            [expr(e1), comparator(c), expr(e2)] => c(e1, e2),
            [expr(e)] => ast::GuardExpr::Atom(e),
            [not_expr(e)] => e
        ))
    }

    fn guard(input: Node) -> ParseResult<Vec<ast::GuardExpr>> {
        Ok(match_nodes!(
            input.into_children();
            [guard_expr(gs)..] =>  gs.collect()
        ))
    }

    fn switch_stmt(input: Node) -> ParseResult<ast::Guard> {
        Ok(match_nodes!(
            input.into_children();
            [guard(guard), expr(expr)] => ast::Guard { guard, expr },
        ))
    }

    fn wire(input: Node) -> ParseResult<ast::Wire> {
        Ok(match_nodes!(
            input.into_children();
            [LHS(dest), expr(expr)] => ast::Wire {
                src: ast::Guard { guard: vec![], expr },
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
            [identifier(name), wire(wire)..] => ast::Group {
                name,
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

    fn if_cond(input: Node) -> ParseResult<Option<ast::Id>> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(cond)] => Some(cond),
            [] => None
        ))
    }

    fn if_stmt(input: Node) -> ParseResult<ast::If> {
        Ok(match_nodes!(
            input.into_children();
            [port(port), if_cond(cond), stmt(stmt)] => ast::If {
                port,
                cond,
                tbranch: Box::new(stmt),
                fbranch: Box::new(ast::Control::empty())
            },
            [port(port), if_cond(cond), stmt(tbranch), stmt(fbranch)] => ast::If {
                port,
                cond,
                tbranch: Box::new(tbranch),
                fbranch: Box::new(fbranch)
            },
            [port(port), if_cond(cond), stmt(tbranch), if_stmt(fbranch)] => ast::If {
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
            [port(port), stmt(stmt)] => ast::While {
                port,
                cond: None,
                body: Box::new(stmt),
            },
            [port(port), identifier(cond), stmt(stmt)] => ast::While {
                port,
                cond: Some(cond),
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
                signature: sig,
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
