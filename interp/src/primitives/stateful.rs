use super::prim_utils::{get_inputs, get_param, get_params, ShiftBuffer};
use super::primitive::Named;
use super::{Entry, Primitive, Serializable};
use crate::errors::{InterpreterError, InterpreterResult};
use crate::logging::{self, warn};
use crate::utils::{construct_bindings, PrintCode};
use crate::validate;
use crate::values::Value;
use calyx::ir;
use ibig::ops::RemEuclid;
use ibig::{ibig, ubig, IBig, UBig};

const DECIMAL_PRINT_WIDTH: usize = 7;

enum BinOpUpdate {
    None,
    Reset,
    Value(Value, Value),
}

impl BinOpUpdate {
    fn clear(&mut self) {
        *self = BinOpUpdate::None;
    }
    fn take(&mut self) -> Self {
        std::mem::replace(&mut *self, BinOpUpdate::None)
    }
}

/// Pipelined Multiplication (3 cycles)
/// How to use:
/// [Primitive::execute] with the desired bindings.
/// To capture these bindings into the internal (out) queue, [Primitive::do_tick].
/// The product associated with a given input will be output on the third [Primitive::do_tick()].
/// Note: Calling [Primitive::execute] multiple times before [Primitive::do_tick] has no effect; only the last
/// set of inputs prior to the [Primitive::do_tick] will be saved.
pub struct StdMultPipe<const SIGNED: bool, const DEPTH: usize> {
    width: u64,
    product: Value,
    update: BinOpUpdate,
    queue: ShiftBuffer<Value, DEPTH>,
    full_name: ir::Id,
    logger: logging::Logger,
    error_on_overflow: bool,
}

impl<const SIGNED: bool, const DEPTH: usize> StdMultPipe<SIGNED, DEPTH> {
    pub fn from_constants(
        width: u64,
        name: ir::Id,
        error_on_overflow: bool,
    ) -> Self {
        StdMultPipe {
            width,
            product: Value::zeroes(width as usize),
            update: BinOpUpdate::None,
            queue: ShiftBuffer::default(),
            logger: logging::new_sublogger(&name),
            full_name: name,
            error_on_overflow,
        }
    }

    pub fn new(
        params: &ir::Binding,
        name: ir::Id,
        error_on_overflow: bool,
    ) -> Self {
        let width = get_param(params, "WIDTH")
            .expect("Missing `WIDTH` param from std_mult_pipe binding");
        Self::from_constants(width, name, error_on_overflow)
    }
}

impl<const SIGNED: bool, const DEPTH: usize> Named
    for StdMultPipe<SIGNED, DEPTH>
{
    fn get_full_name(&self) -> &ir::Id {
        &self.full_name
    }
}

impl<const SIGNED: bool, const DEPTH: usize> Primitive
    for StdMultPipe<SIGNED, DEPTH>
{
    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        let out = match self.update.take() {
            BinOpUpdate::None => {
                self.queue.reset();
                vec![
                    (ir::Id::from("out"), self.product.clone()),
                    (ir::Id::from("done"), Value::bit_low()),
                ]
            }
            BinOpUpdate::Reset => {
                self.queue.reset();
                vec![
                    (ir::Id::from("out"), Value::zeroes(self.width)),
                    (ir::Id::from("done"), Value::bit_low()),
                ]
            }
            BinOpUpdate::Value(left, right) => {
                let (value, overflow) = if SIGNED {
                    Value::from_checked(
                        left.as_signed() * right.as_signed(),
                        self.width,
                    )
                } else {
                    Value::from_checked(
                        left.as_unsigned() * right.as_unsigned(),
                        self.width,
                    )
                };

                if overflow & self.error_on_overflow {
                    return Err(InterpreterError::OverflowError());
                } else if overflow {
                    warn!(
                        self.logger,
                        "Computation under/overflowed ({} -> {})",
                        if SIGNED {
                            format!("{}", left.as_signed() * right.as_signed())
                        } else {
                            format!(
                                "{}",
                                left.as_unsigned() * right.as_unsigned()
                            )
                        },
                        if SIGNED {
                            format!("{}", value.as_signed())
                        } else {
                            format!("{}", value.as_unsigned())
                        }
                    );
                }

                if let Some(value) = self.queue.shift(Some(value)) {
                    self.product = value;
                    vec![
                        (ir::Id::from("out"), self.product.clone()),
                        (ir::Id::from("done"), Value::bit_high()),
                    ]
                } else {
                    vec![
                        (ir::Id::from("out"), self.product.clone()),
                        (ir::Id::from("done"), Value::bit_low()),
                    ]
                }
            }
        };

        Ok(out)
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
                "reset" => assert_eq!(v.len() as u64, 1),
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(
        &mut self,
        inputs: &[(calyx::ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        get_inputs![inputs;
            left: "left",
            right: "right",
            reset: "reset",
            go: "go"
        ];

        self.update = if reset.as_bool() {
            BinOpUpdate::Reset
        } else if go.as_bool() {
            BinOpUpdate::Value(left.clone(), right.clone())
        } else {
            BinOpUpdate::None
        };

        Ok(vec![])
    }

    fn reset(
        &mut self,
        _: &[(calyx::ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        self.update.clear();
        self.queue.reset();
        Ok(vec![
            (ir::Id::from("out"), self.product.clone()),
            (ir::Id::from("done"), Value::bit_low()),
        ])
    }

    fn serialize(&self, signed: Option<PrintCode>) -> Serializable {
        let code = signed.unwrap_or_default();
        Serializable::Array(
            vec![self.product.clone()]
                .iter()
                .map(|x| Entry::from_val_code(x, &code))
                .collect(),
            1.into(),
        )
    }
}

///Pipelined Division (3 cycles)
///Still bounded by u64.
///How to use:
///[Primitive::execute] with the desired bindings. To capture these bindings
///into the internal (out_quotient, out_remainder) queue, [Primitive::do_tick].
///The out_quotient and out_remainder associated with a given input will
///be output on the third [Primitive::do_tick].
///Note: Calling [Primitive::execute] multiple times before [Primitive::do_tick] has no effect; only
///the last set of inputs prior to the [Primitive::do_tick] will be saved.
pub struct StdDivPipe<const SIGNED: bool> {
    pub width: u64,
    pub quotient: Value,
    pub remainder: Value,
    update: BinOpUpdate, //first is left, second is right
    queue: ShiftBuffer<(Value, Value), 2>, //invariant: always length 2
    full_name: ir::Id,
    logger: logging::Logger,
    error_on_overflow: bool,
}

impl<const SIGNED: bool> StdDivPipe<SIGNED> {
    pub fn from_constants(
        width: u64,
        name: ir::Id,
        error_on_overflow: bool,
    ) -> Self {
        StdDivPipe {
            width,
            quotient: Value::zeroes(width as usize),
            remainder: Value::zeroes(width as usize),
            update: BinOpUpdate::None,
            queue: ShiftBuffer::default(),
            logger: logging::new_sublogger(&name),
            full_name: name,
            error_on_overflow,
        }
    }

    pub fn new(
        params: &ir::Binding,
        name: ir::Id,
        error_on_overflow: bool,
    ) -> Self {
        let width = params
            .iter()
            .find(|(n, _)| n.as_ref() == "WIDTH")
            .expect("Missing `WIDTH` param from std_mult_pipe binding")
            .1;
        Self::from_constants(width, name, error_on_overflow)
    }
}

impl<const SIGNED: bool> Named for StdDivPipe<SIGNED> {
    fn get_full_name(&self) -> &ir::Id {
        &self.full_name
    }
}

impl<const SIGNED: bool> Primitive for StdDivPipe<SIGNED> {
    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        let out = match self.update.take() {
            BinOpUpdate::None => {
                self.queue.reset();
                vec![
                    (ir::Id::from("out_quotient"), self.quotient.clone()),
                    (ir::Id::from("out_remainder"), self.remainder.clone()),
                    (ir::Id::from("done"), Value::bit_low()),
                ]
            }
            BinOpUpdate::Reset => {
                self.queue.reset();
                vec![
                    (ir::Id::from("out_quotient"), Value::zeroes(self.width)),
                    (ir::Id::from("out_remainder"), Value::zeroes(self.width)),
                    (ir::Id::from("done"), Value::bit_low()),
                ]
            }
            BinOpUpdate::Value(left, right) => {
                let (q, r) = if right.as_unsigned() != 0_u32.into() {
                    let (q, overflow) = if SIGNED {
                        Value::from_checked(
                            left.as_signed() / right.as_signed(),
                            self.width,
                        )
                    } else {
                        Value::from_checked(
                            left.as_unsigned() / right.as_unsigned(),
                            self.width,
                        )
                    };
                    let r = if SIGNED {
                        Value::from(
                            left.as_signed()
                                - right.as_signed()
                                    * floored_division(
                                        &left.as_signed(),
                                        &right.as_signed(),
                                    ),
                            self.width,
                        )
                    } else {
                        Value::from(
                            left.as_unsigned().rem_euclid(right.as_unsigned()),
                            self.width,
                        )
                    };

                    // the only way this is possible is if the division is signed and the
                    // min_val is divided by negative one as the resultant postitive value will
                    // not be representable in the desired bit width
                    if (overflow) & self.error_on_overflow {
                        return Err(InterpreterError::OverflowError());
                    } else if overflow {
                        warn!(
                            self.logger,
                            "Computation underflow ({} -> {})",
                            left.as_signed() / right.as_signed(),
                            q.as_signed()
                        )
                    }
                    (q, r)
                } else {
                    warn!(self.logger, "Division by zero");
                    (Value::zeroes(self.width), Value::zeroes(self.width))
                };

                if let Some((q, r)) = self.queue.shift(Some((q, r))) {
                    self.quotient = q.clone();
                    self.remainder = r.clone();
                    vec![
                        (ir::Id::from("out_quotient"), q),
                        (ir::Id::from("out_remainder"), r),
                        (ir::Id::from("done"), Value::bit_high()),
                    ]
                } else {
                    vec![
                        (ir::Id::from("out_quotient"), self.quotient.clone()),
                        (ir::Id::from("out_remainder"), self.remainder.clone()),
                        (ir::Id::from("done"), Value::bit_low()),
                    ]
                }
            }
        };

        Ok(out)
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
                "reset" => assert_eq!(v.len() as u64, 1),
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(
        &mut self,
        inputs: &[(calyx::ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        get_inputs![inputs;
            left: "left",
            right: "right",
            reset: "reset",
            go: "go"
        ];

        self.update = if reset.as_bool() {
            BinOpUpdate::Reset
        } else if go.as_bool() {
            BinOpUpdate::Value(left.clone(), right.clone())
        } else {
            BinOpUpdate::None
        };

        Ok(vec![])
    }

    fn reset(
        &mut self,
        _: &[(calyx::ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        self.update.clear();
        self.queue.reset();
        Ok(vec![
            (ir::Id::from("out_quotient"), self.quotient.clone()),
            (ir::Id::from("out_remainder"), self.remainder.clone()),
            (ir::Id::from("done"), Value::bit_low()),
        ])
    }

    fn serialize(&self, signed: Option<PrintCode>) -> Serializable {
        let code = signed.unwrap_or_default();
        Serializable::Array(
            //vec![self.left.clone(), self.right.clone(), self.product.clone()]
            vec![self.quotient.clone(), self.remainder.clone()]
                .iter()
                .map(|x| Entry::from_val_code(x, &code))
                .collect(),
            2.into(),
        )
    }
}

enum RegUpdate {
    None,
    Reset,
    Value(Value),
}

impl RegUpdate {
    fn clear(&mut self) {
        *self = Self::None;
    }

    fn take(&mut self) -> Self {
        std::mem::replace(self, Self::None)
    }
}

/// A register.
pub struct StdReg {
    pub width: u64,
    pub data: [Value; 1],
    update: RegUpdate,
    full_name: ir::Id,
}

impl StdReg {
    pub fn from_constants(width: u64, full_name: ir::Id) -> Self {
        StdReg {
            width,
            data: [Value::new(width as usize)],
            update: RegUpdate::None,
            full_name,
        }
    }

    pub fn new(params: &ir::Binding, name: ir::Id) -> Self {
        let width = params
            .iter()
            .find(|(n, _)| n.as_ref() == "WIDTH")
            .expect("Missing `WIDTH` param from std_reg binding")
            .1;
        Self::from_constants(width, name)
    }
}

impl Named for StdReg {
    fn get_full_name(&self) -> &ir::Id {
        &self.full_name
    }
}

impl Primitive for StdReg {
    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        let out = match self.update.take() {
            RegUpdate::None => vec![
                (ir::Id::from("out"), self.data[0].clone()),
                (ir::Id::from("done"), Value::bit_low()),
            ],
            RegUpdate::Reset => {
                self.data[0] = Value::zeroes(self.width);
                vec![
                    (ir::Id::from("out"), self.data[0].clone()),
                    (ir::Id::from("done"), Value::bit_low()),
                ]
            }
            RegUpdate::Value(v) => {
                self.data[0] = v;
                vec![
                    (ir::Id::from("out"), self.data[0].clone()),
                    (ir::Id::from("done"), Value::bit_high()),
                ]
            }
        };

        Ok(out)
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
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        get_inputs![inputs;
            input: "in",
            write_en: "write_en",
            reset: "reset"
        ];

        self.update = if reset.as_bool() {
            RegUpdate::Reset
        } else if write_en.as_bool() {
            RegUpdate::Value(input.clone())
        } else {
            RegUpdate::None
        };

        Ok(vec![])
    }

    fn reset(
        &mut self,
        _: &[(calyx::ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        self.update.clear();
        Ok(vec![
            (ir::Id::from("out"), self.data[0].clone()),
            (ir::Id::from("done"), Value::bit_low()),
        ])
    }

    fn serialize(&self, signed: Option<PrintCode>) -> Serializable {
        let code = signed.unwrap_or_default();
        Serializable::Val(Entry::from_val_code(&self.data[0], &code))
    }
}

/// A one-dimensional memory. Initialized with
/// StdMemD1.new(WIDTH, SIZE, IDX_SIZE) where:
/// * WIDTH - Size of an individual memory slot.
/// * SIZE - Number of slots in the memory.
/// * IDX_SIZE - The width of the index given to the memory.
///
/// To write to a memory, the `write_en` must be high.
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
    full_name: ir::Id,
    allow_invalid_memory_access: bool,
}

impl StdMemD1 {
    pub fn from_constants(
        width: u64,
        size: u64,
        idx_size: u64,
        full_name: ir::Id,
    ) -> Self {
        let bindings = construct_bindings(
            [("WIDTH", width), ("SIZE", size), ("IDX_SIZE", idx_size)].iter(),
        );
        Self::new(&bindings, full_name, false)
    }
    /// Instantiates a new StdMemD1 storing data of width `width`, containing
    /// `size` slots for memory, accepting indices (addr0) of width `idx_size`.
    /// Note: if `idx_size` is smaller than the length of `size`'s binary
    /// representation, you will not be able to access the slots near the end of
    /// the memory.
    pub fn new(
        params: &ir::Binding,
        name: ir::Id,
        allow_invalid_memory_access: bool,
    ) -> StdMemD1 {
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
            full_name: name,
            allow_invalid_memory_access,
        }
    }

    pub fn initialize_memory(
        &mut self,
        vals: &[Value],
    ) -> InterpreterResult<()> {
        if self.size as usize != vals.len() {
            return Err(InterpreterError::IncorrectMemorySize {
                mem_dim: "1D".into(),
                expected: self.size,
                given: vals.len(),
            });
        }

        for (idx, val) in vals.iter().enumerate() {
            self.data[idx] = val.truncate(self.width.try_into().unwrap())
        }

        Ok(())
    }
}

impl Named for StdMemD1 {
    fn get_full_name(&self) -> &ir::Id {
        &self.full_name
    }
}

impl Primitive for StdMemD1 {
    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        //if there is an update, update and return along w/ a done
        //else this memory was used combinationally and there is nothing to tick
        if self.last_index >= self.size {
            if self.allow_invalid_memory_access {
                if self.write_en {
                    return Ok(vec![
                        (ir::Id::from("read_data"), Value::zeroes(self.width)),
                        (ir::Id::from("done"), Value::bit_high()),
                    ]);
                } else {
                    return Ok(vec![
                        (ir::Id::from("done"), Value::bit_low()),
                        (ir::Id::from("read_data"), Value::zeroes(self.width)),
                    ]);
                }
            }

            return Err(InterpreterError::InvalidMemoryAccess {
                name: self.full_name.clone(),
                access: vec![self.last_index],
                dims: vec![self.size],
            });
        }

        let out = if self.write_en {
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
                unreachable!();
            }
        } else {
            vec![(ir::Id::from("done"), Value::bit_low())]
        };

        Ok(out)
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

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        get_inputs![inputs;
            input: "write_data",
            write_en: "write_en",
            addr0: "addr0"
        ];
        let addr0 = addr0.as_u64();
        self.last_index = addr0;
        if write_en.as_bool() {
            self.update = Some((addr0, (*input).clone()));
            self.write_en = true;
        } else {
            self.update = None;
            self.write_en = false;
        }
        //read_data is combinational w.r.t addr0;
        //if there was an update, [do_tick()] will return a vector w/ a done value
        //else, empty vector return
        Ok(vec![(
            ir::Id::from("read_data"),
            if addr0 < self.size {
                self.data[addr0 as usize].clone()
            } else {
                Value::zeroes(self.width as usize)
            },
        )])
    }

    fn reset(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        //so we don't have to keep using .as_u64()
        let addr0 = addr0.as_u64();
        //check that input data is the appropriate width as well
        let old = self.data[addr0 as usize].clone();
        //also clear update
        self.update = None;
        self.write_en = false;
        self.last_index = addr0;
        Ok(vec![
            ("read_data".into(), old),
            (ir::Id::from("done"), Value::zeroes(1)),
        ])
    }

    fn serialize(&self, signed: Option<PrintCode>) -> Serializable {
        let code = signed.unwrap_or_default();
        Serializable::Array(
            self.data
                .iter()
                .map(|x| Entry::from_val_code(x, &code))
                .collect(),
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
/// read_data: WIDTH - The value stored at mem\[addr0\]\[addr1\]. This value is combinational with respect to addr0 and addr1.
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
    full_name: ir::Id,
    allow_invalid_memory_access: bool,
}

impl StdMemD2 {
    pub fn from_constants(
        width: u64,
        d0_size: u64,
        d1_size: u64,
        d0_idx_size: u64,
        d1_idx_size: u64,
        full_name: ir::Id,
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
        Self::new(&bindings, full_name, false)
    }

    #[inline]
    fn max_idx(&self) -> u64 {
        self.d0_size * self.d1_size
    }

    /// Instantiates a new StdMemD2 storing data of width `width`, containing
    /// `d0_size` * `d1_size` slots for memory, accepting indices \[addr0\]\[addr1\] of widths
    /// `d0_idx_size` and `d1_idx_size` respectively.
    /// Initially the memory is filled with all 0s.
    pub fn new(
        params: &ir::Binding,
        full_name: ir::Id,
        allow_invalid_memory_access: bool,
    ) -> StdMemD2 {
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
            full_name,
            allow_invalid_memory_access,
        }
    }

    pub fn initialize_memory(
        &mut self,
        vals: &[Value],
    ) -> InterpreterResult<()> {
        if (self.d0_size * self.d1_size) as usize != vals.len() {
            return Err(InterpreterError::IncorrectMemorySize {
                mem_dim: "2D".into(),
                expected: self.d0_size * self.d1_size,
                given: vals.len(),
            });
        }

        for (idx, val) in vals.iter().enumerate() {
            self.data[idx] = val.truncate(self.width.try_into().unwrap())
        }
        Ok(())
    }

    #[inline]
    fn calc_addr(&self, addr0: u64, addr1: u64) -> u64 {
        addr0 * self.d1_size + addr1
    }
}

impl Named for StdMemD2 {
    fn get_full_name(&self) -> &ir::Id {
        &self.full_name
    }
}

impl Primitive for StdMemD2 {
    //null-op for now
    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        if self.calc_addr(self.last_idx.0, self.last_idx.1) >= self.max_idx() {
            if self.allow_invalid_memory_access {
                if self.write_en {
                    return Ok(vec![
                        (ir::Id::from("read_data"), Value::zeroes(self.width)),
                        (ir::Id::from("done"), Value::bit_high()),
                    ]);
                } else {
                    return Ok(vec![
                        (ir::Id::from("done"), Value::bit_low()),
                        (ir::Id::from("read_data"), Value::zeroes(self.width)),
                    ]);
                }
            }
            return Err(InterpreterError::InvalidMemoryAccess {
                name: self.full_name.clone(),
                access: vec![self.last_idx.0, self.last_idx.1],
                dims: vec![self.d0_size, self.d1_size],
            });
        }
        let out = if self.write_en {
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
                unreachable!();
            }
        } else {
            vec![(ir::Id::from("done"), Value::bit_low())]
        };

        Ok(out)
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

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        get_inputs![inputs;
            input: "write_data",
            write_en: "write_en",
            addr0: "addr0",
            addr1: "addr1"
        ];
        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();
        self.last_idx = (addr0, addr1);
        let real_addr = self.calc_addr(addr0, addr1);

        if write_en.as_bool() {
            self.update = Some((real_addr, (*input).clone()));
            self.write_en = true;
        } else {
            self.update = None;
            self.write_en = false;
        }
        Ok(vec![(
            ir::Id::from("read_data"),
            if real_addr < self.max_idx() {
                self.data[real_addr as usize].clone()
            } else {
                Value::zeroes(self.width as usize)
            },
        )])
    }

    fn reset(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
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

        Ok(vec![
            (ir::Id::from("read_data"), old),
            (ir::Id::from("done"), Value::zeroes(1)),
        ])
    }

    fn serialize(&self, signed: Option<PrintCode>) -> Serializable {
        let code = signed.unwrap_or_default();
        Serializable::Array(
            self.data
                .iter()
                .map(|x| Entry::from_val_code(x, &code))
                .collect(),
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
/// read_data: WIDTH - The value stored at mem\[addr0\]\[addr1\]\[addr2\]. This value is combinational with respect to addr0, addr1, and addr2.
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
    full_name: ir::Id,
    allow_invalid_memory_access: bool,
}

impl StdMemD3 {
    #[allow(clippy::too_many_arguments)]
    pub fn from_constants(
        width: u64,
        d0_size: u64,
        d1_size: u64,
        d2_size: u64,
        d0_idx_size: u64,
        d1_idx_size: u64,
        d2_idx_size: u64,
        full_name: ir::Id,
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
        Self::new(&bindings, full_name, false)
    }
    /// Instantiates a new StdMemD3 storing data of width `width`, containing
    /// `d0_size` * `d1_size` * `d2_size` slots for memory, accepting indices \[addr0\]\[addr1\]\[addr2\] of widths
    /// \[d0_idx_size\], \[d1_idx_size\], and \[d2_idx_size\] respectively.
    /// Initially the memory is filled with all 0s.
    pub fn new(
        params: &ir::Binding,
        full_name: ir::Id,
        allow_invalid_memory_access: bool,
    ) -> StdMemD3 {
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
            full_name,
            allow_invalid_memory_access,
        }
    }

    pub fn initialize_memory(
        &mut self,
        vals: &[Value],
    ) -> InterpreterResult<()> {
        if (self.d0_size * self.d1_size * self.d2_size) as usize != vals.len() {
            return Err(InterpreterError::IncorrectMemorySize {
                mem_dim: "3D".into(),
                expected: self.d0_size * self.d1_size * self.d2_size,
                given: vals.len(),
            });
        }

        for (idx, val) in vals.iter().enumerate() {
            self.data[idx] = val.truncate(self.width.try_into().unwrap())
        }
        Ok(())
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

impl Named for StdMemD3 {
    fn get_full_name(&self) -> &ir::Id {
        &self.full_name
    }
}

impl Primitive for StdMemD3 {
    //null-op for now
    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        let (addr0, addr1, addr2) = self.last_idx;
        if self.calc_addr(addr0, addr1, addr2) >= self.max_idx() {
            if self.allow_invalid_memory_access {
                if self.write_en {
                    return Ok(vec![
                        (ir::Id::from("read_data"), Value::zeroes(self.width)),
                        (ir::Id::from("done"), Value::bit_high()),
                    ]);
                } else {
                    return Ok(vec![
                        (ir::Id::from("done"), Value::bit_low()),
                        (ir::Id::from("read_data"), Value::zeroes(self.width)),
                    ]);
                }
            }
            return Err(InterpreterError::InvalidMemoryAccess {
                name: self.full_name.clone(),
                access: vec![self.last_idx.0, self.last_idx.1, self.last_idx.2],
                dims: vec![self.d0_size, self.d1_size, self.d2_size],
            });
        }

        let out = if self.write_en {
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
                unreachable!();
            }
        } else {
            vec![(ir::Id::from("done"), Value::bit_low())]
        };

        Ok(out)
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

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        get_inputs![inputs;
            input: "write_data",
            write_en: "write_en",
            addr0: "addr0",
            addr1: "addr1",
            addr2: "addr2"
        ];

        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();
        let addr2 = addr2.as_u64();
        self.last_idx = (addr0, addr1, addr2);

        let real_addr = self.calc_addr(addr0, addr1, addr2);
        if write_en.as_bool() {
            self.update = Some((real_addr, (*input).clone()));
            self.write_en = true;
        } else {
            self.update = None;
            self.write_en = false;
        }
        Ok(vec![(
            ir::Id::from("read_data"),
            if real_addr < self.max_idx() {
                self.data[real_addr as usize].clone()
            } else {
                Value::zeroes(self.width as usize)
            },
        )])
    }

    fn reset(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
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
        Ok(vec![
            (ir::Id::from("read_data"), old),
            (ir::Id::from("done"), Value::zeroes(1)),
        ])
    }

    fn serialize(&self, signed: Option<PrintCode>) -> Serializable {
        let code = signed.unwrap_or_default();
        Serializable::Array(
            self.data
                .iter()
                .map(|x| Entry::from_val_code(x, &code))
                .collect(),
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
/// read_data: WIDTH - The value stored at mem\[addr0\]\[addr1\]\[addr2\]\[addr3\]. This value is combinational with respect to addr0, addr1, addr2, and addr3.
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
    full_name: ir::Id,
    allow_invalid_memory_access: bool,
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
        full_name: ir::Id,
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
        Self::new(&bindings, full_name, false)
    }
    // Instantiates a new StdMemD3 storing data of width `width`, containing
    /// `d0_size` * `d1_size` * `d2_size` * `d3_size` slots for memory, accepting indecies `addr0``addr1``addr2``addr3` of widths
    /// `d0_idx_size`, `d1_idx_size`, `d2_idx_size` and `d3_idx_size` respectively.
    /// Initially the memory is filled with all 0s.
    pub fn new(
        params: &ir::Binding,
        full_name: ir::Id,
        allow_invalid_memory_access: bool,
    ) -> StdMemD4 {
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
            full_name,
            allow_invalid_memory_access,
        }
    }

    pub fn initialize_memory(
        &mut self,
        vals: &[Value],
    ) -> InterpreterResult<()> {
        if (self.d0_size * self.d1_size * self.d2_size * self.d3_size) as usize
            != vals.len()
        {
            return Err(InterpreterError::IncorrectMemorySize {
                mem_dim: "4D".into(),
                expected: self.d0_size
                    * self.d1_size
                    * self.d2_size
                    * self.d3_size,
                given: vals.len(),
            });
        }

        for (idx, val) in vals.iter().enumerate() {
            self.data[idx] = val.truncate(self.width.try_into().unwrap())
        }

        Ok(())
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
impl Named for StdMemD4 {
    fn get_full_name(&self) -> &ir::Id {
        &self.full_name
    }
}

impl Primitive for StdMemD4 {
    //null-op for now
    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        let (addr0, addr1, addr2, addr3) = self.last_idx;
        if self.calc_addr(addr0, addr1, addr2, addr3) >= self.max_idx() {
            if self.allow_invalid_memory_access {
                if self.write_en {
                    return Ok(vec![
                        (ir::Id::from("read_data"), Value::zeroes(self.width)),
                        (ir::Id::from("done"), Value::bit_high()),
                    ]);
                } else {
                    return Ok(vec![
                        (ir::Id::from("done"), Value::bit_low()),
                        (ir::Id::from("read_data"), Value::zeroes(self.width)),
                    ]);
                }
            }

            return Err(InterpreterError::InvalidMemoryAccess {
                name: self.full_name.clone(),
                access: vec![
                    self.last_idx.0,
                    self.last_idx.1,
                    self.last_idx.2,
                    self.last_idx.3,
                ],
                dims: vec![
                    self.d0_size,
                    self.d1_size,
                    self.d2_size,
                    self.d3_size,
                ],
            });
        }

        if self.write_en {
            assert!(self.update.is_some());
            self.write_en = false;
            if let Some((idx, val)) = self.update.take() {
                self.data[idx as usize] = val;
                Ok(vec![
                    (
                        ir::Id::from("read_data"),
                        self.data[idx as usize].clone(),
                    ),
                    (ir::Id::from("done"), Value::bit_high()),
                ])
            } else {
                unreachable!();
            }
        } else {
            Ok(vec![(ir::Id::from("done"), Value::bit_low())])
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

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        get_inputs![inputs;
            input: "write_data",
            write_en: "write_en",
            addr0: "addr0",
            addr1: "addr1",
            addr2: "addr2",
            addr3: "addr3"
        ];

        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();
        let addr2 = addr2.as_u64();
        let addr3 = addr3.as_u64();
        self.last_idx = (addr0, addr1, addr2, addr3);

        let real_addr = self.calc_addr(addr0, addr1, addr2, addr3);
        if write_en.as_bool() {
            self.update = Some((real_addr, (*input).clone()));
            self.write_en = true;
        } else {
            self.update = None;
            self.write_en = false;
        }
        Ok(vec![(
            ir::Id::from("read_data"),
            if real_addr < self.max_idx() {
                self.data[real_addr as usize].clone()
            } else {
                Value::zeroes(self.width as usize)
            },
        )])
    }

    fn reset(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
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
        Ok(vec![
            (ir::Id::from("read_data"), old),
            (ir::Id::from("done"), Value::zeroes(1)),
        ])
    }

    fn serialize(&self, signed: Option<PrintCode>) -> Serializable {
        let code = signed.unwrap_or_default();
        Serializable::Array(
            self.data
                .iter()
                .map(|x| Entry::from_val_code(x, &code))
                .collect(),
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

pub struct StdFpMultPipe<const SIGNED: bool> {
    pub width: u64,
    pub int_width: u64,
    pub frac_width: u64,
    pub product: Value,
    update: BinOpUpdate,
    queue: ShiftBuffer<Value, 2>,
    full_name: ir::Id,
    logger: logging::Logger,
}

impl<const SIGNED: bool> StdFpMultPipe<SIGNED> {
    pub fn from_constants(
        width: u64,
        int_width: u64,
        frac_width: u64,
        full_name: ir::Id,
    ) -> Self {
        assert_eq!(width, int_width + frac_width);
        Self {
            width,
            int_width,
            frac_width,
            product: Value::zeroes(width),
            update: BinOpUpdate::None,
            queue: ShiftBuffer::default(),
            logger: logging::new_sublogger(&full_name),
            full_name,
        }
    }

    pub fn new(params: &ir::Binding, full_name: ir::Id) -> Self {
        let width = get_param(params, "WIDTH")
            .expect("WIDTH parameter missing for fp mult");
        let int_width = get_param(params, "INT_WIDTH")
            .expect("INT_WIDTH parameter missing for fp mult");
        let frac_width = get_param(params, "FRAC_WIDTH")
            .expect("FRAC_WIDTH parameter missing for fp mult");

        Self::from_constants(width, int_width, frac_width, full_name)
    }
}

impl<const SIGNED: bool> Named for StdFpMultPipe<SIGNED> {
    fn get_full_name(&self) -> &ir::Id {
        &self.full_name
    }
}

impl<const SIGNED: bool> Primitive for StdFpMultPipe<SIGNED> {
    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        let out = match self.update.take() {
            BinOpUpdate::None => {
                self.queue.reset();
                vec![
                    (ir::Id::from("out"), self.product.clone()),
                    (ir::Id::from("done"), Value::bit_low()),
                ]
            }
            BinOpUpdate::Reset => {
                self.queue.reset();
                vec![
                    (ir::Id::from("out"), Value::zeroes(self.width)),
                    (ir::Id::from("done"), Value::bit_low()),
                ]
            }
            BinOpUpdate::Value(left, right) => {
                let backing_val = if SIGNED {
                    Value::from(
                        left.as_signed() * right.as_signed(),
                        2 * self.width,
                    )
                } else {
                    Value::from(
                        left.as_unsigned() * right.as_unsigned(),
                        2 * self.width,
                    )
                };

                let upper_idx = (2 * self.width) - self.int_width - 1;
                let lower_idx = self.width - self.int_width;

                if backing_val
                    .iter()
                    .rev()
                    .take((backing_val.len() - 1) - upper_idx as usize)
                    .any(|x| x)
                    && (!backing_val
                        .iter()
                        .rev()
                        .take((backing_val.len() - 1) - upper_idx as usize)
                        .all(|x| x)
                        | !SIGNED)
                {
                    let out = backing_val
                        .clone()
                        .slice_out(upper_idx as usize, lower_idx as usize);

                    warn!(
                        self.logger,
                        "Computation over/underflow: {} to {}",
                        if SIGNED {
                            format!(
                                "{:.fw$}",
                                backing_val
                                    .as_sfp((self.frac_width * 2) as usize),
                                fw = DECIMAL_PRINT_WIDTH
                            )
                        } else {
                            format!(
                                "{:.fw$}",
                                backing_val
                                    .as_ufp((self.frac_width * 2) as usize),
                                fw = DECIMAL_PRINT_WIDTH
                            )
                        },
                        if SIGNED {
                            format!(
                                "{:.fw$}",
                                out.as_sfp(self.frac_width as usize),
                                fw = DECIMAL_PRINT_WIDTH
                            )
                        } else {
                            format!(
                                "{:.fw$}",
                                out.as_ufp(self.frac_width as usize),
                                fw = DECIMAL_PRINT_WIDTH
                            )
                        },
                    )
                }

                let computed = Some(
                    backing_val
                        .slice_out(upper_idx as usize, lower_idx as usize),
                );

                if let Some(out) = self.queue.shift(computed) {
                    self.product = out.clone();
                    vec![
                        (ir::Id::from("out"), out),
                        (ir::Id::from("done"), Value::bit_high()),
                    ]
                } else {
                    vec![
                        (ir::Id::from("out"), Value::zeroes(self.width)),
                        (ir::Id::from("done"), Value::bit_low()),
                    ]
                }
            }
        };

        Ok(out)
    }

    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        validate![inputs;
            left: self.width,
            right: self.width,
            reset: 1,
            go: 1
        ];
    }

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        get_inputs![inputs;
            left: "left",
            right: "right",
            reset: "reset",
            go: "go"
        ];
        self.update = if reset.as_bool() {
            BinOpUpdate::Reset
        } else if go.as_bool() {
            BinOpUpdate::Value(left.clone(), right.clone())
        } else {
            BinOpUpdate::None
        };

        Ok(vec![])
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        self.update.clear();
        self.queue.reset();
        Ok(vec![
            (ir::Id::from("out"), self.product.clone()),
            (ir::Id::from("done"), Value::bit_low()),
        ])
    }
}

pub struct StdFpDivPipe<const SIGNED: bool> {
    pub width: u64,
    pub int_width: u64,
    pub frac_width: u64,
    pub quotient: Value,
    pub remainder: Value,
    update: BinOpUpdate,
    queue: ShiftBuffer<(Value, Value), 2>,
    full_name: ir::Id,
    logger: logging::Logger,
}
impl<const SIGNED: bool> StdFpDivPipe<SIGNED> {
    pub fn from_constants(
        width: u64,
        int_width: u64,
        frac_width: u64,
        name: ir::Id,
    ) -> Self {
        assert_eq!(width, int_width + frac_width);
        Self {
            width,
            int_width,
            frac_width,
            quotient: Value::zeroes(width),
            remainder: Value::zeroes(width),
            update: BinOpUpdate::None,
            queue: ShiftBuffer::default(),
            logger: logging::new_sublogger(&name),
            full_name: name,
        }
    }

    pub fn new(params: &ir::Binding, full_name: ir::Id) -> Self {
        let width = get_param(params, "WIDTH")
            .expect("WIDTH parameter missing for fp div");
        let int_width = get_param(params, "INT_WIDTH")
            .expect("INT_WIDTH parameter missing for fp div");
        let frac_width = get_param(params, "FRAC_WIDTH")
            .expect("FRAC_WIDTH parameter missing for fp div");

        Self::from_constants(width, int_width, frac_width, full_name)
    }
}

impl<const SIGNED: bool> Named for StdFpDivPipe<SIGNED> {
    fn get_full_name(&self) -> &ir::Id {
        &self.full_name
    }
}

impl<const SIGNED: bool> Primitive for StdFpDivPipe<SIGNED> {
    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        let out = match self.update.take() {
            BinOpUpdate::None => {
                self.queue.reset();
                vec![
                    (ir::Id::from("out_quotient"), self.quotient.clone()),
                    (ir::Id::from("out_remainder"), self.remainder.clone()),
                    (ir::Id::from("done"), Value::bit_low()),
                ]
            }
            BinOpUpdate::Reset => {
                self.queue.reset();
                vec![
                    (ir::Id::from("out_quotient"), Value::zeroes(self.width)),
                    (ir::Id::from("out_remainder"), Value::zeroes(self.width)),
                    (ir::Id::from("done"), Value::bit_low()),
                ]
            }
            BinOpUpdate::Value(left, right) => {
                let (q, r) = if right.as_u64() != 0 {
                    if SIGNED {
                        (
                            Value::from(
                                (left.as_signed() << self.frac_width as usize)
                                    / right.as_signed(),
                                self.width,
                            ),
                            Value::from(
                                left.as_signed()
                                    - right.as_signed()
                                        * floored_division(
                                            &left.as_signed(),
                                            &right.as_signed(),
                                        ),
                                self.width,
                            ),
                        )
                    } else {
                        (
                            Value::from(
                                (left.as_unsigned()
                                    << self.frac_width as usize)
                                    / right.as_unsigned(),
                                self.width,
                            ),
                            Value::from(
                                left.as_unsigned()
                                    .rem_euclid(right.as_unsigned()),
                                self.width,
                            ),
                        )
                    }
                } else {
                    warn!(self.logger, "Division by zero");
                    (Value::zeroes(self.width), Value::zeroes(self.width))
                };

                if let Some((q, r)) = self.queue.shift(Some((q, r))) {
                    self.quotient = q.clone();
                    self.remainder = r.clone();
                    vec![
                        (ir::Id::from("out_quotient"), q),
                        (ir::Id::from("out_remainder"), r),
                        (ir::Id::from("done"), Value::bit_high()),
                    ]
                } else {
                    vec![
                        (ir::Id::from("out_quotient"), self.quotient.clone()),
                        (ir::Id::from("out_remainder"), self.remainder.clone()),
                        (ir::Id::from("done"), Value::bit_low()),
                    ]
                }
            }
        };
        Ok(out)
    }

    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        validate![inputs;
            left: self.width,
            right: self.width,
            reset: 1,
            go: 1
        ];
    }

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        get_inputs![inputs;
            left: "left",
            right: "right",
            reset: "reset",
            go: "go"
        ];

        self.update = if reset.as_bool() {
            BinOpUpdate::Reset
        } else if go.as_bool() {
            BinOpUpdate::Value(left.clone(), right.clone())
        } else {
            BinOpUpdate::None
        };

        Ok(vec![])
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        self.update.clear();
        self.queue.reset();
        Ok(vec![
            (ir::Id::from("out_quotient"), self.quotient.clone()),
            (ir::Id::from("out_remainder"), self.remainder.clone()),
            (ir::Id::from("done"), Value::bit_low()),
        ])
    }
}

pub(crate) fn floored_division(left: &IBig, right: &IBig) -> IBig {
    let div = left / right;

    if left.signum() != ibig!(-1) && right.signum() != ibig!(-1) {
        div
    } else if (div.signum() == (-1).into() || div.signum() == 0.into())
        && (left != &(&div * right))
    {
        div - 1_i32
    } else {
        div
    }
}

/// Implementation of integer square root via a basic binary search algorithm
/// based on wikipedia psuedocode
pub(crate) fn int_sqrt(i: &UBig) -> UBig {
    let mut lower: UBig = ubig!(0);
    let mut upper: UBig = i + ubig!(1);
    let mut temp: UBig;

    while lower != (&upper - ubig!(1)) {
        temp = (&lower + &upper) / ubig!(2);
        if &(&temp * &temp) <= i {
            lower = temp
        } else {
            upper = temp
        }
    }
    lower
}

type SqrtUpdate = RegUpdate;

pub struct StdSqrt<const FP: bool> {
    pub width: u64,
    pub output: Value,
    frac_width: u64,
    update: SqrtUpdate,
    name: ir::Id,
}

impl<const FP: bool> StdSqrt<FP> {
    pub fn new(params: &ir::Binding, name: ir::Id) -> Self {
        let width = get_param(params, "WIDTH")
            .expect("Missing `WIDTH` param from std_sqrt binding");
        let frac_width = if FP {
            get_param(params, "FRAC_WIDTH")
                .expect("Missing `FRAC_WIDTH` param from std_sqrt binding")
        } else {
            0
        };

        Self {
            width,
            frac_width,
            output: Value::zeroes(width),
            update: SqrtUpdate::None,
            name,
        }
    }
}

impl<const FP: bool> Named for StdSqrt<FP> {
    fn get_full_name(&self) -> &ir::Id {
        &self.name
    }
}

impl<const FP: bool> Primitive for StdSqrt<FP> {
    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        let out = match self.update.take() {
            SqrtUpdate::None => vec![
                ("out".into(), self.output.clone()),
                ("done".into(), Value::bit_low()),
            ],
            SqrtUpdate::Reset => {
                self.output = Value::zeroes(self.width);
                vec![
                    ("out".into(), self.output.clone()),
                    ("done".into(), Value::bit_low()),
                ]
            }
            SqrtUpdate::Value(v) => {
                self.output = if FP {
                    let val = int_sqrt(
                        &(v.as_unsigned() << (self.frac_width as usize)),
                    );
                    Value::from(val, self.width)
                } else {
                    let val = int_sqrt(&v.as_unsigned());
                    Value::from(val, self.width)
                };

                vec![
                    ("out".into(), self.output.clone()),
                    ("done".into(), Value::bit_high()),
                ]
            }
        };

        Ok(out)
    }

    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        validate![inputs;
            r#in: self.width,
            go: 1
        ]
    }

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        get_inputs![inputs;
            in_val: "in",
            go: "go",
            reset: "reset"
        ];

        self.update = if reset.as_bool() {
            SqrtUpdate::Reset
        } else if go.as_bool() {
            SqrtUpdate::Value(in_val.clone())
        } else {
            SqrtUpdate::None
        };

        Ok(vec![])
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        self.update.clear();
        Ok(vec![
            ("out".into(), self.output.clone()),
            ("done".into(), Value::bit_low()),
        ])
    }
}

enum SeqMemAction<T> {
    None,
    Read(T),
    Write(T, Value),
    Reset,
}

impl<T> Default for SeqMemAction<T> {
    fn default() -> Self {
        Self::None
    }
}

impl<T> SeqMemAction<T> {
    #[inline]
    fn take(&mut self) -> Self {
        std::mem::take(self)
    }

    fn clear(&mut self) {
        *self = Self::None
    }
}

pub trait MemBinder: Sized {
    fn new(params: &ir::Binding, full_name: ir::Id) -> Self;

    fn get_idx(
        &self,
        inputs: &[(ir::Id, &Value)],
        allow_invalid_memory_access: bool,
    ) -> InterpreterResult<u64>;

    fn validate(&self, inputs: &[(ir::Id, &Value)]);

    fn get_dimensions(&self) -> Shape;
}

use super::primitive::Shape;
struct MemD1 {
    size: u64,
    idx_size: u64,
    full_name: ir::Id,
}

impl MemBinder for MemD1 {
    fn new(params: &ir::Binding, full_name: ir::Id) -> Self {
        get_params![params;
            // width: "WIDTH",
            size: "SIZE",
            idx_size: "IDX_SIZE"
        ];

        Self {
            size,
            idx_size,
            full_name,
        }
    }

    fn get_idx(
        &self,
        inputs: &[(ir::Id, &Value)],
        allow_invalid_memory_access: bool,
    ) -> InterpreterResult<u64> {
        let idx = inputs
            .iter()
            .find(|(id, _)| id == "addr0")
            .unwrap()
            .1
            .as_u64();

        if idx >= self.size && !allow_invalid_memory_access {
            Err(InterpreterError::InvalidMemoryAccess {
                access: vec![idx],
                dims: vec![self.size],
                name: self.full_name.clone(),
            })
        } else {
            Ok(idx)
        }
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        validate![inputs;
            addr0: self.idx_size
        ]
    }

    fn get_dimensions(&self) -> Shape {
        Shape::D1((self.size as usize,))
    }
}

struct MemD2 {
    d0_size: u64,
    d1_size: u64,
    d0_idx_size: u64,
    d1_idx_size: u64,
    full_name: ir::Id,
}
impl MemBinder for MemD2 {
    fn new(params: &ir::Binding, full_name: ir::Id) -> Self {
        get_params![params;
            d0_size: "D0_SIZE",
            d1_size: "D1_SIZE",
            d0_idx_size: "D0_IDX_SIZE",
            d1_idx_size: "D1_IDX_SIZE"
        ];

        Self {
            d0_size,
            d1_size,
            d0_idx_size,
            d1_idx_size,
            full_name,
        }
    }

    fn get_idx(
        &self,
        inputs: &[(ir::Id, &Value)],
        allow_invalid_memory_access: bool,
    ) -> InterpreterResult<u64> {
        get_inputs![inputs;
            addr0 [u64]: "addr0",
            addr1 [u64]: "addr1"
        ];

        let address = addr0 * self.d1_size + addr1;

        if address >= (self.d0_size * self.d1_size)
            && !allow_invalid_memory_access
        {
            Err(InterpreterError::InvalidMemoryAccess {
                access: vec![addr0, addr1],
                dims: vec![self.d0_size, self.d1_size],
                name: self.full_name.clone(),
            })
        } else {
            Ok(address)
        }
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        validate![inputs;
            addr0: self.d0_idx_size,
            addr1: self.d1_idx_size
        ]
    }

    fn get_dimensions(&self) -> Shape {
        Shape::D2((self.d0_size as usize, self.d1_size as usize))
    }
}

struct MemD3 {
    d0_size: u64,
    d1_size: u64,
    d2_size: u64,
    d0_idx_size: u64,
    d1_idx_size: u64,
    d2_idx_size: u64,
    full_name: ir::Id,
}

impl MemBinder for MemD3 {
    fn new(params: &ir::Binding, full_name: ir::Id) -> Self {
        get_params![params;
            d0_size: "D0_SIZE",
            d1_size: "D1_SIZE",
            d2_size: "D2_SIZE",
            d0_idx_size: "D0_IDX_SIZE",
            d1_idx_size: "D1_IDX_SIZE",
            d2_idx_size: "D2_IDX_SIZE"
        ];

        Self {
            d0_size,
            d1_size,
            d2_size,
            d0_idx_size,
            d1_idx_size,
            d2_idx_size,
            full_name,
        }
    }

    fn get_idx(
        &self,
        inputs: &[(ir::Id, &Value)],
        allow_invalid_memory_access: bool,
    ) -> InterpreterResult<u64> {
        get_inputs![inputs;
            addr0 [u64]: "addr0",
            addr1 [u64]: "addr1",
            addr2 [u64]: "addr2"
        ];

        let address = self.d2_size * (addr0 * self.d1_size + addr1) + addr2;

        if address >= (self.d0_size * self.d1_size * self.d2_size)
            && !allow_invalid_memory_access
        {
            Err(InterpreterError::InvalidMemoryAccess {
                access: vec![addr0, addr1, addr2],
                dims: vec![self.d0_size, self.d1_size, self.d2_size],
                name: self.full_name.clone(),
            })
        } else {
            Ok(address)
        }
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        validate![inputs;
            addr0: self.d0_idx_size,
            addr1: self.d1_idx_size,
            addr2: self.d2_idx_size
        ]
    }

    fn get_dimensions(&self) -> Shape {
        Shape::D3((
            self.d0_size as usize,
            self.d1_size as usize,
            self.d2_size as usize,
        ))
    }
}

struct MemD4 {
    d0_size: u64,
    d1_size: u64,
    d2_size: u64,
    d3_size: u64,
    d0_idx_size: u64,
    d1_idx_size: u64,
    d2_idx_size: u64,
    d3_idx_size: u64,
    full_name: ir::Id,
}

impl MemBinder for MemD4 {
    fn new(params: &ir::Binding, full_name: ir::Id) -> Self {
        get_params![params;
            d0_size: "D0_SIZE",
            d1_size: "D1_SIZE",
            d2_size: "D2_SIZE",
            d3_size: "D3_SIZE",
            d0_idx_size: "D0_IDX_SIZE",
            d1_idx_size: "D1_IDX_SIZE",
            d2_idx_size: "D2_IDX_SIZE",
            d3_idx_size: "D3_IDX_SIZE"
        ];

        Self {
            d0_size,
            d1_size,
            d2_size,
            d3_size,
            d0_idx_size,
            d1_idx_size,
            d2_idx_size,
            d3_idx_size,
            full_name,
        }
    }

    fn get_idx(
        &self,
        inputs: &[(ir::Id, &Value)],
        allow_invalid_memory_access: bool,
    ) -> InterpreterResult<u64> {
        get_inputs![inputs;
            addr0 [u64]: "addr0",
            addr1 [u64]: "addr1",
            addr2 [u64]: "addr2",
            addr3 [u64]: "addr3"
        ];

        let address = self.d3_size
            * (self.d2_size * (addr0 * self.d1_size + addr1) + addr2)
            + addr3;

        if address
            >= (self.d0_size * self.d1_size * self.d2_size * self.d3_size)
            && !allow_invalid_memory_access
        {
            Err(InterpreterError::InvalidMemoryAccess {
                access: vec![addr0, addr1, addr2, addr3],
                dims: vec![
                    self.d0_size,
                    self.d1_size,
                    self.d2_size,
                    self.d3_size,
                ],
                name: self.full_name.clone(),
            })
        } else {
            Ok(address)
        }
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        validate![inputs;
            addr0: self.d0_idx_size,
            addr1: self.d1_idx_size,
            addr2: self.d2_idx_size,
            addr3: self.d3_idx_size
        ]
    }

    fn get_dimensions(&self) -> Shape {
        Shape::D4((
            self.d0_size as usize,
            self.d1_size as usize,
            self.d2_size as usize,
            self.d3_size as usize,
        ))
    }
}

pub struct SeqMem<T: MemBinder> {
    mem_binder: T,
    // parameters
    width: u64,
    // Internal Details
    data: Vec<Value>,
    full_name: ir::Id,
    allow_invalid_memory_access: bool,
    // I/O
    read_out: Value,
    read_en: bool,
    write_en: bool,
    reset_signal: bool,
    update: SeqMemAction<InterpreterResult<u64>>,
}
impl<T: MemBinder> Named for SeqMem<T> {
    fn get_full_name(&self) -> &ir::Id {
        &self.full_name
    }
}

impl<T: MemBinder> Primitive for SeqMem<T> {
    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        validate![inputs;
            read_en: 1,
            write_en: 1,
            reset: 1,
            r#in: self.width
        ];
        self.mem_binder.validate(inputs);
    }

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        get_inputs![inputs;
            read_en [bool]: "read_en",
            write_en [bool]: "write_en",
            reset [bool]: "reset",
            input: "in"
        ];

        self.read_en = read_en;
        self.write_en = write_en;
        self.reset_signal = reset;
        let idx = self
            .mem_binder
            .get_idx(inputs, self.allow_invalid_memory_access);

        self.update = if reset {
            SeqMemAction::Reset
        } else if write_en && read_en {
            SeqMemAction::None
        } else if write_en {
            SeqMemAction::Write(idx, input.clone())
        } else if read_en {
            SeqMemAction::Read(idx)
        } else {
            SeqMemAction::None
        };

        // nothing on comb path
        Ok(vec![])
    }

    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        if self.read_en && self.write_en {
            return Err(InterpreterError::SeqMemoryError);
        }
        match self.update.take() {
            SeqMemAction::Read(idx) => {
                let idx = idx?;
                if idx >= self.data.len() as u64 {
                    self.read_out = Value::zeroes(self.width)
                } else {
                    self.read_out = self.data[idx as usize].clone()
                }

                Ok(vec![
                    ("out".into(), self.read_out.clone()),
                    ("read_done".into(), Value::bit_high()),
                    ("write_done".into(), Value::bit_low()),
                ])
            }
            SeqMemAction::Write(idx, v) => {
                let idx = idx?;
                if idx >= self.data.len() as u64 {
                    self.read_out = v;
                } else {
                    self.data[idx as usize] = v.clone();
                    self.read_out = v;
                }

                Ok(vec![
                    ("out".into(), self.read_out.clone()),
                    ("read_done".into(), Value::bit_low()),
                    ("write_done".into(), Value::bit_high()),
                ])
            }
            SeqMemAction::Reset => {
                self.read_out = Value::zeroes(self.width);
                Ok(vec![
                    ("out".into(), self.read_out.clone()),
                    ("read_done".into(), Value::bit_low()),
                    ("write_done".into(), Value::bit_low()),
                ])
            }
            SeqMemAction::None => Ok(vec![
                ("out".into(), self.read_out.clone()),
                ("read_done".into(), Value::bit_low()),
                ("write_done".into(), Value::bit_low()),
            ]),
        }
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        self.update.clear();

        Ok(vec![
            ("out".into(), self.read_out.clone()),
            ("read_done".into(), Value::bit_low()),
            ("write_done".into(), Value::bit_low()),
        ])
    }

    fn serialize(&self, code: Option<PrintCode>) -> Serializable {
        let code = code.unwrap_or_default();

        Serializable::Array(
            self.data
                .iter()
                .map(|x| Entry::from_val_code(x, &code))
                .collect(),
            self.mem_binder.get_dimensions(),
        )
    }

    fn has_serializeable_state(&self) -> bool {
        true
    }
}
