use super::{Primitive, Serializeable};
use crate::values::{PulseValue, TimeLockedValue, Value};
use calyx::ir;

fn get_param<S>(params: &ir::Binding, target: S) -> Option<u64>
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

/// A register.
#[derive(Default)]
pub struct StdReg {
    pub width: u64,
    pub data: [Value; 1],
    update: Option<Value>,
}

impl StdReg {
    pub fn new(params: ir::Binding) -> Self {
        let width = params
            .iter()
            .find(|(n, _)| n.as_ref() == "WIDTH")
            .expect("Missing `width` param from std_reg binding")
            .1;
        StdReg {
            width,
            data: [Value::new(width as usize)],
            update: None,
        }
    }
}

impl Primitive for StdReg {
    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(calyx::ir::Id, &Value)]) {
        todo!()
    }

    fn execute(
        &mut self,
        inputs: &[(calyx::ir::Id, &Value)],
        done_val: Option<&Value>,
    ) -> Vec<(calyx::ir::Id, crate::values::OutputValue)> {
        //unwrap the arguments
        let (_, input) = inputs.iter().find(|(id, _)| id == "in").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        //write the input to the register
        if write_en.as_u64() == 1 {
            self.update = Some((*input).clone());
            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new register data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("out"),
                    TimeLockedValue::new(
                        (*input).clone(),
                        1,
                        Some(self.data[0].clone()),
                    )
                    .into(),
                ),
                (
                    "done".into(),
                    PulseValue::new(
                        // XXX(rachit): Do we always expect done_val to exist
                        // here?
                        done_val.unwrap().clone(),
                        Value::bit_high(),
                        Value::bit_low(),
                        1,
                    )
                    .into(),
                ),
            ]
        } else {
            // if write_en was low, so done is 0 b/c nothing was written here
            // in this vector i
            // OUT: the old value in the register, b/c we couldn't write
            // DONE: not TimeLockedValue, b/c it's just 0, b/c our write was unsuccessful
            vec![(ir::Id::from("out"), self.data[0].clone().into())]
        }
    }

    fn reset(
        &mut self,
        _: &[(calyx::ir::Id, &Value)],
    ) -> Vec<(calyx::ir::Id, crate::values::OutputValue)> {
        vec![
            (ir::Id::from("out"), self.data[0].clone().into()),
            (ir::Id::from("done"), Value::zeroes(1).into()),
        ]
    }

    fn commit_updates(&mut self) {
        if let Some(val) = self.update.take() {
            self.data[0] = val;
        }
    }

    fn clear_update_buffer(&mut self) {
        self.update = None;
    }

    fn serialize(&self) -> Serializeable {
        Serializeable::Val(self.data[0].as_u64())
    }
}

#[derive(Default, Debug)]
pub struct StdConst {
    value: Value,
}

impl StdConst {
    pub fn new(params: calyx::ir::Binding) -> Self {
        let width = get_param(&params, "WIDTH")
            .expect("Missing width parameter from std_const binding");

        let init_value = get_param(&params, "VALUE")
            .expect("Missing `vale` param from std_const binding");

        let value = Value::try_from_init(init_value, width).unwrap();

        Self { value }
    }
}

impl Primitive for StdConst {
    fn is_comb(&self) -> bool {
        true
    }

    fn validate(&self, _inputs: &[(ir::Id, &Value)]) {}

    fn execute(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
        _done_val: Option<&Value>,
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
        vec![("out".into(), self.value.clone().into())]
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
        vec![("out".into(), self.value.clone().into())]
    }

    fn commit_updates(&mut self) {}

    fn clear_update_buffer(&mut self) {}
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
}

impl StdMemD1 {
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
                _ => {}
            }
        }
    }

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
        done_val: Option<&Value>,
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
        let (_, input) =
            inputs.iter().find(|(id, _)| id == "write_data").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();

        let addr0 = addr0.as_u64();
        let old = self.data[addr0 as usize].clone();

        if write_en.as_u64() == 1 {
            self.update = Some((addr0, (*input).clone()));

            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new mem data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("read_data"),
                    TimeLockedValue::new((*input).clone(), 1, Some(old)).into(),
                ),
                (
                    "done".into(),
                    PulseValue::new(
                        // TODO (griffin): Remove this done_val buisiness
                        // pending updates to primitive responsibilities
                        done_val.unwrap().clone(),
                        Value::bit_high(),
                        Value::bit_low(),
                        1,
                    )
                    .into(),
                ),
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
        //so we don't have to keep using .as_u64()
        let addr0 = addr0.as_u64();
        //check that input data is the appropriate width as well
        let old = self.data[addr0 as usize].clone();
        vec![
            ("read_data".into(), old.into()),
            (ir::Id::from("done"), Value::zeroes(1).into()),
        ]
    }

    fn commit_updates(&mut self) {
        if let Some((idx, val)) = self.update.take() {
            self.data[idx as usize] = val;
        }
    }

    fn clear_update_buffer(&mut self) {
        self.update = None;
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
