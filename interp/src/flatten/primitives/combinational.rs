use crate::{
    flatten::{
        flat_ir::prelude::GlobalPortId,
        primitives::{
            declare_ports, output, ports, prim_trait::Results, Primitive,
        },
        structures::environment::PortMap,
    },
    values::Value,
};

pub struct StdConst {
    value: Value,
    out: GlobalPortId,
}

impl StdConst {
    pub fn new(value: Value, out: GlobalPortId) -> Self {
        Self { value, out }
    }
}

impl Primitive for StdConst {
    fn exec_comb(&self, _port_map: &PortMap) -> Results {
        Ok(vec![])
    }

    fn exec_cycle(&mut self, _port_map: &PortMap) -> Results {
        Ok(vec![])
    }

    fn has_comb(&self) -> bool {
        false
    }

    fn has_stateful(&self) -> bool {
        false
    }
}

pub struct StdMux {
    base: GlobalPortId,
    width: u32,
}

impl StdMux {
    declare_ports![ COND: 0, TRU: 1, FAL:2, OUT: 3];
}

impl Primitive for StdMux {
    fn exec_comb(&self, port_map: &PortMap) -> Results {
        ports![&self.base; cond: Self::COND, tru: Self::TRU, fal: Self::FAL, out: Self::OUT];

        let out_idx = if port_map[cond].as_bool() { tru } else { fal };

        Ok(output![out: port_map[out_idx].clone()])
    }

    fn reset(&mut self) -> Results {
        ports![&self.base; out: Self::OUT];
        Ok(output![out: Value::zeroes(self.width)])
    }

    fn has_stateful(&self) -> bool {
        false
    }
}
