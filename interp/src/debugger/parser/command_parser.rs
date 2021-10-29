use super::commands::{BreakPointId, Command, GroupName};
use calyx::ir::Id;
use pest_consume::{match_nodes, Error, Parser};

type Result<T> = std::result::Result<T, Error<Rule>>;
type Node<'i> = pest_consume::Node<'i, Rule, ()>;

use crate::errors::{InterpreterError, InterpreterResult};

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("commands.pest");

#[derive(Parser)]
#[grammar = "debugger/parser/commands.pest"]
pub struct CommandParser;

#[pest_consume::parser]
impl CommandParser {
    fn EOI(_input: Node) -> Result<()> {
        Ok(())
    }

    // ----------------------

    fn help(_input: Node) -> Result<Command> {
        Ok(Command::Help)
    }

    fn cont(_input: Node) -> Result<Command> {
        Ok(Command::Continue)
    }

    fn step(_input: Node) -> Result<Command> {
        Ok(Command::Step)
    }

    fn display(_input: Node) -> Result<Command> {
        Ok(Command::Display)
    }

    fn info_break(_input: Node) -> Result<Command> {
        Ok(Command::InfoBreak)
    }

    fn exit(_input: Node) -> Result<Command> {
        Ok(Command::Exit)
    }

    // ----------------------

    fn identifier(input: Node) -> Result<Id> {
        Ok(Id::new(input.as_str(), None))
    }

    fn group(input: Node) -> Result<GroupName> {
        Ok(match_nodes!(input.into_children();
            [identifier(ident)..] => GroupName(ident.collect::<Vec<_>>())
        ))
    }

    fn num(input: Node) -> Result<u64> {
        // TODO (Griffin): Make this a proper error so the whole thing doesn't explode
        Ok(input.as_str().parse::<u64>().unwrap())
    }

    fn brk_id(input: Node) -> Result<BreakPointId> {
        Ok(match_nodes!(input.into_children();
                [num(n)] => n.into(),
                [group(g)] => g.into()
        ))
    }

    fn brk(input: Node) -> Result<Command> {
        Ok(match_nodes!(input.into_children();
            [group(g)..] => Command::Break(g.collect()),
        ))
    }

    fn print(input: Node) -> Result<Command> {
        Ok(match_nodes!(input.into_children();
                [identifier(ident)..] => Command::Print(Some(ident.collect::<Vec<_>>()))
        ))
    }

    fn print_fail(_input: Node) -> Result<()> {
        Ok(())
    }

    fn delete(input: Node) -> Result<Command> {
        Ok(match_nodes!(input.into_children();
                [brk_id(br)..] => Command::Delete(br.collect())
        ))
    }

    fn enable(input: Node) -> Result<Command> {
        Ok(match_nodes!(input.into_children();
                [brk_id(br)..] => Command::Enable(br.collect())
        ))
    }

    fn disable(input: Node) -> Result<Command> {
        Ok(match_nodes!(input.into_children();
                [brk_id(br)..] => Command::Disable(br.collect())
        ))
    }

    fn command(input: Node) -> Result<Command> {
        Ok(match_nodes!(input.into_children();
            [print(p), EOI(_)] => p,
            [print_fail(_), EOI(_)] => Command::Print(None),
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
    let inputs = CommandParser::parse(Rule::command, input_str);
    let input = inputs.map(|x| x.single());
    if let Ok(Ok(input)) = input {
        let result = CommandParser::command(input);
        if let Ok(comm) = result {
            return Ok(comm);
        }
    }
    Err(InterpreterError::UnknownCommand(input_str.to_string()))
}
