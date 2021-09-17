use std::collections::VecDeque;

use super::{Primitive, Serializeable};
use crate::utils::construct_bindings;
use crate::values::Value;
use calyx::ir;

pub(super) fn get_param<S>(params: &ir::Binding, target: S) -> Option<u64>
where
    S: AsRef<str>,
{
    params.iter().find_map(|(id, x)| {
        if id == target.as_ref() {
            Some(*x)
        } else {
            None
        }
    })
}

/// Pipelined Multiplication (3 cycles)
/// Still bounded by u64.
/// How to use:
/// [Primitive::execute] with the desired bindings.
/// To capture these bindings into the internal (out) queue, [Primitive::do_tick].
/// The product associated with a given input will be output on the third [do_tick()].
/// Note: Calling [Primitive::execute] multiple times before [Primitive::do_tick] has no effect; only the last
/// set of inputs prior to the [Primitve::do_tick] will be saved.
#[derive(Default)]
pub struct StdMultPipe {
    pub width: u64,
    pub product: Value,
    update: Option<Value>,
    queue: VecDeque<Option<Value>>, //invariant: always length 2.
}

impl StdMultPipe {
    pub fn from_constants(width: u64) -> Self {
        StdMultPipe {
            width,
            product: Value::zeroes(width as usize),
            update: None,
            queue: VecDeque::from(vec![None, None]),
        }
    }

    pub fn new(params: &ir::Binding) -> Self {
        let width = params
            .iter()
            .find(|(n, _)| n.as_ref() == "WIDTH")
            .expect("Missing `WIDTH` param from std_mult_pipe binding")
            .1;
        Self::from_constants(width)
    }
}

impl Primitive for StdMultPipe {
    fn do_tick(&mut self) -> Vec<(ir::Id, Value)> {
        let out = self.queue.pop_back();
        //push update to the front
        self.queue.push_front(self.update.take());
        //assert queue still has length 2
        assert_eq!(
            self.queue.len(),
            2,
            "std_mult_pipe's internal queue has length {} != 2",
            self.queue.len()
        );
        if let Some(Some(out)) = out {
            //return vec w/ out and done
            vec![
                (ir::Id::from("out"), out),
                (ir::Id::from("done"), Value::bit_high()),
            ]
        } else {
            //return empty vec
            vec![]
        }
    }
    //
    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(calyx::ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "left" => assert_eq!(v.len() as u64, self.width),
                "right" => assert_eq!(v.len() as u64, self.width),
                "go" => assert_eq!(v.len() as u64, 1),
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(
        &mut self,
        inputs: &[(calyx::ir::Id, &Value)],
    ) -> Vec<(calyx::ir::Id, Value)> {
        //unwrap the arguments, left, right, and go
        let (_, left) = inputs.iter().find(|(id, _)| id == "left").unwrap();
        let (_, right) = inputs.iter().find(|(id, _)| id == "right").unwrap();
        let (_, go) = inputs.iter().find(|(id, _)| id == "go").unwrap();
        //continue computation
        if go.as_u64() == 1 {
            self.update =
                Some(Value::from(left.as_u64() * right.as_u64(), self.width));
        } else {
            self.update = None;
        }

        //if go is low don't do anything (don't overwrite Update)
        vec![]
    }

    fn reset(
        &mut self,
        _: &[(calyx::ir::Id, &Value)],
    ) -> Vec<(calyx::ir::Id, crate::values::Value)> {
        self.update = None;
        self.queue = VecDeque::from(vec![None, None]);
        vec![
            (ir::Id::from("out"), self.product.clone()),
            (ir::Id::from("done"), Value::bit_low()),
        ]
    }

    fn serialize(&self) -> Serializeable {
        Serializeable::Array(
            vec![self.product.clone()]
                .iter()
                .map(Value::as_u64)
                .collect(),
            1.into(),
        )
    }
}

///Pipelined Division (3 cycles)
///Still bounded by u64.
///How to use:
///[execute] with the desired bindings. To capture these bindings
///into the internal (out_quotient, out_remainder) queue, [do_tick()].
///The out_quotient and out_remainder associated with a given input will
///be output on the third [do_tick()].
///Note: Calling [execute] multiple times before [do_tick()] has no effect; only
///the last set of inputs prior to the [do_tick()] will be saved.
#[derive(Default)]
pub struct StdDivPipe {
    pub width: u64,
    pub quotient: Value,
    pub remainder: Value,
    update: Option<(Value, Value)>, //first is quotient, second is remainder
    queue: VecDeque<Option<(Value, Value)>>, //invariant: always length 2
}

impl StdDivPipe {
    pub fn from_constants(width: u64) -> Self {
        StdDivPipe {
            width,
            quotient: Value::zeroes(width as usize),
            remainder: Value::zeroes(width as usize),
            update: None,
            queue: VecDeque::from(vec![None, None]),
        }
    }

    pub fn new(params: &ir::Binding) -> Self {
        let width = params
            .iter()
            .find(|(n, _)| n.as_ref() == "WIDTH")
            .expect("Missing `WIDTH` param from std_mult_pipe binding")
            .1;
        Self::from_constants(width)
    }
}

impl Primitive for StdDivPipe {
    fn do_tick(&mut self) -> Vec<(ir::Id, Value)> {
        let out = self.queue.pop_back();
        self.queue.push_front(self.update.take());
        assert_eq!(
            self.queue.len(),
            2,
            "std_div_pipe's internal queue has length {} != 2",
            self.queue.len()
        );
        if let Some(Some((q, r))) = out {
            vec![
                (ir::Id::from("out_quotient"), q),
                (ir::Id::from("out_remainder"), r),
                (ir::Id::from("done"), Value::bit_high()),
            ]
        } else {
            vec![]
        }
    }

    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(calyx::ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "left" => assert_eq!(v.len() as u64, self.width),
                "right" => assert_eq!(v.len() as u64, self.width),
                "go" => assert_eq!(v.len() as u64, 1),
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(
        &mut self,
        inputs: &[(calyx::ir::Id, &Value)],
    ) -> Vec<(calyx::ir::Id, Value)> {
        //unwrap the arguments, left, right, and go
        let (_, left) = inputs.iter().find(|(id, _)| id == "left").unwrap();
        let (_, right) = inputs.iter().find(|(id, _)| id == "right").unwrap();
        let (_, go) = inputs.iter().find(|(id, _)| id == "go").unwrap();
        //continue computation
        if go.as_u64() == 1 {
            let q = left.as_u64() / right.as_u64();
            let r = left.as_u64() % right.as_u64();
            self.update =
                Some((Value::from(q, self.width), Value::from(r, self.width)));
        } else {
            self.update = None;
        }
        vec![]
    }

    fn reset(
        &mut self,
        _: &[(calyx::ir::Id, &Value)],
    ) -> Vec<(calyx::ir::Id, Value)> {
        self.update = None;
        self.queue = VecDeque::from(vec![None, None]);
        vec![
            (ir::Id::from("out_quotient"), self.quotient.clone()),
            (ir::Id::from("out_remainder"), self.remainder.clone()),
            (ir::Id::from("done"), Value::bit_low()),
        ]
    }

    fn serialize(&self) -> Serializeable {
        Serializeable::Array(
            //vec![self.left.clone(), self.right.clone(), self.product.clone()]
            vec![self.quotient.clone(), self.remainder.clone()]
                .iter()
                .map(Value::as_u64)
                .collect(),
            2.into(),
        )
    }
}

/// A register.
#[derive(Default)]
pub struct StdReg {
    pub width: u64,
    pub data: [Value; 1],
    update: Option<Value>,
    write_en: bool,
}

impl StdReg {
    pub fn from_constants(width: u64) -> Self {
        StdReg {
            width,
            data: [Value::new(width as usize)],
            update: None,
            write_en: false,
        }
    }

    pub fn new(params: &ir::Binding) -> Self {
        let width = params
            .iter()
            .find(|(n, _)| n.as_ref() == "WIDTH")
            .expect("Missing `WIDTH` param from std_reg binding")
            .1;
        Self::from_constants(width)
    }
}

impl Primitive for StdReg {
    fn do_tick(&mut self) -> Vec<(ir::Id, Value)> {
        //first commit any updates
        if let Some(val) = self.update.take() {
            self.data[0] = val;
        }
        //then, based on write_en, return
        if self.write_en {
            self.write_en = false; //we are done for this cycle
                                   //if do_tick() is called again w/o an execute() preceeding it,
                                   //then done will be low.
            vec![
                (ir::Id::from("out"), self.data[0].clone()),
                (ir::Id::from("done"), Value::bit_high()),
            ]
        } else {
            //done is low, but there is still data in this reg to return
            vec![
                (ir::Id::from("out"), self.data[0].clone()),
                (ir::Id::from("done"), Value::bit_low()),
            ]
        }
    }

    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(calyx::ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "in" => assert_eq!(v.len() as u64, self.width),
                "write_en" => assert_eq!(v.len(), 1),
                "clk" => assert_eq!(v.len(), 1),
                "reset" => assert_eq!(v.len(), 1),
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(
        &mut self,
        inputs: &[(calyx::ir::Id, &Value)],
    ) -> Vec<(calyx::ir::Id, Value)> {
        //unwrap the arguments
        let (_, input) = inputs.iter().find(|(id, _)| id == "in").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        //write the input to the register
        if write_en.as_u64() == 1 {
            self.update = Some((*input).clone());
            //put cycle_count as 1! B/c do_tick() should return a high done
            self.write_en = true;
        } else {
            self.update = None;
            self.write_en = false;
        }
        vec![]
    }

    fn reset(
        &mut self,
        _: &[(calyx::ir::Id, &Value)],
    ) -> Vec<(calyx::ir::Id, Value)> {
        self.update = None;
        self.write_en = false; //might be redundant, not too sure when reset is used
        vec![
            (ir::Id::from("out"), self.data[0].clone()),
            (ir::Id::from("done"), Value::zeroes(1)),
        ]
    }

    fn serialize(&self) -> Serializeable {
        Serializeable::Val(self.data[0].as_u64())
    }
}

/// A one-dimensional memory. Initialized with
/// StdMemD1.new(WIDTH, SIZE, IDX_SIZE) where:
/// * WIDTH - Size of an individual memory slot.
/// * SIZE - Number of slots in the memory.
/// * IDX_SIZE - The width of the index given to the memory.
///
/// To write to a memory, the [write_en] must be high.
/// Inputs:
/// * addr0: IDX_SIZE - The index to be accessed or updated.
/// * write_data: WIDTH - Data to be written to the selected memory slot.
/// * write_en: 1 - One bit write enabled signal, causes the memory to write
///             write_data to the slot indexed by addr0.
///
/// Outputs:
/// * read_data: WIDTH - The value stored at addr0. This value is combinational
///              with respect to addr0.
/// * done: 1 - The done signal for the memory. This signal goes high for one
///         cycle after finishing a write to the memory.
#[derive(Debug)]
pub struct StdMemD1 {
    pub width: u64,    // size of individual piece of mem
    pub size: u64,     // # slots of mem
    pub idx_size: u64, // # bits needed to index a piece of mem
    pub data: Vec<Value>,
    update: Option<(u64, Value)>,
    write_en: bool,
    last_index: u64,
}

impl StdMemD1 {
    pub fn from_constants(width: u64, size: u64, idx_size: u64) -> Self {
        let bindings = construct_bindings(
            [("WIDTH", width), ("SIZE", size), ("IDX_SIZE", idx_size)].iter(),
        );
        Self::new(&bindings)
    }
    /// Instantiates a new StdMemD1 storing data of width [width], containing [size]
    /// slots for memory, accepting indecies (addr0) of width [idx_size].
    /// Note: if [idx_size] is smaller than the length of [size]'s binary representation,
    /// you will not be able to access the slots near the end of the memory.
    pub fn new(params: &ir::Binding) -> StdMemD1 {
        let width = get_param(params, "WIDTH")
            .expect("Missing width param for std_mem_d1");
        let size = get_param(params, "SIZE")
            .expect("Missing size param for std_mem_d1");
        let idx_size = get_param(params, "IDX_SIZE")
            .expect("Missing idx_size param for std_mem_d1");

        let data = vec![Value::zeroes(width as usize); size as usize];
        StdMemD1 {
            width,
            size,     //how many slots of memory in the vector
            idx_size, //the width of the values used to address the memory
            data,
            update: None,
            write_en: false,
            last_index: 0,
        }
    }

    pub fn initialize_memory(&mut self, vals: &[Value]) {
        assert_eq!(self.size as usize, vals.len());

        for (idx, val) in vals.iter().enumerate() {
            assert_eq!(val.len(), self.width as usize);
            self.data[idx] = val.clone()
        }
    }
}

impl Primitive for StdMemD1 {
    fn do_tick(&mut self) -> Vec<(ir::Id, Value)> {
        //if there is an update, update and return along w/ a done
        //else this memory was used combinationally and there is nothing to tick
        if self.last_index >= self.size {
            panic!(
                "[std_mem_d1] Supplied with an invalid index: {}",
                self.last_index
            )
        }

        if self.write_en {
            assert!(self.update.is_some());
            //set cycle_count to 0 for future
            self.write_en = false;
            //take update
            if let Some((idx, val)) = self.update.take() {
                //alter data
                self.data[idx as usize] = val;
                //return vec w/ done
                vec![
                    (
                        ir::Id::from("read_data"),
                        self.data[idx as usize].clone(),
                    ),
                    (ir::Id::from("done"), Value::bit_high()),
                ]
            } else {
                panic!("[std_mem_d1] : self.update is None, while self.cycle_count == 1");
            }
        } else {
            vec![(ir::Id::from("done"), Value::bit_low())]
        }
    }

    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "write_data" => assert_eq!(v.len() as u64, self.width),
                "write_en" => assert_eq!(v.len(), 1),
                "addr0" => {
                    assert!(v.as_u64() < self.size);
                    assert_eq!(v.len() as u64, self.idx_size, "std_mem_d1: addr0 is not same width ({}) as idx_size ({})", v.len(), self.idx_size)
                }
                "clk" => assert_eq!(v.len(), 1),
                "reset" => assert_eq!(v.len(), 1),
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(&mut self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, Value)> {
        let (_, input) =
            inputs.iter().find(|(id, _)| id == "write_data").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let addr0 = addr0.as_u64();
        self.last_index = addr0;
        if write_en.as_u64() == 1 {
            self.update = Some((addr0, (*input).clone()));
            self.write_en = true;
        } else {
            self.update = None;
            self.write_en = false;
        }
        //read_data is combinational w.r.t addr0;
        //if there was an update, [do_tick()] will return a vector w/ a done value
        //else, empty vector return
        vec![(
            ir::Id::from("read_data"),
            if addr0 < self.size {
                self.data[addr0 as usize].clone()
            } else {
                Value::zeroes(self.width as usize)
            },
        )]
    }

    fn reset(&mut self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, Value)> {
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        //so we don't have to keep using .as_u64()
        let addr0 = addr0.as_u64();
        //check that input data is the appropriate width as well
        let old = self.data[addr0 as usize].clone();
        //also clear update
        self.update = None;
        self.write_en = false;
        self.last_index = addr0;
        vec![
            ("read_data".into(), old),
            (ir::Id::from("done"), Value::zeroes(1)),
        ]
    }

    fn serialize(&self) -> Serializeable {
        Serializeable::Array(
            self.data.iter().map(Value::as_u64).collect(),
            (self.size as usize,).into(),
        )
    }

    fn has_serializeable_state(&self) -> bool {
        true
    }
}

///std_memd2 :
/// A two-dimensional memory.
/// Parameters:
/// WIDTH - Size of an individual memory slot.
/// D0_SIZE - Number of memory slots for the first index.
/// D1_SIZE - Number of memory slots for the second index.
/// D0_IDX_SIZE - The width of the first index.
/// D1_IDX_SIZE - The width of the second index.
/// Inputs:
/// addr0: D0_IDX_SIZE - The first index into the memory
/// addr1: D1_IDX_SIZE - The second index into the memory
/// write_data: WIDTH - Data to be written to the selected memory slot
/// write_en: 1 - One bit write enabled signal, causes the memory to write write_data to the slot indexed by addr0 and addr1
/// Outputs:
/// read_data: WIDTH - The value stored at mem[addr0][addr1]. This value is combinational with respect to addr0 and addr1.
/// done: 1: The done signal for the memory. This signal goes high for one cycle after finishing a write to the memory.
#[derive(Clone, Debug)]
pub struct StdMemD2 {
    pub width: u64,   // size of individual piece of mem
    pub d0_size: u64, // # slots of mem
    pub d1_size: u64,
    pub d0_idx_size: u64,
    pub d1_idx_size: u64, // # bits needed to index a piece of mem
    pub data: Vec<Value>,
    update: Option<(u64, Value)>,
    write_en: bool,
    last_idx: (u64, u64),
}

impl StdMemD2 {
    pub fn from_constants(
        width: u64,
        d0_size: u64,
        d1_size: u64,
        d0_idx_size: u64,
        d1_idx_size: u64,
    ) -> Self {
        let bindings = construct_bindings(
            [
                ("WIDTH", width),
                ("D0_SIZE", d0_size),
                ("D1_SIZE", d1_size),
                ("D0_IDX_SIZE", d0_idx_size),
                ("D1_IDX_SIZE", d1_idx_size),
            ]
            .iter(),
        );
        Self::new(&bindings)
    }

    #[inline]
    fn max_idx(&self) -> u64 {
        self.d0_size * self.d1_size
    }

    /// Instantiates a new StdMemD2 storing data of width [width], containing
    /// [d0_size] * [d1_size] slots for memory, accepting indecies [addr0][addr1] of widths
    /// [d0_idx_size] and [d1_idx_size] respectively.
    /// Initially the memory is filled with all 0s.
    pub fn new(params: &ir::Binding) -> StdMemD2 {
        let width = get_param(params, "WIDTH")
            .expect("Missing width parameter for std_mem_d2");
        let d0_size = get_param(params, "D0_SIZE")
            .expect("Missing d0_size parameter for std_mem_d2");
        let d1_size = get_param(params, "D1_SIZE")
            .expect("Missing d1_size parameter for std_mem_d2");
        let d0_idx_size = get_param(params, "D0_IDX_SIZE")
            .expect("Missing d0_idx_size parameter for std_mem_d2");
        let d1_idx_size = get_param(params, "D1_IDX_SIZE")
            .expect("Missing d1_idx_size parameter for std_mem_d2");

        let data =
            vec![Value::zeroes(width as usize); (d0_size * d1_size) as usize];
        StdMemD2 {
            width,
            d0_size,
            d1_size,
            d0_idx_size,
            d1_idx_size,
            data,
            update: None,
            write_en: false,
            last_idx: (0, 0),
        }
    }

    pub fn initialize_memory(&mut self, vals: &[Value]) {
        assert_eq!((self.d0_size * self.d1_size) as usize, vals.len());

        for (idx, val) in vals.iter().enumerate() {
            assert_eq!(val.len(), self.width as usize);
            self.data[idx] = val.clone()
        }
    }

    #[inline]
    fn calc_addr(&self, addr0: u64, addr1: u64) -> u64 {
        addr0 * self.d1_size + addr1
    }
}

impl Primitive for StdMemD2 {
    //null-op for now
    fn do_tick(&mut self) -> Vec<(ir::Id, Value)> {
        if self.calc_addr(self.last_idx.0, self.last_idx.1) >= self.max_idx() {
            panic!(
                "[std_mem_d2] Supplied with an invalid index {},{}",
                self.last_idx.0, self.last_idx.1
            );
        }
        if self.write_en {
            assert!(self.update.is_some());
            self.write_en = false;
            if let Some((idx, val)) = self.update.take() {
                self.data[idx as usize] = val;
                vec![
                    (
                        ir::Id::from("read_data"),
                        self.data[idx as usize].clone(),
                    ),
                    (ir::Id::from("done"), Value::bit_high()),
                ]
            } else {
                panic!("[std_mem_d2]: self.update is None, while self.cycle_count == 1");
            }
        } else {
            vec![(ir::Id::from("done"), Value::bit_low())]
        }
    }

    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "write_data" => assert_eq!(v.len() as u64, self.width),
                "write_en" => assert_eq!(v.len(), 1),
                "addr0" => {
                    assert!(v.as_u64() < self.d0_size);
                    assert_eq!(v.len() as u64, self.d0_idx_size)
                }
                "addr1" => {
                    assert!(v.as_u64() < self.d1_size);
                    assert_eq!(v.len() as u64, self.d1_idx_size)
                }
                "clk" => assert_eq!(v.len(), 1),
                "reset" => assert_eq!(v.len(), 1),
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(&mut self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, Value)> {
        //unwrap the arguments
        //these come from the primitive definition in verilog
        //don't need to depend on the user -- just make sure this matches primitive + calyx + verilog defs
        let (_, input) =
            inputs.iter().find(|(id, _)| id == "write_data").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();

        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();
        self.last_idx = (addr0, addr1);
        let real_addr = self.calc_addr(addr0, addr1);

        if write_en.as_u64() == 1 {
            self.update = Some((real_addr, (*input).clone()));
            self.write_en = true;
        } else {
            self.update = None;
            self.write_en = false;
        }
        vec![(
            ir::Id::from("read_data"),
            if real_addr < self.max_idx() {
                self.data[real_addr as usize].clone()
            } else {
                Value::zeroes(self.width as usize)
            },
        )]
    }

    fn reset(&mut self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, Value)> {
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();

        let real_addr = self.calc_addr(addr0, addr1);

        let old = self.data[real_addr as usize].clone();

        //clear update
        self.update = None;
        self.write_en = false;
        self.last_idx = (addr0, addr1);

        vec![
            (ir::Id::from("read_data"), old),
            (ir::Id::from("done"), Value::zeroes(1)),
        ]
    }

    fn serialize(&self) -> Serializeable {
        Serializeable::Array(
            self.data.iter().map(Value::as_u64).collect(),
            (self.d0_size as usize, self.d1_size as usize).into(),
        )
    }
}

///std_memd3 :
/// A three-dimensional memory.
/// Parameters:
/// WIDTH - Size of an individual memory slot.
/// D0_SIZE - Number of memory slots for the first index.
/// D1_SIZE - Number of memory slots for the second index.
/// D2_SIZE - Number of memory slots for the third index.
/// D0_IDX_SIZE - The width of the first index.
/// D1_IDX_SIZE - The width of the second index.
/// D2_IDX_SIZE - The width of the third index.
/// Inputs:
/// addr0: D0_IDX_SIZE - The first index into the memory
/// addr1: D1_IDX_SIZE - The second index into the memory
/// addr2: D2_IDX_SIZE - The third index into the memory
/// write_data: WIDTH - Data to be written to the selected memory slot
/// write_en: 1 - One bit write enabled signal, causes the memory to write write_data to the slot indexed by addr0, addr1, and addr2
/// Outputs:
/// read_data: WIDTH - The value stored at mem[addr0][addr1][addr2]. This value is combinational with respect to addr0, addr1, and addr2.
/// done: 1: The done signal for the memory. This signal goes high for one cycle after finishing a write to the memory.
#[derive(Clone, Debug)]
pub struct StdMemD3 {
    width: u64,
    d0_size: u64,
    d1_size: u64,
    d2_size: u64,
    d0_idx_size: u64,
    d1_idx_size: u64,
    d2_idx_size: u64,
    data: Vec<Value>,
    update: Option<(u64, Value)>,
    write_en: bool,
    last_idx: (u64, u64, u64),
}

impl StdMemD3 {
    pub fn from_constants(
        width: u64,
        d0_size: u64,
        d1_size: u64,
        d2_size: u64,
        d0_idx_size: u64,
        d1_idx_size: u64,
        d2_idx_size: u64,
    ) -> Self {
        let bindings = construct_bindings(
            [
                ("WIDTH", width),
                ("D0_SIZE", d0_size),
                ("D1_SIZE", d1_size),
                ("D2_SIZE", d2_size),
                ("D0_IDX_SIZE", d0_idx_size),
                ("D1_IDX_SIZE", d1_idx_size),
                ("D2_IDX_SIZE", d2_idx_size),
            ]
            .iter(),
        );
        Self::new(&bindings)
    }
    /// Instantiates a new StdMemD3 storing data of width [width], containing
    /// [d0_size] * [d1_size] * [d2_size] slots for memory, accepting indecies [addr0][addr1][addr2] of widths
    /// [d0_idx_size], [d1_idx_size], and [d2_idx_size] respectively.
    /// Initially the memory is filled with all 0s.
    pub fn new(params: &ir::Binding) -> StdMemD3 {
        let width = get_param(params, "WIDTH")
            .expect("Missing width parameter for std_mem_d3");
        let d0_size = get_param(params, "D0_SIZE")
            .expect("Missing d0_size parameter for std_mem_d3");
        let d1_size = get_param(params, "D1_SIZE")
            .expect("Missing d1_size parameter for std_mem_d3");
        let d2_size = get_param(params, "D2_SIZE")
            .expect("Missing d2_size parameter for std_mem_d3");
        let d0_idx_size = get_param(params, "D0_IDX_SIZE")
            .expect("Missing d0_idx_size parameter for std_mem_d3");
        let d1_idx_size = get_param(params, "D1_IDX_SIZE")
            .expect("Missing d1_idx_size parameter for std_mem_d3");
        let d2_idx_size = get_param(params, "D2_IDX_SIZE")
            .expect("Missing d2_idx_size parameter for std_mem_d3");

        let data = vec![
            Value::zeroes(width as usize);
            (d0_size * d1_size * d2_size) as usize
        ];
        StdMemD3 {
            width,
            d0_size,
            d1_size,
            d2_size,
            d0_idx_size,
            d1_idx_size,
            d2_idx_size,
            data,
            update: None,
            write_en: false,
            last_idx: (0, 0, 0),
        }
    }

    pub fn initialize_memory(&mut self, vals: &[Value]) {
        assert_eq!(
            (self.d0_size * self.d1_size * self.d2_size) as usize,
            vals.len()
        );

        for (idx, val) in vals.iter().enumerate() {
            assert_eq!(val.len(), self.width as usize);
            self.data[idx] = val.clone()
        }
    }

    #[inline]
    fn max_idx(&self) -> u64 {
        self.d0_size * self.d1_size * self.d2_size
    }

    #[inline]
    fn calc_addr(&self, addr0: u64, addr1: u64, addr2: u64) -> u64 {
        self.d2_size * (addr0 * self.d1_size + addr1) + addr2
    }
}

impl Primitive for StdMemD3 {
    //null-op for now
    fn do_tick(&mut self) -> Vec<(ir::Id, Value)> {
        let (addr0, addr1, addr2) = self.last_idx;
        if self.calc_addr(addr0, addr1, addr2) >= self.max_idx() {
            panic!(
                "[std_mem_d3] Supplied with invalid index {:?}",
                self.last_idx
            )
        }

        if self.write_en {
            assert!(self.update.is_some());
            self.write_en = false;
            if let Some((idx, val)) = self.update.take() {
                self.data[idx as usize] = val;
                vec![
                    (
                        ir::Id::from("read_data"),
                        self.data[idx as usize].clone(),
                    ),
                    (ir::Id::from("done"), Value::bit_high()),
                ]
            } else {
                panic!("[std_mem_d2] : self.update is None, while self.cycle_count == 1");
            }
        } else {
            vec![(ir::Id::from("done"), Value::bit_low())]
        }
    }

    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "write_data" => assert_eq!(v.len() as u64, self.width),
                "write_en" => assert_eq!(v.len(), 1),
                "addr0" => {
                    assert!(v.as_u64() < self.d0_size);
                    assert_eq!(v.len() as u64, self.d0_idx_size)
                }
                "addr1" => {
                    assert!(v.as_u64() < self.d1_size);
                    assert_eq!(v.len() as u64, self.d1_idx_size)
                }
                "addr2" => {
                    assert!(v.as_u64() < self.d2_size);
                    assert_eq!(v.len() as u64, self.d2_idx_size)
                }
                "clk" => assert_eq!(v.len(), 1),
                "reset" => assert_eq!(v.len(), 1),
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(&mut self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, Value)> {
        //unwrap the arguments
        //these come from the primitive definition in verilog
        //don't need to depend on the user -- just make sure this matches primitive + calyx + verilog defs
        let (_, input) =
            inputs.iter().find(|(id, _)| id == "write_data").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        let (_, addr2) = inputs.iter().find(|(id, _)| id == "addr2").unwrap();

        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();
        let addr2 = addr2.as_u64();
        self.last_idx = (addr0, addr1, addr2);

        let real_addr = self.calc_addr(addr0, addr1, addr2);
        if write_en.as_u64() == 1 {
            self.update = Some((real_addr, (*input).clone()));
            self.write_en = true;
        } else {
            self.update = None;
            self.write_en = false;
        }
        vec![(
            ir::Id::from("read_data"),
            if real_addr < self.max_idx() {
                self.data[real_addr as usize].clone()
            } else {
                Value::zeroes(self.width as usize)
            },
        )]
    }

    fn reset(&mut self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, Value)> {
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        let (_, addr2) = inputs.iter().find(|(id, _)| id == "addr2").unwrap();
        //check that addr0 is not out of bounds and that it is the proper width!
        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();
        let addr2 = addr2.as_u64();

        self.last_idx = (addr0, addr1, addr2);

        let real_addr = self.calc_addr(addr0, addr1, addr2);

        let old = self.data[real_addr as usize].clone();
        //clear update, and set write_en false
        self.update = None;
        self.write_en = false;
        vec![
            (ir::Id::from("read_data"), old),
            (ir::Id::from("done"), Value::zeroes(1)),
        ]
    }

    fn serialize(&self) -> Serializeable {
        Serializeable::Array(
            self.data.iter().map(Value::as_u64).collect(),
            (
                self.d0_size as usize,
                self.d1_size as usize,
                self.d2_size as usize,
            )
                .into(),
        )
    }
}

///std_memd4
/// std_mem_d4
/// A four-dimensional memory.
/// Parameters:
/// WIDTH - Size of an individual memory slot.
/// D0_SIZE - Number of memory slots for the first index.
/// D1_SIZE - Number of memory slots for the second index.
/// D2_SIZE - Number of memory slots for the third index.
/// D3_SIZE - Number of memory slots for the fourth index.
/// D0_IDX_SIZE - The width of the first index.
/// D1_IDX_SIZE - The width of the second index.
/// D2_IDX_SIZE - The width of the third index.
/// D3_IDX_SIZE - The width of the fourth index.
/// Inputs:
/// addr0: D0_IDX_SIZE - The first index into the memory
/// addr1: D1_IDX_SIZE - The second index into the memory
/// addr2: D2_IDX_SIZE - The third index into the memory
/// addr3: D3_IDX_SIZE - The fourth index into the memory
/// write_data: WIDTH - Data to be written to the selected memory slot
/// write_en: 1 - One bit write enabled signal, causes the memory to write write_data to the slot indexed by addr0, addr1, addr2, and addr3
/// Outputs:
/// read_data: WIDTH - The value stored at mem[addr0][addr1][addr2][addr3]. This value is combinational with respect to addr0, addr1, addr2, and addr3.
/// done: 1: The done signal for the memory. This signal goes high for one cycle after finishing a write to the memory.
#[derive(Clone, Debug)]
pub struct StdMemD4 {
    width: u64,
    d0_size: u64,
    d1_size: u64,
    d2_size: u64,
    d3_size: u64,
    d0_idx_size: u64,
    d1_idx_size: u64,
    d2_idx_size: u64,
    d3_idx_size: u64,
    data: Vec<Value>,
    update: Option<(u64, Value)>,
    write_en: bool,
    last_idx: (u64, u64, u64, u64),
}

impl StdMemD4 {
    #[allow(clippy::too_many_arguments)]
    pub fn from_constants(
        width: u64,
        d0_size: u64,
        d1_size: u64,
        d2_size: u64,
        d3_size: u64,
        d0_idx_size: u64,
        d1_idx_size: u64,
        d2_idx_size: u64,
        d3_idx_size: u64,
    ) -> Self {
        let bindings = construct_bindings(
            [
                ("WIDTH", width),
                ("D0_SIZE", d0_size),
                ("D1_SIZE", d1_size),
                ("D2_SIZE", d2_size),
                ("D3_SIZE", d3_size),
                ("D0_IDX_SIZE", d0_idx_size),
                ("D1_IDX_SIZE", d1_idx_size),
                ("D2_IDX_SIZE", d2_idx_size),
                ("D3_IDX_SIZE", d3_idx_size),
            ]
            .iter(),
        );
        Self::new(&bindings)
    }
    // Instantiates a new StdMemD3 storing data of width [width], containing
    /// [d0_size] * [d1_size] * [d2_size] * [d3_size] slots for memory, accepting indecies [addr0][addr1][addr2][addr3] of widths
    /// [d0_idx_size], [d1_idx_size], [d2_idx_size] and [d3_idx_size] respectively.
    /// Initially the memory is filled with all 0s.
    pub fn new(params: &ir::Binding) -> StdMemD4 {
        // yes this was incredibly tedious to write. Why do you ask?
        let width = get_param(params, "WIDTH")
            .expect("Missing width parameter for std_mem_d4");
        let d0_size = get_param(params, "D0_SIZE")
            .expect("Missing d0_size parameter for std_mem_d4");
        let d1_size = get_param(params, "D1_SIZE")
            .expect("Missing d1_size parameter for std_mem_d4");
        let d2_size = get_param(params, "D2_SIZE")
            .expect("Missing d2_size parameter for std_mem_d4");
        let d3_size = get_param(params, "D3_SIZE")
            .expect("Missing d3_size parameter for std_mem_d4");
        let d0_idx_size = get_param(params, "D0_IDX_SIZE")
            .expect("Missing d0_idx_size parameter for std_mem_d4");
        let d1_idx_size = get_param(params, "D1_IDX_SIZE")
            .expect("Missing d1_idx_size parameter for std_mem_d4");
        let d2_idx_size = get_param(params, "D2_IDX_SIZE")
            .expect("Missing d2_idx_size parameter for std_mem_d4");
        let d3_idx_size = get_param(params, "D3_IDX_SIZE")
            .expect("Missing d3_idx_size parameter for std_mem_d4");

        let data = vec![
            Value::zeroes(width as usize);
            (d0_size * d1_size * d2_size * d3_size) as usize
        ];
        StdMemD4 {
            width,
            d0_size,
            d1_size,
            d2_size,
            d3_size,
            d0_idx_size,
            d1_idx_size,
            d2_idx_size,
            d3_idx_size,
            data,
            update: None,
            write_en: false,
            last_idx: (0, 0, 0, 0),
        }
    }

    pub fn initialize_memory(&mut self, vals: &[Value]) {
        assert_eq!(
            (self.d0_size * self.d1_size * self.d2_size * self.d3_size)
                as usize,
            vals.len()
        );

        for (idx, val) in vals.iter().enumerate() {
            assert_eq!(val.len(), self.width as usize);
            self.data[idx] = val.clone()
        }
    }

    #[inline]
    fn calc_addr(&self, addr0: u64, addr1: u64, addr2: u64, addr3: u64) -> u64 {
        self.d3_size * (self.d2_size * (addr0 * self.d1_size + addr1) + addr2)
            + addr3
    }

    fn max_idx(&self) -> u64 {
        self.d0_size * self.d1_size * self.d2_size * self.d3_size
    }
}
impl Primitive for StdMemD4 {
    //null-op for now
    fn do_tick(&mut self) -> Vec<(ir::Id, Value)> {
        let (addr0, addr1, addr2, addr3) = self.last_idx;
        if self.calc_addr(addr0, addr1, addr2, addr3) >= self.max_idx() {
            panic!("[std_mem_d4] Supplied an invalid index {:?}", self.last_idx)
        }

        if self.write_en {
            assert!(self.update.is_some());
            self.write_en = false;
            if let Some((idx, val)) = self.update.take() {
                self.data[idx as usize] = val;
                vec![
                    (
                        ir::Id::from("read_data"),
                        self.data[idx as usize].clone(),
                    ),
                    (ir::Id::from("done"), Value::bit_high()),
                ]
            } else {
                panic!("[std_mem_d4] : self.update is None, while self.cycle_count == 1");
            }
        } else {
            vec![(ir::Id::from("done"), Value::bit_low())]
        }
    }

    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "write_data" => assert_eq!(v.len() as u64, self.width),
                "write_en" => assert_eq!(v.len(), 1),
                "addr0" => {
                    assert!(v.as_u64() < self.d0_size);
                    assert_eq!(v.len() as u64, self.d0_idx_size)
                }
                "addr1" => {
                    assert!(v.as_u64() < self.d1_size);
                    assert_eq!(v.len() as u64, self.d1_idx_size)
                }
                "addr2" => {
                    assert!(v.as_u64() < self.d2_size);
                    assert_eq!(v.len() as u64, self.d2_idx_size)
                }
                "addr3" => {
                    assert!(v.as_u64() < self.d3_size);
                    assert_eq!(v.len() as u64, self.d3_idx_size)
                }
                "clk" => assert_eq!(v.len(), 1),
                "reset" => assert_eq!(v.len(), 1),
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(&mut self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, Value)> {
        //unwrap the arguments
        //these come from the primitive definition in verilog
        //don't need to depend on the user -- just make sure this matches primitive + calyx + verilog defs
        let (_, input) =
            inputs.iter().find(|(id, _)| id == "write_data").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        let (_, addr2) = inputs.iter().find(|(id, _)| id == "addr2").unwrap();
        let (_, addr3) = inputs.iter().find(|(id, _)| id == "addr3").unwrap();

        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();
        let addr2 = addr2.as_u64();
        let addr3 = addr3.as_u64();
        self.last_idx = (addr0, addr1, addr2, addr3);

        let real_addr = self.calc_addr(addr0, addr1, addr2, addr3);
        if write_en.as_u64() == 1 {
            self.update = Some((real_addr, (*input).clone()));
            self.write_en = true;
        } else {
            self.update = None;
            self.write_en = false;
        }
        vec![(
            ir::Id::from("read_data"),
            if real_addr < self.max_idx() {
                self.data[real_addr as usize].clone()
            } else {
                Value::zeroes(self.width as usize)
            },
        )]
    }

    fn reset(&mut self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, Value)> {
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        let (_, addr2) = inputs.iter().find(|(id, _)| id == "addr2").unwrap();
        let (_, addr3) = inputs.iter().find(|(id, _)| id == "addr3").unwrap();
        //check that addr0 is not out of bounds and that it is the proper width!
        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();
        let addr2 = addr2.as_u64();
        let addr3 = addr3.as_u64();
        self.last_idx = (addr0, addr1, addr2, addr3);
        let real_addr = self.calc_addr(addr0, addr1, addr2, addr3);

        let old = self.data[real_addr as usize].clone();
        //clear update and write_en
        self.update = None;
        self.write_en = false;
        vec![
            (ir::Id::from("read_data"), old),
            (ir::Id::from("done"), Value::zeroes(1)),
        ]
    }

    fn serialize(&self) -> Serializeable {
        Serializeable::Array(
            self.data.iter().map(Value::as_u64).collect(),
            (
                self.d0_size as usize,
                self.d1_size as usize,
                self.d2_size as usize,
                self.d3_size as usize,
            )
                .into(),
        )
    }
}
