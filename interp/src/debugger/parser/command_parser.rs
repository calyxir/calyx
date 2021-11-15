use super::super::commands::{BreakPointId, Command, GroupName};
use calyx::ir::Id;
use pest_consume::{match_nodes, Error, Parser};

type ParseResult<T> = std::result::Result<T, Error<Rule>>;
type Node<'i> = pest_consume::Node<'i, Rule, ()>;

use crate::{debugger::commands::PrintCode, errors::InterpreterResult};

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("commands.pest");

#[derive(Parser)]
#[grammar = "debugger/parser/commands.pest"]
pub struct CommandParser;

#[pest_consume::parser]
impl CommandParser {
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    // ----------------------

    fn help(_input: Node) -> ParseResult<Command> {
        Ok(Command::Help)
    }

    fn cont(_input: Node) -> ParseResult<Command> {
        Ok(Command::Continue)
    }

    fn step(_input: Node) -> ParseResult<Command> {
        Ok(Command::Step)
    }

    fn display(_input: Node) -> ParseResult<Command> {
        Ok(Command::Display)
    }

    fn info_break(_input: Node) -> ParseResult<Command> {
        Ok(Command::InfoBreak)
    }

    fn exit(_input: Node) -> ParseResult<Command> {
        Ok(Command::Exit)
    }

    fn pc_un(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn pc_s(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn pc_ufx(input: Node) -> ParseResult<usize> {
        Ok(match_nodes!(input.into_children();
            [num(n)] => n as usize
        ))
    }

    fn pc_sfx(input: Node) -> ParseResult<usize> {
        Ok(match_nodes!(input.into_children();
            [num(n)] => n as usize
        ))
    }

    fn pc_fail(input: Node) -> ParseResult<Node> {
        Ok(input)
    }

    fn print_code(input: Node) -> ParseResult<PrintCode> {
        match_nodes!(input.into_children();
            [pc_s(_)] => Ok(PrintCode::Signed),
            [pc_un(_)] => Ok(PrintCode::Unsigned),
            [pc_ufx(n)] => Ok(PrintCode::UFixed(n)),
            [pc_sfx(n)] => Ok(PrintCode::SFixed(n)),
        )
    }

    // ----------------------

    fn identifier(input: Node) -> ParseResult<Id> {
        Ok(Id::new(input.as_str(), None))
    }

    fn group(input: Node) -> ParseResult<GroupName> {
        Ok(match_nodes!(input.into_children();
            [identifier(ident)..] => GroupName(ident.collect::<Vec<_>>())
        ))
    }

    fn num(input: Node) -> ParseResult<u64> {
        input
            .as_str()
            .parse::<u64>()
            .map_err(|_| input.error("Expected non-negative number"))
    }

    fn brk_id(input: Node) -> ParseResult<BreakPointId> {
        Ok(match_nodes!(input.into_children();
                [num(n)] => n.into(),
                [group(g)] => g.into()
        ))
    }

    fn brk(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
            [group(g)..] => Command::Break(g.collect()),
        ))
    }

    fn name(input: Node) -> ParseResult<Vec<Id>> {
        Ok(match_nodes!(input.into_children();
                [identifier(ident)..] => ident.collect()
        ))
    }

    fn print(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
            [print_code(pc), name(ident)..] => Command::Print(Some(ident.collect::<Vec<_>>()), Some(pc)),
            [name(ident)..] => Command::Print(Some(ident.collect::<Vec<_>>()), None),
            [pc_fail(n)] => return Err(n.error("Invalid formatting code")),
            [pc_fail(n), _] => return Err(n.error("Invalid formatting code"))
        ))
    }

    fn print_state(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
            [print_code(pc), name(ident)..] => Command::PrintState(Some(ident.collect::<Vec<_>>()), Some(pc)),
            [name(ident)..] => Command::PrintState(Some(ident.collect::<Vec<_>>()), None),
            [pc_fail(n)] => return Err(n.error("Invalid formatting code")),
            [pc_fail(n), _] => return Err(n.error("Invalid formatting code"))
        ))
    }

    fn print_fail(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn step_over(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
            [group(g)] => Command::StepOver(g)
        ))
    }

    fn delete(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
                [brk_id(br)..] => Command::Delete(br.collect())
        ))
    }

    fn enable(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
                [brk_id(br)..] => Command::Enable(br.collect())
        ))
    }

    fn disable(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
                [brk_id(br)..] => Command::Disable(br.collect())
        ))
    }

    fn command(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
            [print_state(p), EOI(_)] => p,
            [print(p), EOI(_)] => p,
            [print_fail(_), EOI(_)] => Command::Print(None, None),
            [step_over(s), EOI(_)] => s,
            [step(s), EOI(_)] => s,
            [cont(c), EOI(_)] => c,
            [help(h), EOI(_)] => h,
            [display(disp), EOI(_)] => disp,
            [brk(b), EOI(_)] => b,
            [info_break(ib), EOI(_)] => ib,
            [delete(del), EOI(_)] => del,
            [enable(e), EOI(_)] => e,
            [disable(dis), EOI(_)] => dis,
            [exit(exit), EOI(_)] => exit,
            [EOI(_)] => Command::Empty,
        ))
    }
}

pub fn parse_command(input_str: &str) -> InterpreterResult<Command> {
    let inputs = CommandParser::parse(Rule::command, input_str)?;
    let input = inputs.single()?;
    Ok(CommandParser::command(input)?)
}
