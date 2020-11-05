use::crate::ir

// used to evaluate a group; placeholder for now
pub struct GroupInterpreter {
    Group(ir::Group)

}

impl GroupInterpreter {
    fn eval(self, source_env : Vec<RRC<Port>>) -> Result<> {
       let dest_env = source_env.clone();
       // while the done signal is false
       while () {
           // check each assignment statement
           for statement : self.Group.assignments {
               // check the guard
               if (eval_guard (&statement.guard)) {
                   dest_env.insert(statement.dst, statement.src)
                   // don't set guard to false
               }
           }
        }
        dest_env
    }

    fn eval_guard(guard : &ir::Guard, ) -> bool {
        match guard {

        }
    }
}