use btor2i::program::Btor2Program;

use crate::flatten::primitives::prim_trait::Primitive;

use std::collections::HashMap;

pub struct BTOR2Prim {
    program: Btor2Program,
    base_port: GlobalPortIdx, // stuff to add: input ports, output ports
                              // use the declare_ports! macro
                              // declare ports programatically
                              // input names gathered from names of ports in BTOR2 primitive; assigned programatically
                              // start by pre-hardcoding ports, only hand offsets to names.
}

pub struct MyBTOR2Add {
    program: Btor2Program,
    base_port: GlobalPortIdx,
    width: u32, // do stuff
}

impl MyBTOR2Add {
    declare_ports![ LEFT:0, RIGHT:1, OUT:2 ];
    pub fn new(base: GlobalPortIdx, width: u32) {
        Self {
            program: Btor2Program::new(),
            base_port: base,
            width: width,
        };
    }
}

impl Primitive for MyBTOR2Add {
    fn exec_comb(&self, _port_map: &mut PortMap) -> UpdateResult {
        ports![&self.base; left: Self::LEFT, right: Self::RIGHT, out: Self::OUT];
        // construct a hashmap from the names to the inputs
        let input_map = HashMap::from([
            ("left", port_map[left].as_str()),
            ("right", port_map[right].as_str()),
        ]);
        match self.program.run(inputs) {
            Ok(output_map) => Ok(port_map.insert_val(
                out,
                AssignedValue::cell_value(output_map["out"]),
            )?),
            Err(msg) => {
                port_map.write_undef(out)?;
                Ok(UpdateStatus::Unchanged)
            }
        }
    }

    fn reset(&mut self, _port_map: &mut PortMap) -> InterpreterResult<()> {
        Ok(())
    }

    fn has_stateful(&self) -> bool {
        false
    }
}
