use crate::{
    errors::InterpreterError,
    flatten::{
        flat_ir::prelude::{AssignedValue, GlobalPortIdx},
        primitives::{
            declare_ports, make_getters, ports,
            prim_trait::{UpdateResult, UpdateStatus},
            Primitive,
        },
        structures::environment::PortMap,
    },
    serialization::{Entry, Serializable, Shape},
    values::Value,
};

pub struct StdReg {
    base_port: GlobalPortIdx,
    internal_state: Value,
    done_is_high: bool,
}

impl StdReg {
    declare_ports![IN: 0, WRITE_EN: 1, _CLK: 2, RESET: 3, OUT: 4, DONE: 5];

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

pub struct MemDx<const SEQ: bool> {
    shape: Shape,
}

impl<const SEQ: bool> MemDx<SEQ> {
    pub fn new<T>(shape: T) -> Self
    where
        T: Into<Shape>,
    {
        Self {
            shape: shape.into(),
        }
    }

    declare_ports![
        SEQ_ADDR0: 2, COMB_ADDR0: 0,
        SEQ_ADDR1: 3, COMB_ADDR1: 1,
        SEQ_ADDR2: 4, COMB_ADDR2: 2,
        SEQ_ADDR3: 5, COMB_ADDR3: 3
    ];

    pub fn calculate_addr(
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

        match self.shape {
            Shape::D1(_d0_size) => port_map[addr0].as_usize(),
            Shape::D2(_d0_size, d1_size) => {
                let a0 = port_map[addr0].as_usize()?;
                let a1 = port_map[addr1].as_usize()?;

                Some(a0 * d1_size + a1)
            }
            Shape::D3(_d0_size, d1_size, d2_size) => {
                let a0 = port_map[addr0].as_usize()?;
                let a1 = port_map[addr1].as_usize()?;
                let a2 = port_map[addr2].as_usize()?;

                Some(a0 * (d1_size * d2_size) + a1 * d2_size + a2)
            }
            Shape::D4(_d0_size, d1_size, d2_size, d3_size) => {
                let a0 = port_map[addr0].as_usize()?;
                let a1 = port_map[addr1].as_usize()?;
                let a2 = port_map[addr2].as_usize()?;
                let a3 = port_map[addr3].as_usize()?;

                Some(
                    a0 * (d1_size * d2_size * d3_size)
                        + a1 * (d2_size * d3_size)
                        + a2 * d3_size
                        + a3,
                )
            }
        }
    }

    pub fn non_address_base(&self) -> usize {
        if SEQ {
            match self.shape {
                Shape::D1(_) => Self::SEQ_ADDR0 + 1,
                Shape::D2(_, _) => Self::SEQ_ADDR1 + 1,
                Shape::D3(_, _, _) => Self::SEQ_ADDR2 + 1,
                Shape::D4(_, _, _, _) => Self::SEQ_ADDR3 + 1,
            }
        } else {
            match self.shape {
                Shape::D1(_) => Self::COMB_ADDR0 + 1,
                Shape::D2(_, _) => Self::COMB_ADDR1 + 1,
                Shape::D3(_, _, _) => Self::COMB_ADDR2 + 1,
                Shape::D4(_, _, _, _) => Self::COMB_ADDR3 + 1,
            }
        }
    }

    pub fn get_dimensions(&self) -> Shape {
        self.shape.clone()
    }
}

pub struct CombMem {
    base_port: GlobalPortIdx,
    internal_state: Vec<Value>,
    // TODO griffin: This bool is unused in the actual struct and should either
    // be removed or
    _allow_invalid_access: bool,
    _width: u32,
    addresser: MemDx<false>,
    done_is_high: bool,
}
impl CombMem {
    declare_ports![
        WRITE_DATA:0,
        WRITE_EN: 1,
        _CLK: 2,
        RESET: 3,
        READ_DATA: 4,
        DONE: 5
    ];

    make_getters![base_port;
        write_data: Self::WRITE_DATA,
        write_en: Self::WRITE_EN,
        reset_port: Self::RESET,
        read_data: Self::READ_DATA,
        done: Self::DONE
    ];

    pub fn new<T>(
        base: GlobalPortIdx,
        width: u32,
        allow_invalid: bool,
        size: T,
    ) -> Self
    where
        T: Into<Shape>,
    {
        let shape = size.into();
        let internal_state = vec![Value::zeroes(width); shape.len()];

        Self {
            base_port: base,
            internal_state,
            _allow_invalid_access: allow_invalid,
            _width: width,
            addresser: MemDx::new(shape),
            done_is_high: false,
        }
    }
}

impl Primitive for CombMem {
    fn exec_comb(&self, port_map: &mut PortMap) -> UpdateResult {
        let addr = self.addresser.calculate_addr(port_map, self.base_port);
        let read_data = self.read_data();

        let read =
            if addr.is_some() && addr.unwrap() < self.internal_state.len() {
                port_map.insert_val(
                    read_data,
                    AssignedValue::cell_value(
                        self.internal_state[addr.unwrap()].clone(),
                    ),
                )?
            }
            // either the address is undefined or it is outside the range of valid addresses
            else {
                // throw error on cycle boundary rather than here
                port_map.write_undef(read_data)?;
                UpdateStatus::Unchanged
            };

        let done_signal = port_map.insert_val(
            self.done(),
            AssignedValue::cell_value(if self.done_is_high {
                Value::bit_high()
            } else {
                Value::bit_low()
            }),
        )?;
        Ok(done_signal | read)
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
            self.done_is_high = true;
            port_map.insert_val(done, AssignedValue::cell_b_high())?
        } else {
            self.done_is_high = false;
            port_map.insert_val(done, AssignedValue::cell_b_low())?
        };

        if let Some(addr) = addr {
            Ok(port_map.insert_val(
                read_data,
                AssignedValue::cell_value(self.internal_state[addr].clone()),
            )? | done)
        } else {
            port_map.write_undef(read_data)?;
            Ok(done)
        }
    }

    fn serialize(
        &self,
        code: Option<crate::debugger::PrintCode>,
    ) -> Serializable {
        let code = code.unwrap_or_default();

        Serializable::Array(
            self.internal_state
                .iter()
                .map(|x| Entry::from_val_code(x, &code))
                .collect(),
            self.addresser.get_dimensions(),
        )
    }

    fn has_serializable_state(&self) -> bool {
        true
    }
}

pub struct SeqMem {
    base_port: GlobalPortIdx,
    internal_state: Vec<Value>,
    // TODO griffin: This bool is unused in the actual struct and should either
    // be removed or
    _allow_invalid_access: bool,
    _width: u32,
    addresser: MemDx<true>,
    done_is_high: bool,
}

// type aliases
pub type CombMemD1 = CombMem;
pub type CombMemD2 = CombMem;
pub type CombMemD3 = CombMem;
pub type CombMemD4 = CombMem;
