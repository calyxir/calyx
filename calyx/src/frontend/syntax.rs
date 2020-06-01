use crate::lang::ast;
use pest_consume::{match_nodes, Error, Parser};
use std::fs;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, Error<Rule>>;
type Node<'i> = pest_consume::Node<'i, Rule, ()>;

const _GRAMMAR: &str = include_str!("grammar.pest");

#[derive(Parser)]
#[grammar = "frontend/grammar.pest"]
pub struct FutilParser;

#[pest_consume::parser]
impl FutilParser {
    fn EOI(_input: Node) -> Result<()> {
        Ok(())
    }

    fn identifier(input: Node) -> Result<ast::Id> {
        Ok(input.as_str().into())
    }

    fn bitwidth(input: Node) -> Result<u64> {
        Ok(input.as_str().parse::<u64>().unwrap())
    }

    fn num_lit(input: Node) -> Result<u64> {
        Ok(match input.as_str().parse::<u64>() {
            Ok(x) => x,
            _ => panic!("Unable to parse '{}' as a u64", input.as_str()),
        })
    }

    fn signature(input: Node) -> Result<ast::Signature> {
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

    fn io_port(input: Node) -> Result<ast::Portdef> {
        Ok(match_nodes![
            input.into_children();
            [identifier(id), bitwidth(bw)] => ast::Portdef { name: id, width: bw }])
    }

    fn io_ports(input: Node) -> Result<Vec<ast::Portdef>> {
        Ok(match_nodes![
            input.into_children();
            [io_port(p)..] => p.collect()])
    }

    fn args(input: Node) -> Result<Vec<u64>> {
        Ok(match_nodes!(
            input.into_children();
            [bitwidth(bw)..] => bw.collect(),
            [] => vec![]
        ))
    }

    fn primitive_cell(input: Node) -> Result<ast::Cell> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(id), identifier(prim), args(args)] =>
            ast::Cell::prim(id, prim, args)
        ))
    }

    fn component_cell(input: Node) -> Result<ast::Cell> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(id), identifier(name)] =>
                ast::Cell::decl(id, name)
        ))
    }

    fn cells(input: Node) -> Result<Vec<ast::Cell>> {
        input
            .into_children()
            .map(|node| match node.as_rule() {
                Rule::primitive_cell => Self::primitive_cell(node),
                Rule::component_cell => Self::component_cell(node),
                _ => unreachable!(),
            })
            .collect()
    }

    fn port(input: Node) -> Result<ast::Port> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(component), identifier(port)] => ast::Port::Comp { component, port },
            [identifier(port)] => ast::Port::This { port }
        ))
    }

    fn hole(input: Node) -> Result<ast::Port> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(group), identifier(name)] => ast::Port::Hole { group, name }
        ))
    }

    fn LHS(input: Node) -> Result<ast::Port> {
        Ok(match_nodes!(
            input.into_children();
            [port(port)] => port,
            [hole(hole)] => hole
        ))
    }

    fn expr(input: Node) -> Result<ast::Atom> {
        Ok(match_nodes!(
            input.into_children();
            [LHS(port)] => ast::Atom::Port(port),
            [num_lit(num)] => ast::Atom::Num(num)
        ))
    }

    fn comparator(
        input: Node,
    ) -> Result<impl Fn(ast::Atom, ast::Atom) -> ast::GuardExpr> {
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

    fn guard_expr(input: Node) -> Result<ast::GuardExpr> {
        Ok(match_nodes!(
            input.into_children();
            [expr(e1), comparator(c), expr(e2)] => c(e1, e2),
            [expr(e)] => ast::GuardExpr::Atom(e)
        ))
    }

    fn guard(input: Node) -> Result<ast::Guard> {
        Ok(match_nodes!(
            input.into_children();
            [guard_expr(gs)..] => ast::Guard { exprs: gs.collect() }
        ))
    }

    fn switch_stmt(input: Node) -> Result<(ast::Guard, ast::Atom)> {
        Ok(match_nodes!(
            input.into_children();
            [guard(guard), expr(expr)] => (guard, expr),
        ))
    }

    fn switch(input: Node) -> Result<Vec<(ast::Guard, ast::Atom)>> {
        Ok(match_nodes!(
            input.into_children();
            [switch_stmt(sw)..] => sw.collect(),
        ))
    }

    fn wire(input: Node) -> Result<ast::Wire> {
        Ok(match_nodes!(
            input.into_children();
            [LHS(dest), expr(expr)] => ast::Wire {
                src: vec![(ast::Guard { exprs: vec![] }, expr)],
                dest
            },
            [LHS(dest), switch(switch)] => ast::Wire {
                src: switch,
                dest
            }
        ))
    }

    fn group(input: Node) -> Result<ast::Group> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), wire(wire)..] => ast::Group {
                name,
                wires: wire.collect()
            }
        ))
    }

    fn connections(input: Node) -> Result<Vec<ast::Connection>> {
        input
            .into_children()
            .map(|node| match node.as_rule() {
                Rule::wire => Ok(ast::Connection::Wire(Self::wire(node)?)),
                Rule::group => Ok(ast::Connection::Group(Self::group(node)?)),
                _ => unreachable!(),
            })
            .collect()
    }

    fn enable(input: Node) -> Result<ast::Enable> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name)] => ast::Enable { comp: name }
        ))
    }

    fn seq(input: Node) -> Result<ast::Seq> {
        Ok(match_nodes!(
            input.into_children();
            [stmt(stmt)..] => ast::Seq {
                stmts: stmt.collect()
            }
        ))
    }

    fn par(input: Node) -> Result<ast::Par> {
        Ok(match_nodes!(
            input.into_children();
            [stmt(stmt)..] => ast::Par {
                stmts: stmt.collect()
            }
        ))
    }

    fn if_cond(input: Node) -> Result<Option<ast::Id>> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(cond)] => Some(cond),
            [] => None
        ))
    }

    fn if_stmt(input: Node) -> Result<ast::If> {
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

    fn while_stmt(input: Node) -> Result<ast::While> {
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

    fn stmt(input: Node) -> Result<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [enable(data)] => ast::Control::Enable { data },
            [seq(data)] => ast::Control::Seq { data },
            [par(data)] => ast::Control::Par { data },
            [if_stmt(data)] => ast::Control::If { data },
            [while_stmt(data)] => ast::Control::While { data },
        ))
    }

    fn control(input: Node) -> Result<ast::Control> {
        Ok(match_nodes!(
            input.into_children();
            [stmt(stmt)] => stmt
        ))
    }

    fn component(input: Node) -> Result<ast::ComponentDef> {
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

    fn file(input: Node) -> Result<ast::NamespaceDef> {
        Ok(ast::NamespaceDef {
            library: None,
            components: match_nodes!(
                input.into_children();
                [component(comps).., _] => comps.collect()
            ),
        })
    }
}

impl FutilParser {
    pub fn from_file(path: &PathBuf) -> Result<ast::NamespaceDef> {
        let content = &fs::read(path).unwrap();
        let string_content = std::str::from_utf8(content).unwrap();
        let inputs = FutilParser::parse(Rule::file, string_content)?;
        let input = inputs.single()?;
        FutilParser::file(input)
    }
}
