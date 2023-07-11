use crate::{
    flatten::{
        flat_ir::prelude::GlobalPortId,
        primitives::{
            declare_ports, output, ports, prim_trait::Results, Primitive,
        },
        structures::{environment::PortMap, index_trait::IndexRef},
    },
    primitives::{Entry, Serializable},
    values::Value,
};

pub struct StdReg {
    base_port: GlobalPortId,
    internal_state: Value,
}

impl StdReg {
    declare_ports![IN: 0, WRITE_EN: 1, CLK: 2, RESET: 3, OUT: 4, DONE: 5];

    pub fn new(base_port: GlobalPortId, width: u64) -> Self {
        let internal_state = Value::zeroes(width);
        Self {
            base_port,
            internal_state,
        }
    }
}

impl Primitive for StdReg {
    fn exec_cycle(&mut self, port_map: &PortMap) -> Results {
        ports![&self.base_port;
            input: Self::IN,
            write_en: Self::WRITE_EN,
            reset: Self::RESET,
            out: Self::OUT,
            done: Self::DONE
        ];

        let out = if port_map[reset].as_bool() {
            self.internal_state = Value::zeroes(self.internal_state.width());
            output![ out: self.internal_state.clone(), done: Value::bit_low() ]
        } else if port_map[write_en].as_bool() {
            self.internal_state = port_map[input].clone();
            output![ out: self.internal_state.clone(), done: Value::bit_high() ]
        } else {
            output![ out: self.internal_state.clone(), done: Value::bit_high() ]
        };

        Ok(out)
    }

    fn reset(&mut self) -> Results {
        ports![&self.base_port; done: Self::DONE];
        Ok(output![done: Value::bit_low()])
    }

    fn has_comb(&self) -> bool {
        false
    }

    fn serialize(
        &self,
        code: Option<crate::debugger::PrintCode>,
    ) -> Serializable {
        Serializable::Val(Entry::from_val_code(
            &self.internal_state,
            &code.unwrap_or_default(),
        ))
    }

    fn has_serializable_state(&self) -> bool {
        true
    }
}
