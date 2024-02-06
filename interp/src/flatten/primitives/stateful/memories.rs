use crate::{
    errors::{InterpreterError, InterpreterResult},
    flatten::{
        flat_ir::prelude::{AssignedValue, GlobalPortIdx, PortValue},
        primitives::{
            declare_ports, make_getters, ports,
            prim_trait::{UpdateResult, UpdateStatus},
            Primitive,
        },
        structures::environment::PortMap,
    },
    primitives::{Entry, Serializable},
    values::Value,
};

pub struct StdReg {
    base_port: GlobalPortIdx,
    internal_state: Value,
    done_is_high: bool,
}

impl StdReg {
    declare_ports![IN: 0, WRITE_EN: 1, CLK: 2, RESET: 3, OUT: 4, DONE: 5];

    pub fn new(base_port: GlobalPortIdx, width: u32) -> Self {
        let internal_state = Value::zeroes(width);
        Self {
            base_port,
            internal_state,
            done_is_high: false,
        }
    }
}

impl Primitive for StdReg {
    fn exec_cycle(&mut self, port_map: &mut PortMap) -> UpdateResult {
        ports![&self.base_port;
            input: Self::IN,
            write_en: Self::WRITE_EN,
            reset: Self::RESET,
            out_idx: Self::OUT,
            done: Self::DONE
        ];

        let done_port = if port_map[reset].as_bool().unwrap_or_default() {
            self.internal_state = Value::zeroes(self.internal_state.width());
            port_map
                .insert_val(done, AssignedValue::cell_value(Value::bit_low()))?
        } else if port_map[write_en].as_bool().unwrap_or_default() {
            self.internal_state = port_map[input]
                .as_option()
                .ok_or(InterpreterError::UndefinedWrite)?
                .val()
                .clone();

            self.done_is_high = true;

            port_map.insert_val(
                done,
                AssignedValue::cell_value(Value::bit_high()),
            )? | port_map.insert_val(
                out_idx,
                AssignedValue::cell_value(self.internal_state.clone()),
            )?
        } else {
            self.done_is_high = false;
            port_map
                .insert_val(done, AssignedValue::cell_value(Value::bit_low()))?
        };

        Ok(done_port
            | port_map.insert_val(
                out_idx,
                AssignedValue::cell_value(self.internal_state.clone()),
            )?)
    }

    fn exec_comb(&self, port_map: &mut PortMap) -> UpdateResult {
        ports![&self.base_port;
            done: Self::DONE,
            out_idx: Self::OUT];
        let out_signal = port_map.insert_val(
            out_idx,
            AssignedValue::cell_value(self.internal_state.clone()),
        )?;
        let done_signal = port_map.insert_val(
            done,
            AssignedValue::cell_value(if self.done_is_high {
                Value::bit_high()
            } else {
                Value::bit_low()
            }),
        )?;

        Ok(out_signal | done_signal)
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

pub trait MemAddresser {
    const NON_ADDRESS_BASE: usize;

    fn calculate_addr(
        &self,
        port_map: &PortMap,
        base_port: GlobalPortIdx,
    ) -> Option<usize>;
}

pub struct MemD1<const SEQ: bool>;

impl<const SEQ: bool> MemAddresser for MemD1<SEQ> {
    fn calculate_addr(
        &self,
        port_map: &PortMap,
        base_port: GlobalPortIdx,
    ) -> Option<usize> {
        let addr0 = if SEQ {
            ports![&base_port; addr0: Self::SEQ_ADDR0];
            addr0
        } else {
            ports![&base_port; addr0: Self::COMB_ADDR0];
            addr0
        };

        port_map[addr0].as_usize()
    }

    const NON_ADDRESS_BASE: usize = if SEQ {
        Self::SEQ_ADDR0 + 1
    } else {
        Self::COMB_ADDR0 + 1
    };
}

impl<const SEQ: bool> MemD1<SEQ> {
    declare_ports![SEQ_ADDR0: 2, COMB_ADDR0: 0];
}

pub struct MemD2<const SEQ: bool> {
    d1_size: usize,
}

impl<const SEQ: bool> MemD2<SEQ> {
    declare_ports![SEQ_ADDR0: 2, COMB_ADDR0: 0, SEQ_ADDR1: 3, COMB_ADDR1: 1];
}

impl<const SEQ: bool> MemAddresser for MemD2<SEQ> {
    fn calculate_addr(
        &self,
        port_map: &PortMap,
        base_port: GlobalPortIdx,
    ) -> Option<usize> {
        let (addr0, addr1) = if SEQ {
            ports![&base_port;
                addr0: Self::SEQ_ADDR0,
                addr1: Self::SEQ_ADDR1];
            (addr0, addr1)
        } else {
            ports![&base_port;
                addr0: Self::COMB_ADDR0,
                addr1: Self::COMB_ADDR1];
            (addr0, addr1)
        };

        let a0 = port_map[addr0].as_usize()?;
        let a1 = port_map[addr1].as_usize()?;

        Some(a0 * self.d1_size + a1)
    }

    const NON_ADDRESS_BASE: usize = if SEQ {
        Self::SEQ_ADDR1 + 1
    } else {
        Self::COMB_ADDR1 + 1
    };
}

pub struct MemD3<const SEQ: bool> {
    d1_size: usize,
    d2_size: usize,
}

impl<const SEQ: bool> MemD3<SEQ> {
    declare_ports![SEQ_ADDR0: 2, COMB_ADDR0: 0,
                   SEQ_ADDR1: 3, COMB_ADDR1: 1,
                   SEQ_ADDR2: 4, COMB_ADDR2: 2];
}

impl<const SEQ: bool> MemAddresser for MemD3<SEQ> {
    fn calculate_addr(
        &self,
        port_map: &PortMap,
        base_port: GlobalPortIdx,
    ) -> Option<usize> {
        let (addr0, addr1, addr2) = if SEQ {
            ports![&base_port;
                addr0: Self::SEQ_ADDR0,
                addr1: Self::SEQ_ADDR1,
                addr2: Self::SEQ_ADDR2
            ];
            (addr0, addr1, addr2)
        } else {
            ports![&base_port;
                addr0: Self::COMB_ADDR0,
                addr1: Self::COMB_ADDR1,
                addr2: Self::COMB_ADDR2
            ];

            (addr0, addr1, addr2)
        };

        let a0 = port_map[addr0].as_usize()?;
        let a1 = port_map[addr1].as_usize()?;
        let a2 = port_map[addr2].as_usize()?;

        Some(a0 * (self.d1_size * self.d2_size) + a1 * self.d2_size + a2)
    }

    const NON_ADDRESS_BASE: usize = if SEQ {
        Self::SEQ_ADDR2 + 1
    } else {
        Self::COMB_ADDR2 + 1
    };
}

pub struct MemD4<const SEQ: bool> {
    d1_size: usize,
    d2_size: usize,
    d3_size: usize,
}

impl<const SEQ: bool> MemD4<SEQ> {
    declare_ports![
        SEQ_ADDR0: 2, COMB_ADDR0: 0,
        SEQ_ADDR1: 3, COMB_ADDR1: 1,
        SEQ_ADDR2: 4, COMB_ADDR2: 2,
        SEQ_ADDR3: 5, COMB_ADDR3: 3
    ];
}

impl<const SEQ: bool> MemAddresser for MemD4<SEQ> {
    fn calculate_addr(
        &self,
        port_map: &PortMap,
        base_port: GlobalPortIdx,
    ) -> Option<usize> {
        let (addr0, addr1, addr2, addr3) = if SEQ {
            ports![&base_port;
                addr0: Self::SEQ_ADDR0,
                addr1: Self::SEQ_ADDR1,
                addr2: Self::SEQ_ADDR2,
                addr3: Self::SEQ_ADDR3
            ];
            (addr0, addr1, addr2, addr3)
        } else {
            ports![&base_port;
                addr0: Self::COMB_ADDR0,
                addr1: Self::COMB_ADDR1,
                addr2: Self::COMB_ADDR2,
                addr3: Self::COMB_ADDR3
            ];

            (addr0, addr1, addr2, addr3)
        };

        let a0 = port_map[addr0].as_usize()?;
        let a1 = port_map[addr1].as_usize()?;
        let a2 = port_map[addr2].as_usize()?;
        let a3 = port_map[addr3].as_usize()?;

        Some(
            a0 * (self.d1_size * self.d2_size * self.d3_size)
                + a1 * (self.d2_size * self.d3_size)
                + a2 * self.d3_size
                + a3,
        )
    }

    const NON_ADDRESS_BASE: usize = if SEQ {
        Self::SEQ_ADDR3 + 1
    } else {
        Self::COMB_ADDR3 + 1
    };
}

pub struct CombMem<M: MemAddresser> {
    base_port: GlobalPortIdx,
    internal_state: Vec<Value>,
    allow_invalid_access: bool,
    width: u32,
    addresser: M,
}

impl<M: MemAddresser> CombMem<M> {
    declare_ports![
        WRITE_DATA: M::NON_ADDRESS_BASE + 1,
        WRITE_EN: M::NON_ADDRESS_BASE + 2,
        CLK: M::NON_ADDRESS_BASE + 3,
        RESET: M::NON_ADDRESS_BASE + 4,
        READ_DATA: M::NON_ADDRESS_BASE + 5,
        DONE: M::NON_ADDRESS_BASE + 6
    ];

    make_getters![base_port;
        write_data: Self::WRITE_DATA,
        write_en: Self::WRITE_EN,
        reset_port: Self::RESET,
        read_data: Self::READ_DATA,
        done: Self::DONE
    ];
}

impl<M: MemAddresser> Primitive for CombMem<M> {
    fn exec_comb(&self, port_map: &mut PortMap) -> UpdateResult {
        let addr = self.addresser.calculate_addr(port_map, self.base_port);
        let read_data = self.read_data();

        if addr.is_some() && addr.unwrap() < self.internal_state.len() {
            Ok(port_map.insert_val(
                read_data,
                AssignedValue::cell_value(
                    self.internal_state[addr.unwrap()].clone(),
                ),
            )?)
        }
        // either the address is undefined or it is outside the range of valid addresses
        else {
            // throw error on cycle boundary rather than here
            port_map.write_undef(read_data)?;
            Ok(UpdateStatus::Unchanged)
        }
    }

    fn exec_cycle(&mut self, port_map: &mut PortMap) -> UpdateResult {
        // These two behave like false when undefined
        let reset = port_map[self.reset_port()].as_bool().unwrap_or_default();
        let write_en = port_map[self.write_en()].as_bool().unwrap_or_default();

        let addr = self.addresser.calculate_addr(port_map, self.base_port);
        let (read_data, done) = (self.read_data(), self.done());

        let done = if write_en && !reset {
            let addr = addr.ok_or(InterpreterError::UndefinedWriteAddr)?;

            let write_data = port_map[self.write_data()]
                .as_option()
                .ok_or(InterpreterError::UndefinedWrite)?;
            self.internal_state[addr] = write_data.val().clone();
            port_map.insert_val(done, AssignedValue::cell_b_high())?
        } else {
            port_map.insert_val(done, AssignedValue::cell_b_low())?
        };

        if let Some(addr) = addr {
            Ok(port_map.insert_val(
                read_data,
                AssignedValue::cell_value(self.internal_state[addr].clone()),
            )? | done)
        } else {
            Ok(done)
        }
    }

    fn serialize(
        &self,
        _code: Option<crate::debugger::PrintCode>,
    ) -> Serializable {
        todo!("StdMemD1::serialize")
    }

    fn has_serializable_state(&self) -> bool {
        true
    }
}

// type aliases
pub type CombMemD1 = CombMem<MemD1<false>>;
pub type CombMemD2 = CombMem<MemD2<false>>;
pub type CombMemD3 = CombMem<MemD3<false>>;
pub type CombMemD4 = CombMem<MemD4<false>>;

impl CombMemD1 {
    pub fn new(
        base: GlobalPortIdx,
        width: u32,
        allow_invalid: bool,
        size: usize,
    ) -> Self {
        let internal_state = vec![Value::zeroes(width); size];

        Self {
            base_port: base,
            internal_state,
            allow_invalid_access: allow_invalid,
            width,
            addresser: MemD1::<false>,
        }
    }
}

impl CombMemD2 {
    pub fn new(
        base: GlobalPortIdx,
        width: u32,
        allow_invalid: bool,
        size: (usize, usize),
    ) -> Self {
        let internal_state = vec![Value::zeroes(width); size.0 * size.1];

        Self {
            base_port: base,
            internal_state,
            allow_invalid_access: allow_invalid,
            width,
            addresser: MemD2::<false> { d1_size: size.1 },
        }
    }
}

impl CombMemD3 {
    pub fn new(
        base: GlobalPortIdx,
        width: u32,
        allow_invalid: bool,
        size: (usize, usize, usize),
    ) -> Self {
        let internal_state =
            vec![Value::zeroes(width); size.0 * size.1 * size.2];

        Self {
            base_port: base,
            internal_state,
            allow_invalid_access: allow_invalid,
            width,
            addresser: MemD3::<false> {
                d1_size: size.1,
                d2_size: size.2,
            },
        }
    }
}

impl CombMemD4 {
    pub fn new(
        base: GlobalPortIdx,
        width: u32,
        allow_invalid: bool,
        size: (usize, usize, usize, usize),
    ) -> Self {
        let internal_state =
            vec![Value::zeroes(width); size.0 * size.1 * size.2 * size.3];

        Self {
            base_port: base,
            internal_state,
            allow_invalid_access: allow_invalid,
            width,
            addresser: MemD4::<false> {
                d1_size: size.1,
                d2_size: size.2,
                d3_size: size.3,
            },
        }
    }
}
