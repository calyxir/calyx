use super::prim_utils::{
    get_inputs, get_param, get_params, output, ShiftBuffer,
};
use super::primitive::Named;
use super::{Entry, Primitive, Serializable};
use crate::errors::{InterpreterError, InterpreterResult};
use crate::logging::{self, warn};
use crate::utils::{construct_bindings, PrintCode};
use crate::values::Value;
use crate::{validate, validate_friendly};
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
                    return Err(InterpreterError::OverflowError.into());
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
                        return Err(InterpreterError::OverflowError.into());
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

enum StdMemAction {
    None,
    Read(InterpreterResult<u64>),
    Write(InterpreterResult<u64>, Value),
}

impl StdMemAction {
    #[inline]
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

impl Default for StdMemAction {
    fn default() -> Self {
        Self::None
    }
}

/// The primitive skeleton for the std_mem primitives. Supports combinational
/// reads and 1-cycle writes. The read_data output is not latched.
pub struct StdMem<T: MemBinder> {
    mem_binder: T,
    width: u64,
    data: Vec<Value>,
    full_name: ir::Id,
    allow_invalid_memory_access: bool,
    update: StdMemAction,
}

impl<T: MemBinder> StdMem<T> {
    pub fn new(
        params: &ir::Binding,
        name: ir::Id,
        allow_invalid_memory_access: bool,
    ) -> Self {
        let mem_binder = T::new(params, name.clone());
        let width =
            get_param(params, "WIDTH").expect("Missing WIDTH param for memory");

        let data =
            vec![Value::zeroes(width as usize); mem_binder.get_array_length()];

        Self {
            mem_binder,
            width,
            data,
            full_name: name,
            allow_invalid_memory_access,
            update: StdMemAction::None,
        }
    }

    pub fn from_initial_mem(
        params: &ir::Binding,
        name: ir::Id,
        allow_invalid_memory_access: bool,
        initial: Vec<Value>,
    ) -> InterpreterResult<Self> {
        let mem_binder = T::new(params, name.clone());
        let width =
            get_param(params, "WIDTH").expect("Missing WIDTH param for memory");

        let size = mem_binder.get_array_length();

        if initial.len() != size {
            return Err(InterpreterError::IncorrectMemorySize {
                mem_dim: mem_binder.get_dimensions().dim_str(),
                expected: size as u64,
                given: initial.len(),
            }
            .into());
        }

        let mut data = initial;
        for val in data.iter_mut() {
            val.truncate_in_place(width as usize);
        }

        Ok(Self {
            mem_binder,
            width,
            data,
            full_name: name,
            allow_invalid_memory_access,
            update: StdMemAction::None,
        })
    }
}

impl<T: MemBinder> Named for StdMem<T> {
    fn get_full_name(&self) -> &ir::Id {
        &self.full_name
    }
}

impl<T: MemBinder> Primitive for StdMem<T> {
    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        validate_friendly![inputs;
            write_en: 1,
            write_data: self.width
        ];
        self.mem_binder.validate(inputs);
    }

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        get_inputs![inputs;
            write_en [bool]: "write_en",
            write_data: "write_data"
        ];

        let idx = self
            .mem_binder
            .get_idx(inputs, self.allow_invalid_memory_access);

        let out = match &idx {
            Ok(idx) => {
                output![(
                    "read_data",
                    if (*idx as usize) < self.data.len() {
                        self.data[*idx as usize].clone()
                    } else {
                        Value::zeroes(self.width)
                    }
                )]
            }
            Err(_) => {
                output![("read_data", Value::zeroes(self.width))]
            }
        };

        self.update = if write_en {
            StdMemAction::Write(idx, write_data.clone())
        } else {
            StdMemAction::Read(idx)
        };

        Ok(out)
    }

    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        Ok(match self.update.take() {
            StdMemAction::None => {
                output![
                    ("read_data", Value::zeroes(self.width)),
                    ("done", Value::bit_low())
                ]
            }
            StdMemAction::Read(idx) => {
                let idx = idx? as usize;
                if idx >= self.data.len() {
                    output![
                        ("read_data", Value::zeroes(self.width)),
                        ("done", Value::bit_low())
                    ]
                } else {
                    output!(
                        ("read_data", self.data[idx].clone()),
                        ("done", Value::bit_low())
                    )
                }
            }
            StdMemAction::Write(idx, v) => {
                let idx = idx? as usize;
                if idx >= self.data.len() {
                    output![("read_data", v), ("done", Value::bit_high())]
                } else {
                    self.data[idx] = v.clone();
                    output![("read_data", v), ("done", Value::bit_high())]
                }
            }
        })
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        Ok(output![
            ("read_data", Value::zeroes(self.width)),
            ("done", Value::bit_low())
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

impl StdMem<MemD1> {
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
}

impl StdMem<MemD2> {
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
}

impl StdMem<MemD3> {
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
}

impl StdMem<MemD4> {
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
}

pub type StdMemD1 = StdMem<MemD1>;
pub type StdMemD2 = StdMem<MemD2>;
pub type StdMemD3 = StdMem<MemD3>;
pub type StdMemD4 = StdMem<MemD4>;

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
    Error,
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

    fn get_array_length(&self) -> usize;
}

use super::primitive::Shape;
pub struct MemD1 {
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
        get_inputs![inputs;
            idx [u64]: "addr0"
        ];

        if idx >= self.size && !allow_invalid_memory_access {
            Err(InterpreterError::InvalidMemoryAccess {
                access: vec![idx],
                dims: vec![self.size],
                name: self.full_name.clone(),
            }
            .into())
        } else {
            Ok(idx)
        }
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        validate_friendly![inputs;
            addr0: self.idx_size
        ]
    }

    fn get_dimensions(&self) -> Shape {
        Shape::D1((self.size as usize,))
    }

    fn get_array_length(&self) -> usize {
        self.size as usize
    }
}

pub struct MemD2 {
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
            }
            .into())
        } else {
            Ok(address)
        }
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        validate_friendly![inputs;
            addr0: self.d0_idx_size,
            addr1: self.d1_idx_size
        ]
    }

    fn get_dimensions(&self) -> Shape {
        Shape::D2((self.d0_size as usize, self.d1_size as usize))
    }

    fn get_array_length(&self) -> usize {
        (self.d0_size * self.d1_size) as usize
    }
}

pub struct MemD3 {
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
            }
            .into())
        } else {
            Ok(address)
        }
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        validate_friendly![inputs;
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

    fn get_array_length(&self) -> usize {
        (self.d0_size * self.d1_size * self.d2_size) as usize
    }
}

pub struct MemD4 {
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
            }
            .into())
        } else {
            Ok(address)
        }
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        validate_friendly![inputs;
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

    fn get_array_length(&self) -> usize {
        (self.d0_size * self.d1_size * self.d2_size * self.d3_size) as usize
    }
}

/// The primitive skeleton for sequential memories. Both reads and writes take a
/// cycle. Read output is latched. Read and Write signals cannot be asserted at
/// the same time.
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
    update: SeqMemAction<InterpreterResult<u64>>,
}

impl<T: MemBinder> SeqMem<T> {
    pub fn new(
        params: &ir::Binding,
        name: ir::Id,
        allow_invalid_memory_access: bool,
    ) -> Self {
        let mem_binder = T::new(params, name.clone());
        let width =
            get_param(params, "WIDTH").expect("Missing WIDTH param for memory");

        let data =
            vec![Value::zeroes(width as usize); mem_binder.get_array_length()];

        Self {
            mem_binder,
            width,
            data,
            full_name: name,
            allow_invalid_memory_access,
            read_out: Value::zeroes(width),
            update: SeqMemAction::None,
        }
    }

    pub fn from_initial_mem(
        params: &ir::Binding,
        name: ir::Id,
        allow_invalid_memory_access: bool,
        initial: Vec<Value>,
    ) -> InterpreterResult<Self> {
        let mem_binder = T::new(params, name.clone());
        let width =
            get_param(params, "WIDTH").expect("Missing WIDTH param for memory");

        let size = mem_binder.get_array_length();

        if initial.len() != size {
            return Err(InterpreterError::IncorrectMemorySize {
                mem_dim: mem_binder.get_dimensions().dim_str(),
                expected: size as u64,
                given: initial.len(),
            }
            .into());
        }

        let mut data = initial;
        for val in data.iter_mut() {
            val.truncate_in_place(width as usize);
        }

        Ok(Self {
            mem_binder,
            width,
            data,
            full_name: name,
            allow_invalid_memory_access,
            read_out: Value::zeroes(width),
            update: SeqMemAction::None,
        })
    }
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

        let idx = self
            .mem_binder
            .get_idx(inputs, self.allow_invalid_memory_access);

        self.update = if reset {
            SeqMemAction::Reset
        } else if write_en && read_en {
            SeqMemAction::Error
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
        match self.update.take() {
            SeqMemAction::Read(idx) => {
                let idx = idx? as usize;
                if idx >= self.data.len() {
                    self.read_out = Value::zeroes(self.width)
                } else {
                    self.read_out = self.data[idx].clone()
                }

                Ok(vec![
                    ("out".into(), self.read_out.clone()),
                    ("read_done".into(), Value::bit_high()),
                    ("write_done".into(), Value::bit_low()),
                ])
            }
            SeqMemAction::Write(idx, v) => {
                let idx = idx? as usize;
                if idx < self.data.len() {
                    self.data[idx] = v;
                }

                self.read_out = Value::zeroes(self.width);

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
            SeqMemAction::Error => Err(InterpreterError::SeqMemoryError.into()),
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

pub type SeqMemD1 = SeqMem<MemD1>;
pub type SeqMemD2 = SeqMem<MemD2>;
pub type SeqMemD3 = SeqMem<MemD3>;
pub type SeqMemD4 = SeqMem<MemD4>;
