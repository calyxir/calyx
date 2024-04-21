//Placeholder name
pub struct MetaData {
    // This is the next line that will be 'executed'
    _current_line: i64,
    // The next instruction that will be 'executed'
    _current_instruction: DisassembledInstruction,
    //Do we need to recreate these?
    _breakpoints: (),
    //Thread States(?)
    _thread_states: (),
    _events: Vec<DebuggingEvent>,
    //Perhaps all groups in a file and their locations? Can be helpful for
    // knowing the next instruction? Need to communicate step into?
    _groups: (),
    //Not sure if we need but it shows on debugger, probably in the future!
    _stack_frames: Vec<StackFrame>,
    //Do we need to communicate variables and their values?
    //Or worry about memory?
    _variables: Vec<Variable>,
}

//Maybe not enum...
pub enum DebuggingEvent {
    ProgramStart,
    ProgramExit,
    BreakpointHit,
    //etc
}
pub struct Variable {
    name: String,
    //Generic or just int?
    val: T,
}

pub struct StackFrame {
    index: i64,
    name: String,
    file: String,
    instruction: i64,
}

//Add getters and setters??
