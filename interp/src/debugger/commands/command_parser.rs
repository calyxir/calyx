use super::core::{
    Command, ParsedBreakPointID, ParsedGroupName, PrintMode, WatchPosition,
};
use pest_consume::{match_nodes, Error, Parser};

type ParseResult<T> = std::result::Result<T, Error<Rule>>;
type Node<'i> = pest_consume::Node<'i, Rule, ()>;

use crate::{errors::InterpreterResult, serialization::PrintCode};

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("commands.pest");

#[derive(Parser)]
#[grammar = "debugger/commands/commands.pest"]
pub struct CommandParser;

#[pest_consume::parser]
impl CommandParser {
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn code_calyx(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    // ----------------------

    fn help(_input: Node) -> ParseResult<Command> {
        Ok(Command::Help)
    }

    fn cont(_input: Node) -> ParseResult<Command> {
        Ok(Command::Continue)
    }

    fn step(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
            [num(n)] => Command::Step(n),
            [] => Command::Step(1)
        ))
    }

    fn display(_input: Node) -> ParseResult<Command> {
        Ok(Command::Display)
    }

    fn info_break(_input: Node) -> ParseResult<Command> {
        Ok(Command::InfoBreak)
    }

    fn info_watch(_input: Node) -> ParseResult<Command> {
        Ok(Command::InfoWatch)
    }

    fn exit(_input: Node) -> ParseResult<Command> {
        Ok(Command::Exit)
    }

    fn comm_where(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
            [code_calyx(_)] => Command::PrintPC(true),
            [] => Command::PrintPC(false),
        ))
    }

    fn pc_un(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn pc_s(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn before(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn after(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn watch_position(input: Node) -> ParseResult<WatchPosition> {
        Ok(match_nodes!(input.into_children();
            [before(_)] => WatchPosition::Before,
            [after(_)] => WatchPosition::After
        ))
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

    fn identifier(input: Node) -> ParseResult<String> {
        Ok(input.as_str().to_owned())
    }

    fn group(input: Node) -> ParseResult<ParsedGroupName> {
        Ok(match_nodes!(input.into_children();
            [identifier(i)] => ParsedGroupName::from_group_name(i),
            [identifier(comp), identifier(group)] => ParsedGroupName::from_comp_and_group(comp, group)
        ))
    }

    fn num(input: Node) -> ParseResult<u32> {
        input
            .as_str()
            .parse::<u32>()
            .map_err(|_| input.error("Expected non-negative number"))
    }

    fn brk_id(input: Node) -> ParseResult<ParsedBreakPointID> {
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

    fn name(input: Node) -> ParseResult<Vec<String>> {
        Ok(match_nodes!(input.into_children();
                [identifier(ident)..] => ident.collect()
        ))
    }

    fn print(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
            [print_code(pc), name(ident)..] => Command::Print(ident.collect::<Vec<_>>(), Some(pc), PrintMode::Port),
            [name(ident)..] => Command::Print(ident.collect::<Vec<_>>(), None, PrintMode::Port),
        ))
    }

    fn print_state(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
            [print_code(pc), name(ident)..] => Command::Print(ident.collect::<Vec<_>>(), Some(pc), PrintMode::State),
            [name(ident)..] => Command::Print(ident.collect::<Vec<_>>(), None, PrintMode::State),
        ))
    }

    fn print_fail(input: Node) -> ParseResult<Error<Rule>> {
        Ok(match_nodes!(input.children();
            [print_code(_)] => input.error("Command requires a target"),
            [pc_fail(n)] => n.error("Invalid formatting code"),
            [] => input.error("Command requires a target")
        ))
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

    fn delete_watch(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
                [brk_id(br)..] => Command::DeleteWatch(br.collect())
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

    fn enable_watch(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
                [brk_id(br)..] => Command::EnableWatch(br.collect())
        ))
    }

    fn disable_watch(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
                [brk_id(br)..] => Command::DisableWatch(br.collect())
        ))
    }

    fn explain(_input: Node) -> ParseResult<Command> {
        Ok(Command::Explain)
    }

    fn restart(_input: Node) -> ParseResult<Command> {
        Ok(Command::Restart)
    }

    fn watch(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
        [watch_position(wp), group(g), print_state(p)] => {
            if let Command::Print(target, code, _) = p {
                Command::Watch(g, wp, target, code, PrintMode::State)
            } else {
                unreachable!("Parse produced wrong command?")
            }
            },
        [watch_position(wp), group(g), print(p)] => {
                if let Command::Print(target, code, _) = p {
                    Command::Watch(g, wp, target, code, PrintMode::Port)
                } else {
                    unreachable!("Parse produced wrong command?")
                }
            },
        [group(g), print_state(p)] => {
            if let Command::Print(target, code, _) = p {
                Command::Watch(g, WatchPosition::default(), target, code, PrintMode::State)
            } else {
                unreachable!("Parse produced wrong command?")
            }
            },
        [group(g), print(p)] => {
                if let Command::Print(target, code, _) = p {
                    Command::Watch(g, WatchPosition::default(), target, code, PrintMode::Port)
                } else {
                    unreachable!("Parse produced wrong command?")
                }
            }
        ))
    }

    fn command(input: Node) -> ParseResult<Command> {
        Ok(match_nodes!(input.into_children();
            [watch(w), EOI(_)] => w,
            [print_state(p), EOI(_)] => p,
            [print(p), EOI(_)] => p,
            [print_fail(err), EOI(_)] => ParseResult::Err(err)?,
            [step_over(s), EOI(_)] => s,
            [step(s), EOI(_)] => s,
            [cont(c), EOI(_)] => c,
            [comm_where(w), EOI(_)] => w,
            [help(h), EOI(_)] => h,
            [display(disp), EOI(_)] => disp,
            [brk(b), EOI(_)] => b,
            [info_break(ib), EOI(_)] => ib,
            [info_watch(iw), EOI(_)] => iw,
            [delete(del), EOI(_)] => del,
            [delete_watch(del), EOI(_)] => del,
            [enable_watch(ew), EOI(_)] => ew,
            [disable_watch(dw), EOI(_)] => dw,
            [enable(e), EOI(_)] => e,
            [disable(dis), EOI(_)] => dis,
            [exit(exit), EOI(_)] => exit,
            [explain(ex), EOI(_)] => ex,
            [restart(restart), EOI(_)] => restart,
            [EOI(_)] => Command::Empty,
        ))
    }
}

pub fn parse_command(input_str: &str) -> InterpreterResult<Command> {
    let inputs = CommandParser::parse(Rule::command, input_str)?;
    let input = inputs.single()?;
    Ok(CommandParser::command(input)?)
}
