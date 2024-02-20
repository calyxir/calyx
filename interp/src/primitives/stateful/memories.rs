use super::mem_utils::{MemBinder, MemD1, MemD2, MemD3, MemD4};
use crate::{
    debugger::PrintCode,
    errors::{InterpreterError, InterpreterResult},
    primitives::{
        prim_utils::{get_inputs, get_param, output},
        Named, Primitive,
    },
    serialization::{Entry, Serializable},
    utils::construct_bindings,
    validate, validate_friendly,
    values::Value,
};
use calyx_ir as ir;

pub(super) enum RegUpdate {
    None,
    Reset,
    Value(Value),
}

impl RegUpdate {
    pub(super) fn clear(&mut self) {
        *self = Self::None;
    }

    pub(super) fn take(&mut self) -> Self {
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

    fn validate(&self, inputs: &[(calyx_ir::Id, &Value)]) {
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
        inputs: &[(calyx_ir::Id, &Value)],
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
        _: &[(calyx_ir::Id, &Value)],
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
        let mem_binder = T::new(params, name);
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
        let mem_binder = T::new(params, name);
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
        let bindings = construct_bindings([
            ("WIDTH", width),
            ("SIZE", size),
            ("IDX_SIZE", idx_size),
        ]);
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
        let bindings = construct_bindings([
            ("WIDTH", width),
            ("D0_SIZE", d0_size),
            ("D1_SIZE", d1_size),
            ("D0_IDX_SIZE", d0_idx_size),
            ("D1_IDX_SIZE", d1_idx_size),
        ]);
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
        let bindings = construct_bindings([
            ("WIDTH", width),
            ("D0_SIZE", d0_size),
            ("D1_SIZE", d1_size),
            ("D2_SIZE", d2_size),
            ("D0_IDX_SIZE", d0_idx_size),
            ("D1_IDX_SIZE", d1_idx_size),
            ("D2_IDX_SIZE", d2_idx_size),
        ]);
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
        let bindings = construct_bindings([
            ("WIDTH", width),
            ("D0_SIZE", d0_size),
            ("D1_SIZE", d1_size),
            ("D2_SIZE", d2_size),
            ("D3_SIZE", d3_size),
            ("D0_IDX_SIZE", d0_idx_size),
            ("D1_IDX_SIZE", d1_idx_size),
            ("D2_IDX_SIZE", d2_idx_size),
            ("D3_IDX_SIZE", d3_idx_size),
        ]);
        Self::new(&bindings, full_name, false)
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
        let mem_binder = T::new(params, name);
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
        let mem_binder = T::new(params, name);
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
            content_en: 1,
            write_en: 1,
            reset: 1,
            write_data: self.width
        ];
        self.mem_binder.validate(inputs);
    }

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        get_inputs![inputs;
            content_en [bool]: "content_en",
            write_en [bool]: "write_en",
            reset [bool]: "reset",
            input: "write_data"
        ];

        let idx = self
            .mem_binder
            .get_idx(inputs, self.allow_invalid_memory_access);

        self.update = if reset {
            SeqMemAction::Reset
        } else if write_en && content_en {
            SeqMemAction::Write(idx, input.clone())
        } else if content_en {
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
                    ("read_data".into(), self.read_out.clone()),
                    ("done".into(), Value::bit_high()),
                ])
            }
            SeqMemAction::Write(idx, v) => {
                let idx = idx? as usize;
                if idx < self.data.len() {
                    self.data[idx] = v;
                }

                self.read_out = Value::zeroes(self.width);

                Ok(vec![
                    ("read_data".into(), self.read_out.clone()),
                    ("done".into(), Value::bit_high()),
                ])
            }
            SeqMemAction::Reset => {
                self.read_out = Value::zeroes(self.width);
                Ok(vec![
                    ("read_data".into(), self.read_out.clone()),
                    ("done".into(), Value::bit_low()),
                ])
            }
            SeqMemAction::None => Ok(vec![
                ("read_data".into(), self.read_out.clone()),
                ("done".into(), Value::bit_low()),
            ]),
        }
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        self.update.clear();

        Ok(vec![
            ("read_data".into(), self.read_out.clone()),
            ("done".into(), Value::bit_low()),
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
