use std::iter::repeat_n;

use cider_idx::{IndexRef, iter::SplitIndexRange, maps::IndexedMap};

use crate::{
    errors::{RuntimeError, RuntimeResult},
    flatten::{
        flat_ir::{
            indexes::{GlobalCellIdx, MemoryLocation, MemoryRegion},
            prelude::{AssignedValue, GlobalPortIdx, PortValue},
        },
        primitives::{
            Primitive, declare_ports, declare_ports_no_signature, make_getters,
            ports,
            prim_trait::{
                RaceDetectionPrimitive, SerializeState, UpdateResult,
                UpdateStatus,
            },
            utils::infer_thread_id,
        },
        structures::{
            environment::{MemoryMap, PortMap, clock::ClockMap},
            thread::{ThreadIdx, ThreadMap},
        },
    },
    serialization::{Dimensions, LazySerializable, PrintCode},
};

use baa::{BitVecOps, BitVecValue};

#[derive(Clone)]
pub struct StdReg {
    base_port: GlobalPortIdx,
    internal_state: MemoryLocation,
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
        memory_map: &mut MemoryMap,
    ) -> Self {
        let val = BitVecValue::zero(width);
        let internal_state =
            memory_map.allocate_memory_location(val, global_idx, None, clocks);
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

    fn exec_cycle(
        &mut self,
        port_map: &mut PortMap,
        state_map: &mut MemoryMap,
    ) -> UpdateResult {
        ports![&self.base_port;
            input: Self::IN,
            write_en: Self::WRITE_EN,
            reset: Self::RESET,
            out_idx: Self::OUT,
            done: Self::DONE
        ];

        let mut changed = UpdateStatus::Unchanged;

        if port_map[reset].as_bool().unwrap_or_default() {
            changed |= state_map.set_location(
                self.internal_state,
                BitVecValue::zero(state_map[self.internal_state].width()),
            );
            changed |= port_map.insert_val_general(
                done,
                AssignedValue::cell_value(BitVecValue::new_false()),
            )?;
        } else if port_map[write_en].as_bool().unwrap_or_default() {
            let Some(port_value) = port_map[input].as_option() else {
                return Err(
                    RuntimeError::UndefinedWrite(self.global_idx).into()
                );
            };
            changed |= state_map
                .set_location(self.internal_state, port_value.val().clone());

            self.done_is_high = true;

            changed |= port_map.insert_val_general(
                done,
                AssignedValue::cell_value(BitVecValue::new_true()),
            )?;
        } else {
            self.done_is_high = false;
            changed |= port_map.insert_val_general(
                done,
                AssignedValue::cell_value(BitVecValue::new_false()),
            )?;
        };

        changed |= port_map.insert_val_general(
            out_idx,
            AssignedValue::cell_value(state_map[self.internal_state].clone())
                .with_clocks(
                    state_map.get_clock_or_default(self.internal_state),
                ),
        )?;
        Ok(changed)
    }

    fn exec_comb(
        &self,
        port_map: &mut PortMap,
        mem_map: &MemoryMap,
    ) -> UpdateResult {
        ports![&self.base_port;
            done: Self::DONE,
            out_idx: Self::OUT];

        let out_signal =
            port_map[done].is_undef() || port_map[out_idx].is_undef();

        if out_signal {
            port_map.insert_val_unchecked(
                out_idx,
                AssignedValue::cell_value(mem_map[self.internal_state].clone())
                    .with_clocks(
                        mem_map.get_clock_or_default(self.internal_state),
                    ),
            );
            port_map.insert_val_unchecked(
                done,
                AssignedValue::cell_value(if self.done_is_high {
                    BitVecValue::new_true()
                } else {
                    BitVecValue::new_false()
                }),
            );
        }

        Ok(out_signal.into())
    }

    fn get_ports(&self) -> SplitIndexRange<GlobalPortIdx> {
        self.get_signature()
    }

    fn serializer(&self) -> Option<&dyn SerializeState> {
        Some(self as &dyn SerializeState)
    }
}

impl SerializeState for StdReg {
    fn serialize<'a>(
        &self,
        code: PrintCode,
        map: &'a MemoryMap,
    ) -> LazySerializable<'a> {
        LazySerializable::new_val(code, &map[self.internal_state])
    }
    fn dump_data(&self, memory_map: &MemoryMap) -> Vec<u8> {
        memory_map[self.internal_state].to_bytes_le()
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
        state_map: &mut MemoryMap,
    ) -> UpdateResult {
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

            state_map
                .get_clock(self.internal_state)
                .unwrap()
                .check_write_with_ascription(
                    thread,
                    thread_map,
                    clock_map,
                    port_map[write_en].winner().unwrap(),
                )
                .map_err(|e| e.add_cell_info(self.global_idx, None))?;
        }

        self.exec_cycle(port_map, state_map)
    }
}

#[derive(Clone)]
pub struct MemDx<const SEQ: bool> {
    shape: Dimensions,
}

impl<const SEQ: bool> MemDx<SEQ> {
    pub fn new<T>(shape: T) -> Self
    where
        T: Into<Dimensions>,
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
            Dimensions::D1(..) => vec![port_map[addr0].as_u64().unwrap()],
            Dimensions::D2(..) => {
                let a0 = port_map[addr0].as_u64()? as usize;
                let a1 = port_map[addr1].as_u64()? as usize;

                vec![a0 as u64, a1 as u64]
            }
            Dimensions::D3(..) => {
                let a0 = port_map[addr0].as_u64()? as usize;
                let a1 = port_map[addr1].as_u64()? as usize;
                let a2 = port_map[addr2].as_u64()? as usize;

                vec![a0 as u64, a1 as u64, a2 as u64]
            }
            Dimensions::D4(..) => {
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
            Dimensions::D1(..) => {
                let a0 = port_map[addr0].as_u64()? as usize;
                Some(self.shape.compute_address_nocheck(&[a0]))
            }
            Dimensions::D2(..) => {
                let a0 = port_map[addr0].as_u64()? as usize;
                let a1 = port_map[addr1].as_u64()? as usize;

                Some(self.shape.compute_address_nocheck(&[a0, a1]))
            }
            Dimensions::D3(..) => {
                let a0 = port_map[addr0].as_u64()? as usize;
                let a1 = port_map[addr1].as_u64()? as usize;
                let a2 = port_map[addr2].as_u64()? as usize;

                Some(self.shape.compute_address_nocheck(&[a0, a1, a2]))
            }
            Dimensions::D4(..) => {
                let a0 = port_map[addr0].as_u64()? as usize;
                let a1 = port_map[addr1].as_u64()? as usize;
                let a2 = port_map[addr2].as_u64()? as usize;
                let a3 = port_map[addr3].as_u64()? as usize;

                Some(self.shape.compute_address_nocheck(&[a0, a1, a2, a3]))
            }
        }
    }

    pub fn non_address_base(&self) -> usize {
        if SEQ {
            match self.shape {
                Dimensions::D1(_) => Self::SEQ_ADDR0 + 1,
                Dimensions::D2(_, _) => Self::SEQ_ADDR1 + 1,
                Dimensions::D3(_, _, _) => Self::SEQ_ADDR2 + 1,
                Dimensions::D4(_, _, _, _) => Self::SEQ_ADDR3 + 1,
            }
        } else {
            match self.shape {
                Dimensions::D1(_) => Self::COMB_ADDR0 + 1,
                Dimensions::D2(_, _) => Self::COMB_ADDR1 + 1,
                Dimensions::D3(_, _, _) => Self::COMB_ADDR2 + 1,
                Dimensions::D4(_, _, _, _) => Self::COMB_ADDR3 + 1,
            }
        }
    }

    pub fn get_dimensions(&self) -> Dimensions {
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
            Dimensions::D1(_) => Box::new(std::iter::once(addr0)),
            Dimensions::D2(_, _) => Box::new([addr0, addr1].into_iter()),
            Dimensions::D3(_, _, _) => {
                Box::new([addr0, addr1, addr2].into_iter())
            }
            Dimensions::D4(_, _, _, _) => {
                Box::new([addr0, addr1, addr2, addr3].into_iter())
            }
        }
    }
}

#[derive(Clone)]
pub struct CombMem {
    base_port: GlobalPortIdx,
    internal_state: MemoryRegion,
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

    fn build(
        MemConfigInfo {
            base,
            global_idx,
            width,
            allow_invalid,
            size,
        }: MemConfigInfo,
        internal_state: MemoryRegion,
    ) -> Self {
        Self {
            base_port: base,
            internal_state,
            _allow_invalid_access: allow_invalid,
            addresser: MemDx::new(size),
            done_is_high: false,
            global_idx,
            _width: width,
        }
    }

    pub fn new(
        info: MemConfigInfo,
        clocks: &mut Option<&mut ClockMap>,
        state_map: &mut MemoryMap,
    ) -> Self {
        let iterator =
            repeat_n(BitVecValue::zero(info.width), info.size.size());
        let internal_state =
            state_map.allocate_region(iterator, info.global_idx, clocks);

        Self::build(info, internal_state)
    }

    pub fn new_with_init(
        info: MemConfigInfo,
        data: &[u8],
        clocks: &mut Option<&mut ClockMap>,
        state_map: &mut MemoryMap,
    ) -> Self {
        let byte_count = info.width.div_ceil(8);
        let iterator = data
            .chunks_exact(byte_count as usize)
            .map(|x| BitVecValue::from_bytes_le(x, info.width));

        let internal_state =
            state_map.allocate_region(iterator, info.global_idx, clocks);

        assert_eq!(internal_state.size(), info.size.size());
        assert!(
            data.chunks_exact(byte_count as usize)
                .remainder()
                .is_empty()
        );

        Self::build(info, internal_state)
    }

    pub fn new_with_region(info: MemConfigInfo, region: MemoryRegion) -> Self {
        Self::build(info, region)
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

    fn exec_comb(
        &self,
        port_map: &mut PortMap,
        state_map: &MemoryMap,
    ) -> UpdateResult {
        let addr: Option<usize> = self
            .addresser
            .calculate_addr(port_map, self.base_port, self.global_idx)
            .unwrap_or_default();

        let read_data = self.read_data();

        let read =
            if addr.is_some() && addr.unwrap() < self.internal_state.size() {
                let addr = addr.unwrap();
                let addr = self.internal_state.nth_entry(addr);

                port_map.insert_val_general(
                    read_data,
                    AssignedValue::cell_value(state_map[addr].clone())
                        .with_clocks(state_map.get_clock_or_default(addr)),
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

    fn exec_cycle(
        &mut self,
        port_map: &mut PortMap,
        state_map: &mut MemoryMap,
    ) -> UpdateResult {
        // These two behave like false when undefined
        let reset = port_map[self.reset_port()].as_bool().unwrap_or_default();
        let write_en = port_map[self.write_en()].as_bool().unwrap_or_default();

        let mut changed = UpdateStatus::Unchanged;

        let addr = self.addresser.calculate_addr(
            port_map,
            self.base_port,
            self.global_idx,
        )?;
        let (read_data, done) = (self.read_data(), self.done());

        if write_en && !reset {
            let Some(addr) = addr else {
                return RuntimeError::UndefinedWriteAddr(self.global_idx)
                    .into();
            };

            let addr = self.internal_state.nth_entry(addr);

            let write_data = port_map[self.write_data()]
                .as_option()
                .ok_or(RuntimeError::UndefinedWrite(self.global_idx))?;
            changed |= state_map.set_location(addr, write_data.val().clone());
            self.done_is_high = true;
            changed |= port_map
                .insert_val_general(done, AssignedValue::cell_b_high())?
        } else {
            self.done_is_high = false;
            changed |= port_map
                .insert_val_general(done, AssignedValue::cell_b_low())?;
        };

        if let Some(addr) = addr {
            let addr = self.internal_state.nth_entry(addr);

            changed |= port_map.insert_val_general(
                read_data,
                AssignedValue::cell_value(state_map[addr].clone())
                    .with_clocks(state_map.get_clock_or_default(addr)),
            )?;
        } else {
            port_map.write_undef(read_data)?;
        }
        Ok(changed)
    }

    fn get_ports(&self) -> SplitIndexRange<GlobalPortIdx> {
        SplitIndexRange::new(
            self.base_port,
            self.read_data(),
            (self.done().index() + 1).into(),
        )
    }

    fn serializer(&self) -> Option<&dyn SerializeState> {
        Some(self as &dyn SerializeState)
    }
}

impl SerializeState for CombMem {
    fn serialize<'a>(
        &self,
        code: PrintCode,
        state_map: &'a MemoryMap,
    ) -> LazySerializable<'a> {
        LazySerializable::new_array(
            code,
            state_map.get_region_slice(self.internal_state),
            self.addresser.get_dimensions(),
        )
    }

    fn dump_data(&self, state_map: &MemoryMap) -> Vec<u8> {
        state_map
            .map_region(self.internal_state, |x| x.to_bytes_le())
            .flatten()
            .collect()
    }
}

impl RaceDetectionPrimitive for CombMem {
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
        state_map: &mut MemoryMap,
    ) -> UpdateResult {
        let thread = self.infer_thread(port_map);
        if let Some(addr) = self.addresser.calculate_addr(
            port_map,
            self.base_port,
            self.global_idx,
        )?
            && addr < self.internal_state.size() {
                if let Some(thread) = thread {
                    let addr_loc = self.internal_state.nth_entry(addr);
                    let clock = &state_map.get_clock(addr_loc).unwrap();

                    if port_map[self.write_en()].as_bool().unwrap_or_default() {
                        clock
                            .check_write_with_ascription(
                                thread,
                                thread_map,
                                clock_map,
                                port_map[self.write_en()].winner().unwrap(),
                            )
                            .map_err(|e| {
                                let cell_info =
                                    clock_map.lookup_cell(*clock).unwrap();
                                e.add_cell_info(
                                    cell_info.attached_cell,
                                    cell_info.entry_number,
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

        self.exec_cycle(port_map, state_map)
    }
}

#[derive(Copy, Clone, Debug)]
enum MemOut {
    /// Points to a valid address in the memory
    Valid(MemoryLocation),
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

    fn get_value(&self, data: &MemoryMap, width: u32) -> PortValue {
        match self {
            MemOut::Valid(addr) => {
                let assigned_val =
                    AssignedValue::cell_value(data[*addr].clone())
                        .with_clocks_optional(data.get_clock(*addr));
                assigned_val.into()
            }
            MemOut::Zero => PortValue::new_cell(BitVecValue::zero(width)),
            MemOut::Undef => PortValue::new_undef(),
        }
    }
}

pub struct MemConfigInfo {
    base: GlobalPortIdx,
    global_idx: GlobalCellIdx,
    width: u32,
    allow_invalid: bool,
    size: Dimensions,
}

impl MemConfigInfo {
    pub fn new<T: Into<Dimensions>>(
        base: GlobalPortIdx,
        global_idx: GlobalCellIdx,
        width: u32,
        allow_invalid: bool,
        size: T,
    ) -> Self {
        Self {
            base,
            global_idx,
            width,
            allow_invalid,
            size: size.into(),
        }
    }
}

#[derive(Clone)]
pub struct SeqMem {
    base_port: GlobalPortIdx,
    internal_state: MemoryRegion,
    global_idx: GlobalCellIdx,
    // TODO griffin: This bool is unused in the actual struct and should either
    // be removed or
    _allow_invalid_access: bool,
    addresser: MemDx<true>,
    done_is_high: bool,
    // memory index which is currently latched
    read_out: MemOut,
    width: u32,
}

impl SeqMem {
    fn build(
        MemConfigInfo {
            base,
            global_idx,
            width,
            allow_invalid,
            size,
        }: MemConfigInfo,
        internal_state: MemoryRegion,
    ) -> Self {
        Self {
            base_port: base,
            internal_state,
            _allow_invalid_access: allow_invalid,
            addresser: MemDx::new(size),
            done_is_high: false,
            read_out: MemOut::Undef,
            global_idx,
            width,
        }
    }
    pub fn new(
        info: MemConfigInfo,
        clocks: &mut Option<&mut ClockMap>,
        state_map: &mut MemoryMap,
    ) -> Self {
        let iterator = std::iter::repeat_n(
            BitVecValue::zero(info.width),
            info.size.size(),
        );
        let internal_state =
            state_map.allocate_region(iterator, info.global_idx, clocks);

        Self::build(info, internal_state)
    }

    pub fn new_with_init(
        info: MemConfigInfo,
        data: &[u8],
        clocks: &mut Option<&mut ClockMap>,
        state_map: &mut MemoryMap,
    ) -> Self {
        let byte_count = info.width.div_ceil(8);
        let iterator = data
            .chunks_exact(byte_count as usize)
            .map(|x| BitVecValue::from_bytes_le(x, info.width));

        let internal_state =
            state_map.allocate_region(iterator, info.global_idx, clocks);

        assert_eq!(internal_state.size(), info.size.size());
        assert!(
            data.chunks_exact(byte_count as usize)
                .remainder()
                .is_empty()
        );

        Self::build(info, internal_state)
    }

    pub fn new_with_region(info: MemConfigInfo, region: MemoryRegion) -> Self {
        Self::build(info, region)
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

    fn exec_comb(
        &self,
        port_map: &mut PortMap,
        state_map: &MemoryMap,
    ) -> UpdateResult {
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
                    .get_value(state_map, self.width)
                    .into_option()
                    .unwrap(),
            )?
        } else {
            UpdateStatus::Unchanged
        };

        Ok(done_signal | out_signal)
    }

    fn exec_cycle(
        &mut self,
        port_map: &mut PortMap,
        state_map: &mut MemoryMap,
    ) -> UpdateResult {
        let mut changed = UpdateStatus::Unchanged;

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
            let addr_actual = self.internal_state.nth_entry(addr_actual);
            let write_data = port_map[self.write_data()]
                .as_option()
                .ok_or(RuntimeError::UndefinedWrite(self.global_idx))?;
            changed |=
                state_map.set_location(addr_actual, write_data.val().clone());
        } else if content_en {
            self.done_is_high = true;
            let addr_actual =
                addr.ok_or(RuntimeError::UndefinedReadAddr(self.global_idx))?;
            let addr_actual = self.internal_state.nth_entry(addr_actual);
            self.read_out = MemOut::Valid(addr_actual);
        } else {
            self.done_is_high = false;
        }

        changed |= port_map.insert_val_general(
            self.done(),
            AssignedValue::cell_value(if self.done_is_high {
                BitVecValue::new_true()
            } else {
                BitVecValue::new_false()
            }),
        )?;
        changed |= port_map.write_exact_unchecked(
            self.read_data(),
            self.read_out.get_value(state_map, self.width),
        );
        Ok(changed)
    }

    fn has_comb_path(&self) -> bool {
        false
    }

    fn has_stateful_path(&self) -> bool {
        true
    }

    fn serializer(&self) -> Option<&dyn SerializeState> {
        Some(self as &dyn SerializeState)
    }

    fn get_ports(&self) -> SplitIndexRange<GlobalPortIdx> {
        SplitIndexRange::new(
            self.base_port,
            self.read_data(),
            (self.done().index() + 1).into(),
        )
    }
}

impl SerializeState for SeqMem {
    fn serialize<'a>(
        &self,
        code: PrintCode,
        state_map: &'a MemoryMap,
    ) -> LazySerializable<'a> {
        LazySerializable::new_array(
            code,
            state_map.get_region_slice(self.internal_state),
            self.addresser.get_dimensions(),
        )
    }

    fn dump_data(&self, state_map: &MemoryMap) -> Vec<u8> {
        state_map
            .map_region(self.internal_state, |x| x.to_bytes_le())
            .flatten()
            .collect()
    }
}

impl RaceDetectionPrimitive for SeqMem {
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
        state_map: &mut MemoryMap,
    ) -> UpdateResult {
        let thread = self.infer_thread(port_map);
        if let Some(addr) = self.addresser.calculate_addr(
            port_map,
            self.base_port,
            self.global_idx,
        )?
            && addr < self.internal_state.size() {
                let addr_loc = self.internal_state.nth_entry(addr);

                let clock = state_map.get_clock(addr_loc).unwrap();

                if port_map[self.write_enable()].as_bool().unwrap_or_default()
                    && port_map[self.content_enable()]
                        .as_bool()
                        .unwrap_or_default()
                {
                    clock
                        .check_write_with_ascription(
                            thread.expect(
                                "unable to determine thread for seq mem",
                            ),
                            thread_map,
                            clock_map,
                            port_map[self.write_enable()].winner().unwrap(),
                        )
                        .map_err(|e| {
                            let cell_info =
                                clock_map.lookup_cell(clock).unwrap();
                            e.add_cell_info(
                                cell_info.attached_cell,
                                cell_info.entry_number,
                            )
                        })?;
                } else if port_map[self.content_enable()]
                    .as_bool()
                    .unwrap_or_default()
                {
                    // we don't want to check the read here, since that makes
                    // merely assigning the content_en constitute a read even if
                    // the value is never used
                }
            }
        self.exec_cycle(port_map, state_map)
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
