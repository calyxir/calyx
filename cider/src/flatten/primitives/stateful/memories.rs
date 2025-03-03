use cider_idx::{iter::SplitIndexRange, maps::IndexedMap, IndexRef};
use itertools::Itertools;

use crate::{
    errors::{RuntimeError, RuntimeResult},
    flatten::{
        flat_ir::{
            base::GlobalCellIdx,
            prelude::{AssignedValue, GlobalPortIdx, PortValue},
        },
        primitives::{
            declare_ports, declare_ports_no_signature, make_getters, ports,
            prim_trait::{RaceDetectionPrimitive, UpdateResult, UpdateStatus},
            utils::infer_thread_id,
            Primitive,
        },
        structures::{
            environment::{
                clock::{new_clock_pair, ClockMap, ReadSource, ValueWithClock},
                PortMap,
            },
            thread::{ThreadIdx, ThreadMap},
        },
    },
    serialization::{Entry, PrintCode, Serializable, Shape},
};

use baa::{BitVecOps, BitVecValue, WidthInt};

#[derive(Clone)]
pub struct StdReg {
    base_port: GlobalPortIdx,
    internal_state: ValueWithClock,
    global_idx: GlobalCellIdx,
    done_is_high: bool,
}

impl StdReg {
    declare_ports![IN: 0, WRITE_EN: 1, _CLK: 2, RESET: 3, | OUT: 4, DONE: 5];

    pub fn new(
        base_port: GlobalPortIdx,
        global_idx: GlobalCellIdx,
        width: u32,
        clocks: &mut Option<&mut ClockMap>,
    ) -> Self {
        let internal_state = ValueWithClock::zero(
            width,
            new_clock_pair(clocks, global_idx, None),
        );
        Self {
            base_port,
            global_idx,
            internal_state,
            done_is_high: false,
        }
    }
}

impl Primitive for StdReg {
    fn clone_boxed(&self) -> Box<dyn Primitive> {
        Box::new(self.clone())
    }

    fn exec_cycle(&mut self, port_map: &mut PortMap) -> RuntimeResult<()> {
        ports![&self.base_port;
            input: Self::IN,
            write_en: Self::WRITE_EN,
            reset: Self::RESET,
            out_idx: Self::OUT,
            done: Self::DONE
        ];

        if port_map[reset].as_bool().unwrap_or_default() {
            self.internal_state.value =
                BitVecValue::zero(self.internal_state.value.width());
            port_map.insert_val_general(
                done,
                AssignedValue::cell_value(BitVecValue::new_false()),
            )?
        } else if port_map[write_en].as_bool().unwrap_or_default() {
            self.internal_state.value = port_map[input]
                .as_option()
                .ok_or(RuntimeError::UndefinedWrite(self.global_idx))?
                .val()
                .clone();

            self.done_is_high = true;

            port_map.insert_val_general(
                done,
                AssignedValue::cell_value(BitVecValue::new_true()),
            )?
        } else {
            self.done_is_high = false;
            port_map.insert_val_general(
                done,
                AssignedValue::cell_value(BitVecValue::new_false()),
            )?
        };

        port_map.insert_val_general(
            out_idx,
            AssignedValue::cell_value(self.internal_state.value.clone())
                .with_clocks(self.internal_state.clocks),
        )?;
        Ok(())
    }

    fn exec_comb(&self, port_map: &mut PortMap) -> UpdateResult {
        ports![&self.base_port;
            done: Self::DONE,
            out_idx: Self::OUT];

        let out_signal = port_map.insert_val_general(
            out_idx,
            AssignedValue::cell_value(self.internal_state.value.clone())
                .with_clocks(self.internal_state.clocks),
        )?;
        let done_signal = port_map.insert_val_general(
            done,
            AssignedValue::cell_value(if self.done_is_high {
                BitVecValue::new_true()
            } else {
                BitVecValue::new_false()
            }),
        )?;

        Ok(out_signal | done_signal)
    }

    fn serialize(&self, code: Option<PrintCode>) -> Serializable {
        Serializable::Val(Entry::from_val_code(
            &self.internal_state.value,
            &code.unwrap_or_default(),
        ))
    }

    fn has_serializable_state(&self) -> bool {
        true
    }

    fn dump_memory_state(&self) -> Option<Vec<u8>> {
        Some(self.internal_state.value.clone().to_bytes_le())
    }

    fn get_ports(&self) -> SplitIndexRange<GlobalPortIdx> {
        self.get_signature()
    }
}

impl RaceDetectionPrimitive for StdReg {
    fn clone_boxed_rd(&self) -> Box<dyn RaceDetectionPrimitive> {
        Box::new(self.clone())
    }

    fn as_primitive(&self) -> &dyn Primitive {
        self
    }

    fn exec_cycle_checked(
        &mut self,
        port_map: &mut PortMap,
        clock_map: &mut ClockMap,
        thread_map: &ThreadMap,
    ) -> RuntimeResult<()> {
        ports![&self.base_port;
            input: Self::IN,
            write_en: Self::WRITE_EN,
            reset: Self::RESET
        ];

        // If we are writing to the reg, check that the write is not concurrent
        // with another write or a read. We can't easily check if the reg is
        // being read.
        if port_map[write_en].as_bool().unwrap_or_default() {
            let thread = infer_thread_id(
                [&port_map[input], &port_map[write_en], &port_map[reset]]
                    .into_iter(),
            )
            .expect("Could not infer thread id for reg");

            self.internal_state
                .clocks
                .check_write_with_ascription(
                    thread,
                    thread_map,
                    clock_map,
                    port_map[write_en].winner().unwrap(),
                )
                .map_err(|e| e.add_cell_info(self.global_idx, None))?;
        }

        self.exec_cycle(port_map)
    }
}

#[derive(Clone)]
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

    declare_ports_no_signature![
        SEQ_ADDR0: 2, COMB_ADDR0: 0,
        SEQ_ADDR1: 3, COMB_ADDR1: 1,
        SEQ_ADDR2: 4, COMB_ADDR2: 2,
        SEQ_ADDR3: 5, COMB_ADDR3: 3
    ];

    pub fn addr_as_vec(
        &self,
        port_map: &PortMap,
        base_port: GlobalPortIdx,
    ) -> Option<Vec<u64>> {
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

        Some(match self.shape {
            Shape::D1(..) => vec![port_map[addr0].as_u64().unwrap()],
            Shape::D2(..) => {
                let a0 = port_map[addr0].as_u64()? as usize;
                let a1 = port_map[addr1].as_u64()? as usize;

                vec![a0 as u64, a1 as u64]
            }
            Shape::D3(..) => {
                let a0 = port_map[addr0].as_u64()? as usize;
                let a1 = port_map[addr1].as_u64()? as usize;
                let a2 = port_map[addr2].as_u64()? as usize;

                vec![a0 as u64, a1 as u64, a2 as u64]
            }
            Shape::D4(..) => {
                let a0 = port_map[addr0].as_u64()? as usize;
                let a1 = port_map[addr1].as_u64()? as usize;
                let a2 = port_map[addr2].as_u64()? as usize;
                let a3 = port_map[addr3].as_u64()? as usize;

                vec![a0 as u64, a1 as u64, a2 as u64, a3 as u64]
            }
        })
    }

    pub fn calculate_addr(
        &self,
        port_map: &PortMap,
        base_port: GlobalPortIdx,
        cell_idx: GlobalCellIdx,
    ) -> RuntimeResult<Option<usize>> {
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

        let option: Option<usize> =
            self.compute_address(port_map, addr0, addr1, addr2, addr3);

        if let Some(v) = option {
            if v >= self.shape.size() {
                Err(RuntimeError::InvalidMemoryAccess {
                    access: self.addr_as_vec(port_map, base_port).unwrap(),
                    dims: self.shape.clone(),
                    idx: cell_idx,
                }
                .into())
            } else {
                Ok(Some(v))
            }
        } else {
            Ok(None)
        }
    }

    fn compute_address(
        &self,
        port_map: &IndexedMap<GlobalPortIdx, PortValue>,
        addr0: GlobalPortIdx,
        addr1: GlobalPortIdx,
        addr2: GlobalPortIdx,
        addr3: GlobalPortIdx,
    ) -> Option<usize> {
        match self.shape {
            Shape::D1(_d0_size) => port_map[addr0].as_u64().map(|v| v as usize),
            Shape::D2(_d0_size, d1_size) => {
                let a0 = port_map[addr0].as_u64()? as usize;
                let a1 = port_map[addr1].as_u64()? as usize;

                Some(a0 * d1_size + a1)
            }
            Shape::D3(_d0_size, d1_size, d2_size) => {
                let a0 = port_map[addr0].as_u64()? as usize;
                let a1 = port_map[addr1].as_u64()? as usize;
                let a2 = port_map[addr2].as_u64()? as usize;

                Some(a0 * (d1_size * d2_size) + a1 * d2_size + a2)
            }
            Shape::D4(_d0_size, d1_size, d2_size, d3_size) => {
                let a0 = port_map[addr0].as_u64()? as usize;
                let a1 = port_map[addr1].as_u64()? as usize;
                let a2 = port_map[addr2].as_u64()? as usize;
                let a3 = port_map[addr3].as_u64()? as usize;

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

    pub fn iter_addr_ports(
        &self,
        base_port: GlobalPortIdx,
    ) -> Box<dyn Iterator<Item = GlobalPortIdx>> {
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
            Shape::D1(_) => Box::new(std::iter::once(addr0)),
            Shape::D2(_, _) => Box::new([addr0, addr1].into_iter()),
            Shape::D3(_, _, _) => Box::new([addr0, addr1, addr2].into_iter()),
            Shape::D4(_, _, _, _) => {
                Box::new([addr0, addr1, addr2, addr3].into_iter())
            }
        }
    }
}

#[derive(Clone)]
pub struct CombMem {
    base_port: GlobalPortIdx,
    internal_state: Vec<ValueWithClock>,
    // TODO griffin: This bool is unused in the actual struct and should either
    // be removed or
    _allow_invalid_access: bool,
    _width: u32,
    addresser: MemDx<false>,
    done_is_high: bool,
    global_idx: GlobalCellIdx,
}
impl CombMem {
    declare_ports_no_signature![
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
        global_idx: GlobalCellIdx,
        width: u32,
        allow_invalid: bool,
        size: T,
        clocks: &mut Option<&mut ClockMap>,
    ) -> Self
    where
        T: Into<Shape>,
    {
        let shape = size.into();
        let mut internal_state = Vec::with_capacity(shape.size());
        for i in 0_usize..shape.size() {
            internal_state.push(ValueWithClock::zero(
                width,
                new_clock_pair(clocks, global_idx, Some(i.try_into().unwrap())),
            ));
        }

        Self {
            base_port: base,
            internal_state,
            _allow_invalid_access: allow_invalid,
            _width: width,
            addresser: MemDx::new(shape),
            done_is_high: false,
            global_idx,
        }
    }

    pub fn new_with_init<T>(
        base_port: GlobalPortIdx,
        global_idx: GlobalCellIdx,
        width: WidthInt,
        allow_invalid: bool,
        size: T,
        data: &[u8],
        clocks: &mut Option<&mut ClockMap>,
    ) -> Self
    where
        T: Into<Shape>,
    {
        let byte_count = width.div_ceil(8);
        let size = size.into();

        let internal_state = data
            .chunks_exact(byte_count as usize)
            .map(|x| BitVecValue::from_bytes_le(x, width))
            .enumerate()
            .map(|(i, x)| {
                ValueWithClock::new(
                    x,
                    new_clock_pair(
                        clocks,
                        global_idx,
                        Some(i.try_into().unwrap()),
                    ),
                )
            })
            .collect_vec();

        assert_eq!(internal_state.len(), size.size());
        assert!(data
            .chunks_exact(byte_count as usize)
            .remainder()
            .is_empty());

        Self {
            base_port,
            internal_state,
            _allow_invalid_access: allow_invalid,
            _width: width,
            addresser: MemDx::new(size),
            done_is_high: false,
            global_idx,
        }
    }

    pub fn dump_data(&self) -> Vec<u8> {
        self.internal_state
            .iter()
            .flat_map(|x| x.value.to_bytes_le())
            .collect()
    }

    fn infer_thread(&self, port_map: &mut PortMap) -> Option<ThreadIdx> {
        let ports = self
            .addresser
            .iter_addr_ports(self.base_port)
            .chain([self.write_en(), self.write_data()])
            .map(|x| &port_map[x]);
        infer_thread_id(ports)
    }
}

impl Primitive for CombMem {
    fn clone_boxed(&self) -> Box<dyn Primitive> {
        Box::new(self.clone())
    }

    fn exec_comb(&self, port_map: &mut PortMap) -> UpdateResult {
        let addr: Option<usize> = self
            .addresser
            .calculate_addr(port_map, self.base_port, self.global_idx)
            .unwrap_or_default();

        let read_data = self.read_data();

        let read =
            if addr.is_some() && addr.unwrap() < self.internal_state.len() {
                let addr = addr.unwrap();

                port_map.insert_val_general(
                    read_data,
                    AssignedValue::cell_value(
                        self.internal_state[addr].value.clone(),
                    )
                    .with_clocks(self.internal_state[addr].clocks),
                )?
            }
            // either the address is undefined or it is outside the range of valid addresses
            else {
                // throw error on cycle boundary rather than here
                port_map.write_undef(read_data)?;
                UpdateStatus::Unchanged
            };

        let done_signal = port_map.insert_val_general(
            self.done(),
            AssignedValue::cell_value(if self.done_is_high {
                BitVecValue::new_true()
            } else {
                BitVecValue::new_false()
            }),
        )?;
        Ok(done_signal | read)
    }

    fn exec_cycle(&mut self, port_map: &mut PortMap) -> RuntimeResult<()> {
        // These two behave like false when undefined
        let reset = port_map[self.reset_port()].as_bool().unwrap_or_default();
        let write_en = port_map[self.write_en()].as_bool().unwrap_or_default();

        let addr = self.addresser.calculate_addr(
            port_map,
            self.base_port,
            self.global_idx,
        )?;
        let (read_data, done) = (self.read_data(), self.done());

        if write_en && !reset {
            let addr =
                addr.ok_or(RuntimeError::UndefinedWriteAddr(self.global_idx))?;

            let write_data = port_map[self.write_data()]
                .as_option()
                .ok_or(RuntimeError::UndefinedWrite(self.global_idx))?;
            self.internal_state[addr].value = write_data.val().clone();
            self.done_is_high = true;
            port_map.insert_val_general(done, AssignedValue::cell_b_high())?
        } else {
            self.done_is_high = false;
            port_map.insert_val_general(done, AssignedValue::cell_b_low())?
        };

        if let Some(addr) = addr {
            port_map.insert_val_general(
                read_data,
                AssignedValue::cell_value(
                    self.internal_state[addr].value.clone(),
                )
                .with_clocks(self.internal_state[addr].clocks),
            )?;
        } else {
            port_map.write_undef(read_data)?;
        }
        Ok(())
    }

    fn serialize(&self, code: Option<PrintCode>) -> Serializable {
        let code = code.unwrap_or_default();

        Serializable::Array(
            self.internal_state
                .iter()
                .map(|x| Entry::from_val_code(&x.value, &code))
                .collect(),
            self.addresser.get_dimensions(),
        )
    }

    fn has_serializable_state(&self) -> bool {
        true
    }

    fn dump_memory_state(&self) -> Option<Vec<u8>> {
        Some(self.dump_data())
    }

    fn get_ports(&self) -> SplitIndexRange<GlobalPortIdx> {
        SplitIndexRange::new(
            self.base_port,
            self.read_data(),
            (self.done().index() + 1).into(),
        )
    }
}

impl RaceDetectionPrimitive for CombMem {
    fn clone_boxed_rd(&self) -> Box<dyn RaceDetectionPrimitive> {
        Box::new(self.clone())
    }

    fn as_primitive(&self) -> &dyn Primitive {
        self
    }

    // fn exec_comb_checked(
    //     &self,
    //     port_map: &mut PortMap,
    //     clock_map: &mut ClockMap,
    //     thread_map: &ThreadMap,
    // ) -> UpdateResult {
    //     let thread = self.infer_thread(port_map);

    //     if let Some(addr) =
    //         self.addresser.calculate_addr(port_map, self.base_port)
    //     {
    //         if addr < self.internal_state.len() {
    //             let thread =
    //                 thread.expect("Could not infer thread id for comb mem");
    //             let reading_clock = thread_map.unwrap_clock_id(thread);

    //             let val = &self.internal_state[addr];
    //             val.check_read(reading_clock, clock_map)?;
    //         }
    //     }

    //     self.exec_comb(port_map)
    // }

    fn exec_cycle_checked(
        &mut self,
        port_map: &mut PortMap,
        clock_map: &mut ClockMap,
        thread_map: &ThreadMap,
    ) -> RuntimeResult<()> {
        let thread = self.infer_thread(port_map);
        if let Some(addr) = self.addresser.calculate_addr(
            port_map,
            self.base_port,
            self.global_idx,
        )? {
            if addr < self.internal_state.len() {
                if let Some(thread) = thread {
                    let val = &self.internal_state[addr];

                    if port_map[self.write_en()].as_bool().unwrap_or_default() {
                        val.clocks
                            .check_write_with_ascription(
                                thread,
                                thread_map,
                                clock_map,
                                port_map[self.write_en()].winner().unwrap(),
                            )
                            .map_err(|e| {
                                e.add_cell_info(
                                    self.global_idx,
                                    Some(addr.try_into().unwrap()),
                                )
                            })?;
                    }
                } else if addr != 0
                    || port_map[self.write_en()].as_bool().unwrap_or_default()
                {
                    // HACK: if the addr is 0, we're reading, and the thread
                    // can't be determined then we assume the read is not real
                    panic!("unable to determine thread for comb mem");
                }
            }
        }

        self.exec_cycle(port_map)
    }
}

#[derive(Copy, Clone, Debug)]
enum MemOut {
    /// Points to a valid address in the memory
    Valid(usize),
    /// Output is zero, but not a memory address
    Zero,
    /// Output is undefined
    Undef,
}

impl MemOut {
    fn is_def(&self) -> bool {
        match self {
            MemOut::Valid(_) | MemOut::Zero => true,
            MemOut::Undef => false,
        }
    }

    fn get_value(&self, data: &[ValueWithClock]) -> PortValue {
        match self {
            MemOut::Valid(addr) => {
                PortValue::new_cell(data[*addr].value.clone())
            }
            MemOut::Zero => {
                PortValue::new_cell(BitVecValue::zero(data[0].value.width()))
            }
            MemOut::Undef => PortValue::new_undef(),
        }
    }
}

#[derive(Clone)]
pub struct SeqMem {
    base_port: GlobalPortIdx,
    internal_state: Vec<ValueWithClock>,
    global_idx: GlobalCellIdx,
    // TODO griffin: This bool is unused in the actual struct and should either
    // be removed or
    _allow_invalid_access: bool,
    addresser: MemDx<true>,
    done_is_high: bool,
    // memory index which is currently latched
    read_out: MemOut,
}

impl SeqMem {
    pub fn new<T: Into<Shape>>(
        base: GlobalPortIdx,
        global_idx: GlobalCellIdx,
        width: u32,
        allow_invalid: bool,
        size: T,
        clocks: &mut Option<&mut ClockMap>,
    ) -> Self {
        let shape = size.into();
        let mut internal_state = Vec::with_capacity(shape.size());
        for i in 0_usize..shape.size() {
            internal_state.push(ValueWithClock::zero(
                width,
                new_clock_pair(clocks, global_idx, Some(i.try_into().unwrap())),
            ));
        }

        Self {
            base_port: base,
            internal_state,
            _allow_invalid_access: allow_invalid,
            addresser: MemDx::new(shape),
            done_is_high: false,
            read_out: MemOut::Undef,
            global_idx,
        }
    }

    pub fn new_with_init<T>(
        base_port: GlobalPortIdx,
        global_idx: GlobalCellIdx,
        width: WidthInt,
        allow_invalid: bool,
        size: T,
        data: &[u8],
        clocks: &mut Option<&mut ClockMap>,
    ) -> Self
    where
        T: Into<Shape>,
    {
        let byte_count = width.div_ceil(8);
        let size = size.into();

        let internal_state = data
            .chunks_exact(byte_count as usize)
            .map(|x| BitVecValue::from_bytes_le(x, width))
            .enumerate()
            .map(|(i, x)| {
                ValueWithClock::new(
                    x,
                    new_clock_pair(
                        clocks,
                        global_idx,
                        Some(i.try_into().unwrap()),
                    ),
                )
            })
            .collect_vec();

        assert_eq!(internal_state.len(), size.size());
        assert!(data
            .chunks_exact(byte_count as usize)
            .remainder()
            .is_empty());

        Self {
            base_port,
            internal_state,
            _allow_invalid_access: allow_invalid,
            addresser: MemDx::new(size),
            done_is_high: false,
            read_out: MemOut::Undef,
            global_idx,
        }
    }

    declare_ports_no_signature![
        _CLK: 0,
        RESET: 1,
    ];

    // these port offsets are placed after the address ports and so need the end
    // of the address base to work correctly.
    declare_ports_no_signature![
        CONTENT_ENABLE: 0,
        WRITE_ENABLE: 1,
        WRITE_DATA: 2,
        READ_DATA: 3,
        DONE: 4
    ];

    make_getters![base_port;
          content_enable: Self::CONTENT_ENABLE,
          write_enable: Self::WRITE_ENABLE,
          write_data: Self::WRITE_DATA,
          read_data: Self::READ_DATA,
          done: Self::DONE
    ];

    pub fn _clk(&self) -> GlobalPortIdx {
        (self.base_port.index() + Self::_CLK).into()
    }

    pub fn reset(&self) -> GlobalPortIdx {
        (self.base_port.index() + Self::RESET).into()
    }

    pub fn dump_data(&self) -> Vec<u8> {
        self.internal_state
            .iter()
            .flat_map(|x| x.value.to_bytes_le())
            .collect()
    }

    fn infer_thread(&self, port_map: &mut PortMap) -> Option<ThreadIdx> {
        let ports = self
            .addresser
            .iter_addr_ports(self.base_port)
            .chain([
                self.content_enable(),
                self.write_data(),
                self.write_enable(),
            ])
            .map(|x| &port_map[x]);
        infer_thread_id(ports)
    }
}

impl Primitive for SeqMem {
    fn clone_boxed(&self) -> Box<dyn Primitive> {
        Box::new(self.clone())
    }

    fn exec_comb(&self, port_map: &mut PortMap) -> UpdateResult {
        let done_signal = port_map.insert_val_general(
            self.done(),
            AssignedValue::cell_value(if self.done_is_high {
                BitVecValue::new_true()
            } else {
                BitVecValue::new_false()
            }),
        )?;

        let out_signal = if port_map[self.read_data()].is_undef()
            && self.read_out.is_def()
        {
            port_map.insert_val_general(
                self.read_data(),
                self.read_out
                    .get_value(&self.internal_state)
                    .into_option()
                    .unwrap(),
            )?
        } else {
            UpdateStatus::Unchanged
        };

        Ok(done_signal | out_signal)
    }

    fn exec_cycle(&mut self, port_map: &mut PortMap) -> RuntimeResult<()> {
        let reset = port_map[self.reset()].as_bool().unwrap_or_default();
        let write_en =
            port_map[self.write_enable()].as_bool().unwrap_or_default();
        let content_en = port_map[self.content_enable()]
            .as_bool()
            .unwrap_or_default();
        let addr = self.addresser.calculate_addr(
            port_map,
            self.base_port,
            self.global_idx,
        )?;

        if reset {
            self.done_is_high = false;
            self.read_out = MemOut::Zero;
        } else if content_en && write_en {
            self.done_is_high = true;
            self.read_out = MemOut::Undef;
            let addr_actual =
                addr.ok_or(RuntimeError::UndefinedWriteAddr(self.global_idx))?;
            let write_data = port_map[self.write_data()]
                .as_option()
                .ok_or(RuntimeError::UndefinedWrite(self.global_idx))?;
            self.internal_state[addr_actual].value = write_data.val().clone();
        } else if content_en {
            self.done_is_high = true;
            let addr_actual =
                addr.ok_or(RuntimeError::UndefinedReadAddr(self.global_idx))?;
            self.read_out = MemOut::Valid(addr_actual);
        } else {
            self.done_is_high = false;
        }

        port_map.insert_val_general(
            self.done(),
            AssignedValue::cell_value(if self.done_is_high {
                BitVecValue::new_true()
            } else {
                BitVecValue::new_false()
            }),
        )?;
        port_map.write_exact_unchecked(
            self.read_data(),
            self.read_out.get_value(&self.internal_state),
        );
        Ok(())
    }

    fn has_comb_path(&self) -> bool {
        false
    }

    fn has_stateful_path(&self) -> bool {
        true
    }

    fn serialize(&self, code: Option<PrintCode>) -> Serializable {
        let code = code.unwrap_or_default();

        Serializable::Array(
            self.internal_state
                .iter()
                .map(|x| Entry::from_val_code(&x.value, &code))
                .collect(),
            self.addresser.get_dimensions(),
        )
    }

    fn has_serializable_state(&self) -> bool {
        true
    }

    fn dump_memory_state(&self) -> Option<Vec<u8>> {
        Some(self.dump_data())
    }

    fn get_ports(&self) -> SplitIndexRange<GlobalPortIdx> {
        SplitIndexRange::new(
            self.base_port,
            self.read_data(),
            (self.done().index() + 1).into(),
        )
    }
}

impl RaceDetectionPrimitive for SeqMem {
    fn clone_boxed_rd(&self) -> Box<dyn RaceDetectionPrimitive> {
        Box::new(self.clone())
    }

    fn as_primitive(&self) -> &dyn Primitive {
        self
    }

    fn exec_comb_checked(
        &self,
        port_map: &mut PortMap,
        _clock_map: &mut ClockMap,
        _thread_map: &ThreadMap,
    ) -> UpdateResult {
        self.exec_comb(port_map)
    }

    fn exec_cycle_checked(
        &mut self,
        port_map: &mut PortMap,
        clock_map: &mut ClockMap,
        thread_map: &ThreadMap,
    ) -> RuntimeResult<()> {
        let thread = self.infer_thread(port_map);
        if let Some(addr) = self.addresser.calculate_addr(
            port_map,
            self.base_port,
            self.global_idx,
        )? {
            if addr < self.internal_state.len() {
                let thread_clock =
                    thread.map(|thread| thread_map.unwrap_clock_id(thread));

                let val = &self.internal_state[addr];

                if port_map[self.write_enable()].as_bool().unwrap_or_default()
                    && port_map[self.content_enable()]
                        .as_bool()
                        .unwrap_or_default()
                {
                    val.clocks
                        .check_write_with_ascription(
                            thread.expect(
                                "unable to determine thread for seq mem",
                            ),
                            thread_map,
                            clock_map,
                            port_map[self.write_enable()].winner().unwrap(),
                        )
                        .map_err(|e| {
                            e.add_cell_info(
                                self.global_idx,
                                Some(addr.try_into().unwrap()),
                            )
                        })?;
                } else if port_map[self.content_enable()]
                    .as_bool()
                    .unwrap_or_default()
                {
                    let (assignment_idx, cell) = port_map
                        [self.content_enable()]
                    .winner()
                    .unwrap()
                    .as_assign()
                    .unwrap();
                    val.clocks
                        .check_read_with_ascription(
                            (
                                thread.expect(
                                    "unable to determine thread for seq mem",
                                ),
                                thread_clock.unwrap(),
                            ),
                            ReadSource::Assignment(assignment_idx),
                            cell,
                            clock_map,
                        )
                        .map_err(|e| {
                            e.add_cell_info(
                                self.global_idx,
                                Some(addr.try_into().unwrap()),
                            )
                        })?;
                }
            }
        }
        self.exec_cycle(port_map)
    }
}

// type aliases, this is kinda stupid and should probably be changed. or maybe
// it's fine, I really don't know.
pub type CombMemD1 = CombMem;
pub type CombMemD2 = CombMem;
pub type CombMemD3 = CombMem;
pub type CombMemD4 = CombMem;

pub type SeqMemD1 = SeqMem;
pub type SeqMemD2 = SeqMem;
pub type SeqMemD3 = SeqMem;
pub type SeqMemD4 = SeqMem;
