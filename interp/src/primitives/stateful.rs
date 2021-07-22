use super::{Primitive, Serializeable};
use crate::utils::construct_bindings;
use crate::values::{OutputValue, PulseValue, TimeLockedValue, Value};
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

//pipelined multiplication, but as of now takes one cycle
#[derive(Default)]
pub struct StdMultPipe {
    pub width: u64,
    pub product: Value,
    update: Option<Value>,
    // right now, trying to be 1 cycle, so no need for these
    // left: Value,
    // right: Value,
    //cycle_count: u64, //0, 1, 2
}

impl StdMultPipe {
    pub fn from_constants(width: u64) -> Self {
        StdMultPipe {
            width,
            product: Value::zeroes(width as usize),
            update: None,
            // left: Value::zeroes(width as usize),
            // right: Value::zeroes(width as usize),
            // cycle_count: 0,
        }
    }

    pub fn new(params: ir::Binding) -> Self {
        let width = params
            .iter()
            .find(|(n, _)| n.as_ref() == "WIDTH")
            .expect("Missing `WIDTH` param from std_mult_pipe binding")
            .1;
        Self::from_constants(width)
    }
}

impl Primitive for StdMultPipe {
    //null-op for now
    fn do_tick(&mut self) -> Vec<(ir::Id, OutputValue)> {
        todo!()
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

    /// Currently a 1 cycle std_mult_pipe that has a similar interface
    /// to a register.
    fn execute(
        &mut self,
        inputs: &[(calyx::ir::Id, &Value)],
    ) -> Vec<(calyx::ir::Id, crate::values::OutputValue)> {
        //unwrap the arguments, left, right, and go
        let (_, left) = inputs.iter().find(|(id, _)| id == "left").unwrap();
        let (_, right) = inputs.iter().find(|(id, _)| id == "right").unwrap();
        let (_, go) = inputs.iter().find(|(id, _)| id == "go").unwrap();
        //continue computation
        if go.as_u64() == 1 {
            //if [go] is high and
            //if left and right are the same as the interior left and right
            // if (left.as_u64() == self.left.as_u64())
            //     & (right.as_u64() == self.right.as_u64())
            // {
            //if self.cycle_count == 1 {
            //and cycle_count == 1, return the product, and write product as update
            self.update = Some(
                Value::from(left.as_u64() * right.as_u64(), self.width)
                    .unwrap(),
            );
            // //reset cycle_count
            // self.cycle_count = 0;
            //return
            return vec![
                (
                    ir::Id::from("out"),
                    TimeLockedValue::new(
                        (&Value::from(
                            left.as_u64() * right.as_u64(),
                            self.width,
                        )
                        .unwrap())
                            .clone(),
                        1,
                        //Some(Value::from(0, self.width).unwrap()),
                        Some(self.product.clone()),
                    )
                    .into(),
                ),
                // (
                //     "done".into(),
                //     PulseValue::new(
                //         done_val.unwrap().clone(),
                //         Value::bit_high(),
                //         Value::bit_low(),
                //         1,
                //     )
                //     .into(),
                // ),
            ];
            // } else {
            //     //else just increment cycle_count
            //     self.cycle_count += 1;
            //     // and return whatever was committed to [product]
            //     // not a TLV
            //     return vec![(
            //         ir::Id::from("out"),
            //         self.product.clone().into(),
            //     )];
            // }
            // } else {
            //     //else, left!=left and so on, restart (write these new left and right to interior left and right),
            //     //set cycle_count to 1
            //     self.cycle_count = 1;
            //     self.left = Value::clone(left);
            //     self.right = Value::clone(right);
            //     // and return whatever was committed to [product]
            //     return vec![(
            //         ir::Id::from("out"),
            //         self.product.clone().into(),
            //     )];
            // }
        } else {
            //if [go] is low, return whatever is in product
            //this is not guaranteed to be meaningful
            return vec![(
                ir::Id::from("out"),
                //     Value::from(0, self.width).unwrap().into(),
                // )];
                self.product.clone().into(),
            )];
        }
    }

    fn reset(
        &mut self,
        _: &[(calyx::ir::Id, &Value)],
    ) -> Vec<(calyx::ir::Id, crate::values::OutputValue)> {
        //if self.cycle_count == 2 {
        vec![
            (ir::Id::from("out"), self.product.clone().into()),
            (ir::Id::from("done"), Value::bit_low().into()),
        ]
        // } else {
        //     //this component hasn't computed, so it's all zeroed out
        //     vec![
        //         (ir::Id::from("out"), Value::zeroes(1).into()),
        //         (ir::Id::from("done"), Value::zeroes(1).into()),
        //     ]
        // }
    }

    fn serialize(&self) -> Serializeable {
        Serializeable::Array(
            //vec![self.left.clone(), self.right.clone(), self.product.clone()]
            vec![self.product.clone()]
                .iter()
                .map(Value::as_u64)
                .collect(),
            1.into(),
            //3.into(),
        )
    }
}

//pipelined division, but as of now takes one cycle
#[derive(Default)]
pub struct StdDivPipe {
    pub width: u64,
    pub quotient: Value,
    pub remainder: Value,
    update_quotient: Option<Value>,
    update_remainder: Option<Value>,
    // right now, trying to be 1 cycle, so no need for these
    // left: Value,
    // right: Value,
    //cycle_count: u64, //0, 1, 2
}

impl StdDivPipe {
    pub fn from_constants(width: u64) -> Self {
        StdDivPipe {
            width,
            quotient: Value::zeroes(width as usize),
            remainder: Value::zeroes(width as usize),
            update_quotient: None,
            update_remainder: None,
            // left: Value::zeroes(width as usize),
            // right: Value::zeroes(width as usize),
            // cycle_count: 0,
        }
    }

    pub fn new(params: ir::Binding) -> Self {
        let width = params
            .iter()
            .find(|(n, _)| n.as_ref() == "WIDTH")
            .expect("Missing `WIDTH` param from std_mult_pipe binding")
            .1;
        Self::from_constants(width)
    }
}

impl Primitive for StdDivPipe {
    //null-op for now
    fn do_tick(&mut self) -> Vec<(ir::Id, OutputValue)> {
        todo!()
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

    /// Currently a 1 cycle std_div_pipe that has a similar interface
    /// to a register.
    fn execute(
        &mut self,
        inputs: &[(calyx::ir::Id, &Value)],
    ) -> Vec<(calyx::ir::Id, crate::values::OutputValue)> {
        //unwrap the arguments, left, right, and go
        let (_, left) = inputs.iter().find(|(id, _)| id == "left").unwrap();
        let (_, right) = inputs.iter().find(|(id, _)| id == "right").unwrap();
        let (_, go) = inputs.iter().find(|(id, _)| id == "go").unwrap();
        //continue computation
        if go.as_u64() == 1 {
            self.update_quotient = Some(
                Value::from(left.as_u64() / right.as_u64(), self.width)
                    .unwrap(),
            );
            self.update_remainder = Some(
                Value::from(left.as_u64() % right.as_u64(), self.width)
                    .unwrap(),
            );
            return vec![
                (
                    ir::Id::from("out_quotient"),
                    TimeLockedValue::new(
                        (&Value::from(
                            left.as_u64() / right.as_u64(),
                            self.width,
                        )
                        .unwrap())
                            .clone(),
                        1,
                        //Some(Value::from(0, self.width).unwrap()),
                        Some(self.quotient.clone()),
                    )
                    .into(),
                ),
                (
                    ir::Id::from("out_remainder"),
                    TimeLockedValue::new(
                        (&Value::from(
                            left.as_u64() % right.as_u64(),
                            self.width,
                        )
                        .unwrap())
                            .clone(),
                        1,
                        //Some(Value::from(0, self.width).unwrap()),
                        Some(self.remainder.clone()),
                    )
                    .into(),
                ),
                // (
                //     "done".into(),
                //     PulseValue::new(
                //         done_val.unwrap().clone(),
                //         Value::bit_high(),
                //         Value::bit_low(),
                //         1,
                //     )
                //     .into(),
                // ),
            ];
        } else {
            //if [go] is low, return whatever is in product
            //this is not guaranteed to be meaningful
            return vec![
                (
                    ir::Id::from("out_quotient"),
                    //     Value::from(0, self.width).unwrap().into(),
                    // )];
                    self.quotient.clone().into(),
                ),
                (
                    ir::Id::from("out_remainder"),
                    //     Value::from(0, self.width).unwrap().into(),
                    // )];
                    self.remainder.clone().into(),
                ),
            ];
        }
    }

    fn reset(
        &mut self,
        _: &[(calyx::ir::Id, &Value)],
    ) -> Vec<(calyx::ir::Id, crate::values::OutputValue)> {
        //if self.cycle_count == 2 {
        vec![
            (ir::Id::from("out_quotient"), self.quotient.clone().into()),
            (ir::Id::from("out_remainder"), self.remainder.clone().into()),
            (ir::Id::from("done"), Value::bit_low().into()),
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
    //does it need a cycle count?
    //yes. execute will set cycle count to 1,
    //do_tick() will set it to 0. if you call
    //do_tick() while cycle count is 0, no done signal will be emitted
    cycle_count: u64,
}

impl StdReg {
    pub fn from_constants(width: u64) -> Self {
        StdReg {
            width,
            data: [Value::new(width as usize)],
            update: None,
            cycle_count: 0,
        }
    }

    pub fn new(params: ir::Binding) -> Self {
        let width = params
            .iter()
            .find(|(n, _)| n.as_ref() == "WIDTH")
            .expect("Missing `WIDTH` param from std_reg binding")
            .1;
        Self::from_constants(width)
    }
}

impl Primitive for StdReg {
    fn do_tick(&mut self) -> Vec<(ir::Id, OutputValue)> {
        //first commit any updates
        //is there a point in only putting this in cycle_count == 1?
        //idt it's possible for there to be an update that wasn't read right
        //after the execute call.
        if let Some(val) = self.update.take() {
            self.data[0] = val;
        }
        //then, based on cycle count, return
        if self.cycle_count == 1 {
            self.cycle_count = 0; //we are done for this cycle
                                  //if do_tick() is called again w/o an execute() preceeding it,
                                  //then done will be low.
            vec![
                (ir::Id::from("out"), self.data[0].clone().into()),
                (ir::Id::from("done"), Value::bit_high().into()),
            ]
        } else if self.cycle_count == 0 {
            //done is low, but there is still data in this reg to return
            vec![
                (ir::Id::from("out"), self.data[0].clone().into()),
                //(ir::Id::from("done"), Value::bit_low().into()),
                //not sure if we shld return low done, or just not specify done?
                //NOTE: goes in an infinite loop if we return a low done; why?
            ]
        } else {
            panic!("StdReg's cycle_count is not 0 or 1!: {}", self.cycle_count);
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
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(
        &mut self,
        inputs: &[(calyx::ir::Id, &Value)],
    ) -> Vec<(calyx::ir::Id, crate::values::OutputValue)> {
        //unwrap the arguments
        let (_, input) = inputs.iter().find(|(id, _)| id == "in").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        //write the input to the register
        if write_en.as_u64() == 1 {
            self.update = Some((*input).clone());
            //put cycle_count as 1! B/c do_tick() should return a high done
            self.cycle_count = 1;
        }
        //if write_en wasn't high, cycle_count shouldn't be set 1, b/c register shouldn't emit
        //a high done from a low write_en. But, cycle_count shouldn't explicitly be set to 0; what if execute() is called
        //multiple times in a cycle, j/ w write_en on only once? Then in the next cycle
        //the register should still emit a value + high done, as it just ignored the write_en off
        //executes.
        vec![]
    }

    fn reset(
        &mut self,
        _: &[(calyx::ir::Id, &Value)],
    ) -> Vec<(calyx::ir::Id, crate::values::OutputValue)> {
        self.update = None;
        self.cycle_count = 0; //might be redundant, not too sure when reset is used
        vec![
            (ir::Id::from("out"), self.data[0].clone().into()),
            (ir::Id::from("done"), Value::zeroes(1).into()),
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
    cycle_count: u64,
    //NOTE: in old implementation, execute w/ write_en low would just return
    //whatever was at [addr0]. So [last_idx] should just follow the u64 in
    //[update], because we need to have something to return even if the update
    //was taken (for instance, last cycle)
    last_idx: u64,
}

impl StdMemD1 {
    pub fn from_constants(width: u64, size: u64, idx_size: u64) -> Self {
        let bindings = construct_bindings(
            [("WIDTH", width), ("SIZE", size), ("IDX_SIZE", idx_size)].iter(),
        );
        Self::new(bindings)
    }
    /// Instantiates a new StdMemD1 storing data of width [width], containing [size]
    /// slots for memory, accepting indecies (addr0) of width [idx_size].
    /// Note: if [idx_size] is smaller than the length of [size]'s binary representation,
    /// you will not be able to access the slots near the end of the memory.
    pub fn new(params: ir::Binding) -> StdMemD1 {
        let width = get_param(&params, "WIDTH")
            .expect("Missing width param for std_mem_d1");
        let size = get_param(&params, "SIZE")
            .expect("Missing size param for std_mem_d1");
        let idx_size = get_param(&params, "IDX_SIZE")
            .expect("Missing idx_size param for std_mem_d1");

        let data = vec![Value::zeroes(width as usize); size as usize];
        StdMemD1 {
            width,
            size,     //how many slots of memory in the vector
            idx_size, //the width of the values used to address the memory
            data,
            update: None,
            cycle_count: 0,
            last_idx: 0,
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
    fn do_tick(&mut self) -> Vec<(ir::Id, OutputValue)> {
        //get the index from the update, which is only filled
        //if the cycle_count is 1. Then if cycle_count is 1 (panic if else),
        //return the vector.
        if let Some((idx, val)) = self.update.take() {
            //assert_eq!(idx, self.last_idx); //the most recent [execute] sets both update and last_idx
            self.data[idx as usize] = val;
            if self.cycle_count == 1 {
                self.cycle_count = 0;
                vec![
                    (
                        ir::Id::from("read_data"),
                        self.data[idx as usize].clone().into(),
                    ),
                    (ir::Id::from("done"), Value::bit_high().into()),
                ]
            } else {
                panic!("std_mem_d1 had an update, and cycle_count was not 1")
            }
        } else if self.cycle_count == 0 {
            //done is low, but there is still data in this mem to return, the last_idx
            //this really may not be meaningful -- last data output
            vec![(
                ir::Id::from("read_data"),
                self.data[self.last_idx as usize].clone().into(),
            )]
        } else {
            panic!(
                "std_mem_d1 has no update but cycle_count is not 0: {}",
                self.cycle_count
            );
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
                    assert_eq!(v.len() as u64, self.idx_size)
                }
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
        let (_, input) =
            inputs.iter().find(|(id, _)| id == "write_data").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();

        let addr0 = addr0.as_u64();
        //no matter what, we need to remember the most recent index
        //requested
        self.last_idx = addr0;
        if write_en.as_u64() == 1 {
            self.update = Some((addr0, (*input).clone()));
            self.cycle_count = 1;
        }
        vec![]
    }

    fn reset(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        //so we don't have to keep using .as_u64()
        let addr0 = addr0.as_u64();
        //check that input data is the appropriate width as well
        let old = self.data[addr0 as usize].clone();
        vec![
            ("read_data".into(), old.into()),
            (ir::Id::from("done"), Value::zeroes(1).into()),
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
        Self::new(bindings)
    }

    /// Instantiates a new StdMemD2 storing data of width [width], containing
    /// [d0_size] * [d1_size] slots for memory, accepting indecies [addr0][addr1] of widths
    /// [d0_idx_size] and [d1_idx_size] respectively.
    /// Initially the memory is filled with all 0s.
    pub fn new(params: ir::Binding) -> StdMemD2 {
        let width = get_param(&params, "WIDTH")
            .expect("Missing width parameter for std_mem_d2");
        let d0_size = get_param(&params, "D0_SIZE")
            .expect("Missing d0_size parameter for std_mem_d2");
        let d1_size = get_param(&params, "D1_SIZE")
            .expect("Missing d1_size parameter for std_mem_d2");
        let d0_idx_size = get_param(&params, "D0_IDX_SIZE")
            .expect("Missing d0_idx_size parameter for std_mem_d2");
        let d1_idx_size = get_param(&params, "D1_IDX_SIZE")
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
    fn do_tick(&mut self) -> Vec<(ir::Id, OutputValue)> {
        todo!()
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
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
        //unwrap the arguments
        //these come from the primitive definition in verilog
        //don't need to depend on the user -- just make sure this matches primitive + calyx + verilog defs
        let (_, input) =
            inputs.iter().find(|(id, _)| id == "write_data").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        //chec that write_data is the exact right width
        //check that addr0 is not out of bounds and that it is the proper width!
        let addr0 = addr0.as_u64();

        //chech that addr1 is not out of bounds and that it is the proper iwdth
        let addr1 = addr1.as_u64();
        let real_addr = self.calc_addr(addr0, addr1);

        let old = self.data[real_addr as usize].clone(); //not sure if this could lead to errors (Some(old)) is borrow?
                                                         // only write to memory if write_en is 1
        if write_en.as_u64() == 1 {
            self.update =
                Some((self.calc_addr(addr0, addr1), (*input).clone()));
            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new mem data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("read_data"),
                    TimeLockedValue::new((*input).clone(), 1, Some(old)).into(),
                ),
                // (
                //     "done".into(),
                //     PulseValue::new(
                //         // TODO (griffin): FIXME
                //         done_val.unwrap().clone(),
                //         Value::bit_high(),
                //         Value::bit_low(),
                //         1,
                //     )
                //     .into(),
                // ),
            ]
        } else {
            // if write_en was low, so done is 0 b/c nothing was written here
            // in this vector i
            // READ_DATA: (immediate), just the old value b/c write was unsuccessful
            // DONE: not TimeLockedValue, b/c it's just 0, b/c our write was unsuccessful
            vec![(ir::Id::from("read_data"), old.into())]
        }
    }

    fn reset(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();

        let real_addr = self.calc_addr(addr0, addr1);

        let old = self.data[real_addr as usize].clone();

        vec![
            (ir::Id::from("read_data"), old.into()),
            (ir::Id::from("done"), Value::zeroes(1).into()),
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
        Self::new(bindings)
    }
    /// Instantiates a new StdMemD3 storing data of width [width], containing
    /// [d0_size] * [d1_size] * [d2_size] slots for memory, accepting indecies [addr0][addr1][addr2] of widths
    /// [d0_idx_size], [d1_idx_size], and [d2_idx_size] respectively.
    /// Initially the memory is filled with all 0s.
    pub fn new(params: ir::Binding) -> StdMemD3 {
        let width = get_param(&params, "WIDTH")
            .expect("Missing width parameter for std_mem_d3");
        let d0_size = get_param(&params, "D0_SIZE")
            .expect("Missing d0_size parameter for std_mem_d3");
        let d1_size = get_param(&params, "D1_SIZE")
            .expect("Missing d1_size parameter for std_mem_d3");
        let d2_size = get_param(&params, "D2_SIZE")
            .expect("Missing d2_size parameter for std_mem_d3");
        let d0_idx_size = get_param(&params, "D0_IDX_SIZE")
            .expect("Missing d0_idx_size parameter for std_mem_d3");
        let d1_idx_size = get_param(&params, "D1_IDX_SIZE")
            .expect("Missing d1_idx_size parameter for std_mem_d3");
        let d2_idx_size = get_param(&params, "D2_IDX_SIZE")
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
    fn calc_addr(&self, addr0: u64, addr1: u64, addr2: u64) -> u64 {
        self.d2_size * (addr0 * self.d1_size + addr1) + addr2
    }
}

impl Primitive for StdMemD3 {
    //null-op for now
    fn do_tick(&mut self) -> Vec<(ir::Id, OutputValue)> {
        todo!()
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
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
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

        let real_addr = self.calc_addr(addr0, addr1, addr2);

        let old = self.data[real_addr as usize].clone();
        //not sure if this could lead to errors (Some(old)) is borrow?
        // only write to memory if write_en is 1
        if write_en.as_u64() == 1 {
            self.update =
                Some((self.calc_addr(addr0, addr1, addr2), (*input).clone()));

            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new mem data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("read_data"),
                    TimeLockedValue::new((*input).clone(), 1, Some(old)).into(),
                ),
                // (
                //     "done".into(),
                //     PulseValue::new(
                //         // TODO (griffin): FIXME
                //         done_val.unwrap().clone(),
                //         Value::bit_high(),
                //         Value::bit_low(),
                //         1,
                //     )
                //     .into(),
                // ),
            ]
        } else {
            // if write_en was low, so done is 0 b/c nothing was written here
            // in this vector i
            // READ_DATA: (immediate), just the old value b/c write was unsuccessful
            // DONE: not TimeLockedValue, b/c it's just 0, b/c our write was unsuccessful
            vec![(ir::Id::from("read_data"), old.into())]
        }
    }

    fn reset(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        let (_, addr2) = inputs.iter().find(|(id, _)| id == "addr2").unwrap();
        //check that addr0 is not out of bounds and that it is the proper width!
        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();
        let addr2 = addr2.as_u64();

        let real_addr = self.calc_addr(addr0, addr1, addr2);

        let old = self.data[real_addr as usize].clone();
        vec![
            (ir::Id::from("read_data"), old.into()),
            (ir::Id::from("done"), Value::zeroes(1).into()),
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
        Self::new(bindings)
    }
    // Instantiates a new StdMemD3 storing data of width [width], containing
    /// [d0_size] * [d1_size] * [d2_size] * [d3_size] slots for memory, accepting indecies [addr0][addr1][addr2][addr3] of widths
    /// [d0_idx_size], [d1_idx_size], [d2_idx_size] and [d3_idx_size] respectively.
    /// Initially the memory is filled with all 0s.
    pub fn new(params: ir::Binding) -> StdMemD4 {
        // yes this was incredibly tedious to write. Why do you ask?
        let width = get_param(&params, "WIDTH")
            .expect("Missing width parameter for std_mem_d4");
        let d0_size = get_param(&params, "D0_SIZE")
            .expect("Missing d0_size parameter for std_mem_d4");
        let d1_size = get_param(&params, "D1_SIZE")
            .expect("Missing d1_size parameter for std_mem_d4");
        let d2_size = get_param(&params, "D2_SIZE")
            .expect("Missing d2_size parameter for std_mem_d4");
        let d3_size = get_param(&params, "D3_SIZE")
            .expect("Missing d3_size parameter for std_mem_d4");
        let d0_idx_size = get_param(&params, "D0_IDX_SIZE")
            .expect("Missing d0_idx_size parameter for std_mem_d4");
        let d1_idx_size = get_param(&params, "D1_IDX_SIZE")
            .expect("Missing d1_idx_size parameter for std_mem_d4");
        let d2_idx_size = get_param(&params, "D2_IDX_SIZE")
            .expect("Missing d2_idx_size parameter for std_mem_d4");
        let d3_idx_size = get_param(&params, "D3_IDX_SIZE")
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
}
impl Primitive for StdMemD4 {
    //null-op for now
    fn do_tick(&mut self) -> Vec<(ir::Id, OutputValue)> {
        todo!()
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
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
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

        let real_addr = self.calc_addr(addr0, addr1, addr2, addr3);

        let old = self.data[real_addr as usize].clone(); //not sure if this could lead to errors (Some(old)) is borrow?
                                                         // only write to memory if write_en is 1
        if write_en.as_u64() == 1 {
            self.update = Some((
                self.calc_addr(addr0, addr1, addr2, addr3),
                (*input).clone(),
            ));

            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new mem data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("read_data"),
                    TimeLockedValue::new((*input).clone(), 1, Some(old)).into(),
                ),
                // (
                //     "done".into(),
                //     PulseValue::new(
                //         // TODO (griffin): FIXME
                //         done_val.unwrap().clone(),
                //         Value::bit_high(),
                //         Value::bit_low(),
                //         1,
                //     )
                //     .into(),
                // ),
            ]
        } else {
            // if write_en was low, so done is 0 b/c nothing was written here
            // in this vector i
            // READ_DATA: (immediate), just the old value b/c write was unsuccessful
            // DONE: not TimeLockedValue, b/c it's just 0, b/c our write was unsuccessful
            vec![(ir::Id::from("read_data"), old.into())]
        }
    }

    fn reset(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        let (_, addr2) = inputs.iter().find(|(id, _)| id == "addr2").unwrap();
        let (_, addr3) = inputs.iter().find(|(id, _)| id == "addr3").unwrap();
        //check that addr0 is not out of bounds and that it is the proper width!
        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();
        let addr2 = addr2.as_u64();
        let addr3 = addr3.as_u64();
        let real_addr = self.calc_addr(addr0, addr1, addr2, addr3);

        let old = self.data[real_addr as usize].clone();

        vec![
            (ir::Id::from("read_data"), old.into()),
            (ir::Id::from("done"), Value::zeroes(1).into()),
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
