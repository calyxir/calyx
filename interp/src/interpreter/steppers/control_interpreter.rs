use super::AssignmentInterpreter;
use calyx::ir::{self, Assignment, Control, Group};

pub struct EmptyInterpreter {}
pub struct EnableInterpreter {}
pub struct SeqInterpreter {}
pub struct ParInterpreter {}
pub struct IfInterpreter {}
pub struct WhileInterpreter {}
pub struct InvokeInterpreter {}

pub enum ControlInterpreter {
    Empty(EnableInterpreter),
    Enable(EnableInterpreter),
    Seq(SeqInterpreter),
    Par(ParInterpreter),
    If(IfInterpreter),
    While(WhileInterpreter),
    Invoke(InvokeInterpreter),
}
