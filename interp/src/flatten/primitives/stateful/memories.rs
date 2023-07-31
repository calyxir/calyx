use crate::{
    flatten::{
        flat_ir::prelude::GlobalPortId,
        primitives::{
            declare_ports, output, ports, prim_trait::Results, Primitive,
        },
        structures::environment::PortMap,
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

    pub fn new(base_port: GlobalPortId, width: u32) -> Self {
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

    fn reset(&mut self, _: &PortMap) -> Results {
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

pub trait MemAddresser {
    const NON_ADDRESS_BASE: usize;

    fn calculate_addr(
        &self,
        port_map: &PortMap,
        base_port: GlobalPortId,
    ) -> usize;
}

struct MemD1<const SEQ: bool>;

impl<const SEQ: bool> MemAddresser for MemD1<SEQ> {
    fn calculate_addr(
        &self,
        port_map: &PortMap,
        base_port: GlobalPortId,
    ) -> usize {
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

struct MemD2<const SEQ: bool> {
    d1_size: usize,
}

impl<const SEQ: bool> MemD2<SEQ> {
    declare_ports![SEQ_ADDR0: 2, COMB_ADDR0: 0, SEQ_ADDR1: 3, COMB_ADDR1: 1];
}

impl<const SEQ: bool> MemAddresser for MemD2<SEQ> {
    fn calculate_addr(
        &self,
        port_map: &PortMap,
        base_port: GlobalPortId,
    ) -> usize {
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

        let a0 = port_map[addr0].as_usize();
        let a1 = port_map[addr1].as_usize();

        a0 * self.d1_size + a1
    }

    const NON_ADDRESS_BASE: usize = if SEQ {
        Self::SEQ_ADDR1 + 1
    } else {
        Self::COMB_ADDR1 + 1
    };
}

struct MemD3<const SEQ: bool> {
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
        base_port: GlobalPortId,
    ) -> usize {
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

        let a0 = port_map[addr0].as_usize();
        let a1 = port_map[addr1].as_usize();
        let a2 = port_map[addr2].as_usize();

        a0 * (self.d1_size * self.d2_size) + a1 * self.d2_size + a2
    }

    const NON_ADDRESS_BASE: usize = if SEQ {
        Self::SEQ_ADDR2 + 1
    } else {
        Self::COMB_ADDR2 + 1
    };
}

struct MemD4<const SEQ: bool> {
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
        base_port: GlobalPortId,
    ) -> usize {
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

        let a0 = port_map[addr0].as_usize();
        let a1 = port_map[addr1].as_usize();
        let a2 = port_map[addr2].as_usize();
        let a3 = port_map[addr3].as_usize();

        a0 * (self.d1_size * self.d2_size * self.d3_size)
            + a1 * (self.d2_size * self.d3_size)
            + a2 * self.d3_size
            + a3
    }

    const NON_ADDRESS_BASE: usize = if SEQ {
        Self::SEQ_ADDR3 + 1
    } else {
        Self::COMB_ADDR3 + 1
    };
}

pub struct StdMem<M: MemAddresser> {
    base_port: GlobalPortId,
    internal_state: Vec<Value>,
    allow_invalid_access: bool,
    width: u32,
    addresser: M,
}

impl<M: MemAddresser> StdMem<M> {
    declare_ports![
        WRITE_DATA: M::NON_ADDRESS_BASE + 1,
        WRITE_EN: M::NON_ADDRESS_BASE + 2,
        CLK: M::NON_ADDRESS_BASE + 3,
        RESET: M::NON_ADDRESS_BASE + 4,
        READ_DATA: M::NON_ADDRESS_BASE + 5,
        DONE: M::NON_ADDRESS_BASE + 6
    ];
}

impl<M: MemAddresser> Primitive for StdMem<M> {
    fn exec_comb(&self, port_map: &PortMap) -> Results {
        ports![&self.base_port; read_data: Self::READ_DATA ];
        let addr = self.addresser.calculate_addr(port_map, self.base_port);

        if addr < self.internal_state.len() {
            Ok(output![read_data: self.internal_state[addr].clone()])
        } else {
            // throw error on cycle boundary rather than here
            Ok(output![read_data: Value::zeroes(self.width)])
        }
    }

    fn exec_cycle(&mut self, port_map: &PortMap) -> Results {
        ports![&self.base_port;
            write_data: Self::WRITE_DATA,
            write_en: Self::WRITE_EN,
            reset: Self::RESET,
            read_data: Self::READ_DATA,
            done: Self::DONE
        ];
        let reset = port_map[reset].as_bool();
        let write_en = port_map[write_en].as_bool();
        let addr = self.addresser.calculate_addr(port_map, self.base_port);

        if write_en && !reset {
            let write_data = port_map[write_data].clone();
            self.internal_state[addr] = write_data;
            Ok(
                output![read_data: self.internal_state[addr].clone(), done: Value::bit_high()],
            )
        } else {
            Ok(
                output![read_data: self.internal_state[addr].clone(), done: Value::bit_low()],
            )
        }
    }

    fn reset(&mut self, _port_map: &PortMap) -> Results {
        ports![&self.base_port; read_data: Self::READ_DATA, done: Self::DONE ];
        Ok(
            output![read_data: Value::zeroes(self.width), done: Value::bit_low()],
        )
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
