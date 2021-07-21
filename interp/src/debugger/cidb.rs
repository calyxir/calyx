use super::commands::Command;
use super::io_utils::Input;
use crate::environment::InterpreterState;
use crate::interpreter::{ComponentInterpreter, Interpreter};
use calyx::ir;
pub struct Debugger<'a> {
    _context: &'a ir::Context,
    main_component: &'a ir::Component,
}

impl<'a> Debugger<'a> {
    pub fn new(
        context: &'a ir::Context,
        main_component: &'a ir::Component,
    ) -> Self {
        Self {
            _context: context,
            main_component,
        }
    }

    pub fn main_loop(&self, env: InterpreterState) -> InterpreterState {
        let control: &ir::Control = &self.main_component.control.borrow();
        let mut component_interpreter = ComponentInterpreter::from_component(
            &self.main_component,
            control,
            env,
        );
        let mut input_stream = Input::default();
        println!("== Calyx Interactive Debugger ==");
        loop {
            let comm = input_stream.next_command();

            match comm {
                Command::Step => component_interpreter.step(),
                Command::Continue => todo!(),
                Command::Empty => {}
            }
        }
    }
}
