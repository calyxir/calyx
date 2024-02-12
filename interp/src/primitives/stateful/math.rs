use super::super::prim_utils::{get_inputs, get_param, ShiftBuffer};
use super::super::primitive_traits::Named;
use super::super::Primitive;
use crate::errors::{InterpreterError, InterpreterResult};
use crate::logging::{self, warn};
use crate::serialization::{Entry, Serializable};
use crate::utils::PrintCode;
use crate::validate;
use crate::values::Value;
use calyx_ir as ir;
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
            logger: logging::new_sublogger(name),
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

    fn validate(&self, inputs: &[(calyx_ir::Id, &Value)]) {
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
        inputs: &[(calyx_ir::Id, &Value)],
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
        _: &[(calyx_ir::Id, &Value)],
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
            [self.product.clone()]
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
            logger: logging::new_sublogger(name),
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

    fn validate(&self, inputs: &[(calyx_ir::Id, &Value)]) {
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
        inputs: &[(calyx_ir::Id, &Value)],
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
        _: &[(calyx_ir::Id, &Value)],
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
            [self.quotient.clone(), self.remainder.clone()]
                .iter()
                .map(|x| Entry::from_val_code(x, &code))
                .collect(),
            2.into(),
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
            logger: logging::new_sublogger(full_name),
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
            logger: logging::new_sublogger(name),
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

type SqrtUpdate = super::memories::RegUpdate;

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
