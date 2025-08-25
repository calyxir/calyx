use cider_idx::iter::SplitIndexRange;

use crate::{
    errors::RuntimeError,
    flatten::{
        flat_ir::indexes::{GlobalCellIdx, GlobalPortIdx, PortValue},
        primitives::{
            Primitive,
            macros::declare_ports,
            prim_trait::{UpdateResult, UpdateStatus},
        },
        structures::environment::{MemoryMap, PortMap},
    },
};

#[derive(Clone)]
pub struct UnsynAssert {
    base_port: GlobalPortIdx,
    cell_id: GlobalCellIdx,
    output: PortValue,
}

impl UnsynAssert {
    declare_ports![IN: 0, EN: 1, _CLK:2, _RESET: 3 | OUT: 4];
    pub fn new(base_port: GlobalPortIdx, cell_id: GlobalCellIdx) -> Self {
        Self {
            base_port,
            output: PortValue::new_undef(),
            cell_id,
        }
    }
}

impl Primitive for UnsynAssert {
    fn exec_comb(
        &self,
        port_map: &mut PortMap,
        _state_map: &MemoryMap,
    ) -> crate::flatten::primitives::prim_trait::UpdateResult {
        crate::flatten::primitives::macros::ports![&self.base_port;
            out: Self::OUT
        ];

        Ok(port_map.write_exact_unchecked(out, self.output.clone()))
    }

    fn exec_cycle(
        &mut self,
        port_map: &mut PortMap,
        _: &mut MemoryMap,
    ) -> UpdateResult {
        crate::flatten::primitives::macros::ports![&self.base_port;
            in_: Self::IN,
            en: Self::EN
        ];
        if port_map[en].as_bool().unwrap_or_default()
            && !port_map[in_].as_bool().unwrap_or_default()
        {
            Err(RuntimeError::AssertionError(self.cell_id).into())
        } else {
            self.output = port_map[in_].clone();
            Ok(UpdateStatus::Unchanged)
        }
    }

    fn clone_boxed(&self) -> Box<dyn Primitive> {
        Box::new(self.clone())
    }

    fn get_ports(&self) -> SplitIndexRange<GlobalPortIdx> {
        self.get_signature()
    }
}
