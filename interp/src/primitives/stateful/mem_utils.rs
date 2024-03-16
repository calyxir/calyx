use calyx_ir as ir;

use crate::{
    errors::{InterpreterError, InterpreterResult},
    primitives::prim_utils::{get_inputs, get_params},
    serialization::Shape,
    validate_friendly,
    values::Value,
};

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
                name: self.full_name,
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
        Shape::D1(self.size as usize)
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
                name: self.full_name,
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
        (self.d0_size as usize, self.d1_size as usize).into()
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
                name: self.full_name,
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
        (
            self.d0_size as usize,
            self.d1_size as usize,
            self.d2_size as usize,
        )
            .into()
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
                name: self.full_name,
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
        (
            self.d0_size as usize,
            self.d1_size as usize,
            self.d2_size as usize,
            self.d3_size as usize,
        )
            .into()
    }

    fn get_array_length(&self) -> usize {
        (self.d0_size * self.d1_size * self.d2_size * self.d3_size) as usize
    }
}
