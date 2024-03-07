use btor2i::program::Btor2Program;

use crate::flatten::flat_ir::prelude::AssignedValue;
use crate::flatten::flat_ir::prelude::GlobalPortIdx;
use crate::flatten::primitives::declare_ports;
use crate::flatten::primitives::ports;
use crate::flatten::primitives::prim_trait::Primitive;
use crate::flatten::primitives::prim_trait::UpdateResult;
use crate::flatten::primitives::prim_trait::UpdateStatus;
use crate::flatten::structures::environment::PortMap;

use crate::values::Value;

use std::cell::RefCell;
use std::collections::HashMap;

pub struct BTOR2Prim<'a> {
    program: Btor2Program<'a>,
    base_port: GlobalPortIdx, // stuff to add: input ports, output ports
                              // use the declare_ports! macro
                              // declare ports programatically
                              // input names gathered from names of ports in BTOR2 primitive; assigned programatically
                              // start by pre-hardcoding ports, only hand offsets to names.
}

pub struct MyBtor2Add<'a> {
    program: RefCell<Btor2Program<'a>>,
    base_port: GlobalPortIdx,
    width: usize, // do stuff
    loaded: bool,
}

impl<'a> MyBtor2Add<'a> {
    declare_ports![ LEFT:0, RIGHT:1, OUT:2 ];
    pub fn new(base: GlobalPortIdx, width: usize) -> Self {
        Self {
            program: RefCell::new(Btor2Program::new(
                "tools/btor2/core/std_add.btor",
            )),
            base_port: base,
            width,
            loaded: false,
        }
    }
}

impl<'a> Primitive for MyBtor2Add<'a> {
    fn exec_comb(&self, _port_map: &mut PortMap) -> UpdateResult {
        ports![&self.base_port; left: Self::LEFT, right: Self::RIGHT, out: Self::OUT];
        // let mut program_mut = RefCell::new(self.program);
        // construct a hashmap from the names to the inputs
        let input_map = HashMap::from([
            (
                "left".to_string(),
                _port_map[left].as_usize().unwrap().to_string(),
            ),
            (
                "right".to_string(),
                _port_map[right].as_usize().unwrap().to_string(),
            ),
        ]);
        match self.program.borrow_mut().run(input_map) {
            Ok(output_map) => Ok(_port_map.insert_val(
                out,
                AssignedValue::cell_value(Value::from(
                    output_map["out"],
                    self.width,
                )),
            )?),
            Err(_msg) => {
                _port_map.write_undef(out)?;
                Ok(UpdateStatus::Unchanged)
            }
        }
    }

    fn has_stateful(&self) -> bool {
        false
    }
}
