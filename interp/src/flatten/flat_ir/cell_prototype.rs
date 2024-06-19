use calyx_ir::{self as cir};
use smallvec::SmallVec;

use crate::{
    primitives::prim_utils::get_params, serialization::data_dump::Dimensions,
};

use super::prelude::ComponentIdx;

#[derive(Debug, Clone)]
pub enum LiteralOrPrimitive {
    Literal,
    Primitive,
}

/// An enum for encoding primitive operator types with only one width parameter
#[derive(Debug, Clone)]
pub enum PrimType1 {
    Reg,
    //
    Not,
    And,
    Or,
    Xor,
    //
    Add,
    Sub,
    Gt,
    Lt,
    Eq,
    Neq,
    Ge,
    Le,
    //
    Lsh,
    Rsh,
    Mux,
    Wire,
    //
    SignedAdd,
    SignedSub,
    SignedGt,
    SignedLt,
    SignedEq,
    SignedNeq,
    SignedGe,
    SignedLe,
    SignedLsh,
    SignedRsh,
    MultPipe,
    SignedMultPipe,
    DivPipe,
    SignedDivPipe,
    Sqrt,
    //
    UnsynMult,
    UnsynDiv,
    UnsynMod,
    UnsynSMult,
    UnsynSDiv,
    UnsynSMod,
}

/// An enum for encoding FP primitives operator types
#[derive(Debug, Clone)]
pub enum FPType {
    Add,
    Sub,
    Mult,
    Div,
    SignedAdd,
    SignedSub,
    SignedMult,
    SignedDiv,
    Gt,
    SignedGt,
    SignedLt,
    Sqrt,
}

#[derive(Debug, Clone)]
pub enum MemType {
    Seq,
    Std,
}

#[derive(Debug, Clone)]
pub enum MemoryDimensions {
    D1 {
        d0_size: Width,
        d0_idx_size: Width,
    },
    D2 {
        d0_size: Width,
        d0_idx_size: Width,
        d1_size: Width,
        d1_idx_size: Width,
    },
    D3 {
        d0_size: Width,
        d0_idx_size: Width,
        d1_size: Width,
        d1_idx_size: Width,
        d2_size: Width,
        d2_idx_size: Width,
    },
    D4 {
        d0_size: Width,
        d0_idx_size: Width,
        d1_size: Width,
        d1_idx_size: Width,
        d2_size: Width,
        d2_idx_size: Width,
        d3_size: Width,
        d3_idx_size: Width,
    },
}

impl MemoryDimensions {
    pub fn size(&self) -> usize {
        match self {
            Self::D1 { d0_size, .. } => *d0_size as usize,
            Self::D2 {
                d0_size, d1_size, ..
            } => *d0_size as usize * *d1_size as usize,
            Self::D3 {
                d0_size,
                d1_size,
                d2_size,
                ..
            } => *d0_size as usize * *d1_size as usize * *d2_size as usize,
            Self::D4 {
                d0_size,
                d1_size,
                d2_size,
                d3_size,
                ..
            } => {
                *d0_size as usize
                    * *d1_size as usize
                    * *d2_size as usize
                    * *d3_size as usize
            }
        }
    }

    /// Returns a Dimensions object
    pub fn as_serializing_dim(&self) -> Dimensions {
        match self {
            MemoryDimensions::D1 { d0_size, .. } => {
                Dimensions::D1(*d0_size as usize)
            }
            MemoryDimensions::D2 {
                d0_size, d1_size, ..
            } => Dimensions::D2(*d0_size as usize, *d1_size as usize),
            MemoryDimensions::D3 {
                d0_size,
                d1_size,
                d2_size,
                ..
            } => Dimensions::D3(
                *d0_size as usize,
                *d1_size as usize,
                *d2_size as usize,
            ),
            MemoryDimensions::D4 {
                d0_size,
                d1_size,
                d2_size,
                d3_size,
                ..
            } => Dimensions::D4(
                *d0_size as usize,
                *d1_size as usize,
                *d2_size as usize,
                *d3_size as usize,
            ),
        }
    }
}

/// A type alias to allow potential space hacks
pub type Width = u32;

#[derive(Debug, Clone)]
pub enum CellPrototype {
    Component(ComponentIdx),
    Constant {
        value: u64,
        width: Width,
        c_type: LiteralOrPrimitive,
    },
    SingleWidth {
        op: PrimType1,
        width: Width,
    },
    FixedPoint {
        op: FPType,
        width: Width,
        int_width: Width,
        frac_width: Width,
    },
    // The awkward three that don't fit the other patterns
    Slice {
        in_width: Width,
        out_width: Width,
    },
    Pad {
        in_width: Width,
        out_width: Width,
    },
    Cat {
        left: Width,
        right: Width,
        out: Width,
    },
    // Memories
    Memory {
        mem_type: MemType,
        width: Width,
        dims: MemoryDimensions,
    },

    // TODO Griffin: lots more
    Unknown(String, Box<cir::Binding>),
}

impl From<ComponentIdx> for CellPrototype {
    fn from(v: ComponentIdx) -> Self {
        Self::Component(v)
    }
}

impl CellPrototype {
    #[must_use]
    pub fn as_component(&self) -> Option<&ComponentIdx> {
        if let Self::Component(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn construct_primitive(cell: &cir::CellType) -> Self {
        if let cir::CellType::Primitive {
            name,
            param_binding,
            ..
        } = cell
        {
            let name: &str = name.as_ref();
            let params: &SmallVec<_> = param_binding;

            match name {
                "std_reg" => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: PrimType1::Reg,
                        width: width.try_into().unwrap(),
                    }
                }
                "std_const" => {
                    get_params![params;
                        value: "VALUE",
                        width: "WIDTH"
                    ];

                    Self::Constant {
                        value,
                        width: width.try_into().unwrap(),
                        c_type: LiteralOrPrimitive::Primitive,
                    }
                }
                n @ ("std_add" | "std_sadd") => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: if n == "std_add" {
                            PrimType1::Add
                        } else {
                            PrimType1::SignedAdd
                        },
                        width: width.try_into().unwrap(),
                    }
                }
                n @ ("std_sub" | "std_ssub") => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: if n == "std_sub" {
                            PrimType1::Sub
                        } else {
                            PrimType1::SignedSub
                        },
                        width: width.try_into().unwrap(),
                    }
                }
                n @ ("std_fp_add" | "std_fp_sadd") => {
                    get_params![params;
                        width: "WIDTH",
                        int_width: "INT_WIDTH",
                        frac_width: "FRAC_WIDTH"
                    ];

                    Self::FixedPoint {
                        op: if n == "std_fp_add" {
                            FPType::Add
                        } else {
                            FPType::SignedAdd
                        },
                        width: width.try_into().unwrap(),
                        int_width: int_width.try_into().unwrap(),
                        frac_width: frac_width.try_into().unwrap(),
                    }
                }
                n @ ("std_fp_sub" | "std_fp_ssub") => {
                    get_params![params;
                        width: "WIDTH",
                        int_width: "INT_WIDTH",
                        frac_width: "FRAC_WIDTH"
                    ];

                    Self::FixedPoint {
                        op: if n == "std_fp_sub" {
                            FPType::Sub
                        } else {
                            FPType::SignedSub
                        },
                        width: width.try_into().unwrap(),
                        int_width: int_width.try_into().unwrap(),
                        frac_width: frac_width.try_into().unwrap(),
                    }
                }
                n @ ("std_mult_pipe" | "std_smult_pipe") => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: if n == "std_mult_pipe" {
                            PrimType1::MultPipe
                        } else {
                            PrimType1::SignedMultPipe
                        },
                        width: width.try_into().unwrap(),
                    }
                }
                n @ ("std_div_pipe" | "std_sdiv_pipe") => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: if n == "std_div_pipe" {
                            PrimType1::DivPipe
                        } else {
                            PrimType1::SignedDivPipe
                        },
                        width: width.try_into().unwrap(),
                    }
                }
                "sqrt" => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: PrimType1::Sqrt,
                        width: width.try_into().unwrap(),
                    }
                }
                "fp_sqrt" => {
                    get_params![params;
                        width: "WIDTH",
                        int_width: "INT_WIDTH",
                        frac_width: "FRAC_WIDTH"
                    ];

                    Self::FixedPoint {
                        op: FPType::Sqrt,
                        width: width.try_into().unwrap(),
                        int_width: int_width.try_into().unwrap(),
                        frac_width: frac_width.try_into().unwrap(),
                    }
                }

                n @ ("std_fp_mult_pipe" | "std_fp_smult_pipe"
                | "std_fp_div_pipe" | "std_fp_sdiv_pipe") => {
                    get_params![params;
                        width: "WIDTH",
                        int_width: "INT_WIDTH",
                        frac_width: "FRAC_WIDTH"
                    ];

                    Self::FixedPoint {
                        op: match n {
                            "std_fp_mult_pipe" => FPType::Mult,
                            "std_fp_smult_pipe" => FPType::SignedMult,
                            "std_fp_div_pipe" => FPType::Div,
                            _ => FPType::SignedDiv,
                        },
                        width: width.try_into().unwrap(),
                        int_width: int_width.try_into().unwrap(),
                        frac_width: frac_width.try_into().unwrap(),
                    }
                }

                n @ ("std_lsh" | "std_rsh" | "std_slsh" | "std_srsh") => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: match n {
                            "std_lsh" => PrimType1::Lsh,
                            "std_rsh" => PrimType1::Rsh,
                            "std_lrsh" => PrimType1::SignedLsh,
                            _ => PrimType1::SignedRsh,
                        },
                        width: width.try_into().unwrap(),
                    }
                }
                n @ ("std_and" | "std_or" | "std_xor" | "std_not") => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: match n {
                            "std_and" => PrimType1::And,
                            "std_or" => PrimType1::Or,
                            "std_xor" => PrimType1::Xor,
                            _ => PrimType1::Not,
                        },
                        width: width.try_into().unwrap(),
                    }
                }
                "std_wire" => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: PrimType1::Wire,
                        width: width.try_into().unwrap(),
                    }
                }
                n @ ("std_eq" | "std_neq" | "std_lt" | "std_le" | "std_gt"
                | "std_ge") => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: match n {
                            "std_eq" => PrimType1::Eq,
                            "std_neq" => PrimType1::Neq,
                            "std_lt" => PrimType1::Lt,
                            "std_le" => PrimType1::Le,
                            "std_gt" => PrimType1::Gt,
                            _ => PrimType1::Ge,
                        },
                        width: width.try_into().unwrap(),
                    }
                }

                n @ ("std_sge" | "std_sle" | "std_sgt" | "std_slt"
                | "std_seq" | "std_sneq") => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: match n {
                            "std_sge" => PrimType1::SignedGe,
                            "std_sle" => PrimType1::SignedLe,
                            "std_sgt" => PrimType1::SignedGt,
                            "std_slt" => PrimType1::SignedLt,
                            "std_seq" => PrimType1::SignedEq,
                            _ => PrimType1::SignedNeq,
                        },
                        width: width.try_into().unwrap(),
                    }
                }
                n @ ("std_fp_gt" | "std_fp_sgt" | "std_fg_slt") => {
                    get_params![params;
                        width: "WIDTH",
                        int_width: "INT_WIDTH",
                        frac_width: "FRAC_WIDTH"
                    ];

                    Self::FixedPoint {
                        op: if n == "std_fp_gt" {
                            FPType::Gt
                        } else if n == "std_fp_sgt" {
                            FPType::SignedGt
                        } else {
                            FPType::SignedLt
                        },
                        width: width.try_into().unwrap(),
                        int_width: int_width.try_into().unwrap(),
                        frac_width: frac_width.try_into().unwrap(),
                    }
                }

                "std_slice" => {
                    get_params![params;
                        in_width: "IN_WIDTH",
                        out_width: "OUT_WIDTH"
                    ];

                    Self::Slice {
                        in_width: in_width.try_into().unwrap(),
                        out_width: out_width.try_into().unwrap(),
                    }
                }
                "std_pad" => {
                    get_params![params;
                        in_width: "IN_WIDTH",
                        out_width: "OUT_WIDTH"
                    ];

                    Self::Pad {
                        in_width: in_width.try_into().unwrap(),
                        out_width: out_width.try_into().unwrap(),
                    }
                }
                "std_cat" => {
                    get_params![params;
                        left_width: "LEFT_WIDTH",
                        right_width: "RIGHT_WIDTH",
                        out_width: "OUT_WIDTH"
                    ];
                    Self::Cat {
                        left: left_width.try_into().unwrap(),
                        right: right_width.try_into().unwrap(),
                        out: out_width.try_into().unwrap(),
                    }
                }
                n @ ("comb_mem_d1" | "seq_mem_d1") => {
                    get_params![params;
                        width: "WIDTH",
                        size: "SIZE",
                        idx_size: "IDX_SIZE"
                    ];
                    Self::Memory {
                        mem_type: if n == "comb_mem_d1" {
                            MemType::Std
                        } else {
                            MemType::Seq
                        },
                        width: width.try_into().unwrap(),
                        dims: MemoryDimensions::D1 {
                            d0_size: size.try_into().unwrap(),
                            d0_idx_size: idx_size.try_into().unwrap(),
                        },
                    }
                }
                n @ ("comb_mem_d2" | "seq_mem_d2") => {
                    get_params![params;
                        width: "WIDTH",
                        d0_size: "D0_SIZE",
                        d1_size: "D1_SIZE",
                        d0_idx_size: "D0_IDX_SIZE",
                        d1_idx_size: "D1_IDX_SIZE"
                    ];
                    Self::Memory {
                        mem_type: if n == "comb_mem_d2" {
                            MemType::Std
                        } else {
                            MemType::Seq
                        },
                        width: width.try_into().unwrap(),
                        dims: MemoryDimensions::D2 {
                            d0_size: d0_size.try_into().unwrap(),
                            d1_size: d1_size.try_into().unwrap(),
                            d0_idx_size: d0_idx_size.try_into().unwrap(),
                            d1_idx_size: d1_idx_size.try_into().unwrap(),
                        },
                    }
                }
                n @ ("comb_mem_d3" | "seq_mem_d3") => {
                    get_params![params;
                        width: "WIDTH",
                        d0_size: "D0_SIZE",
                        d1_size: "D1_SIZE",
                        d2_size: "D2_SIZE",
                        d0_idx_size: "D0_IDX_SIZE",
                        d1_idx_size: "D1_IDX_SIZE",
                        d2_idx_size: "D2_IDX_SIZE"
                    ];
                    Self::Memory {
                        mem_type: if n == "comb_mem_d3" {
                            MemType::Std
                        } else {
                            MemType::Seq
                        },
                        width: width.try_into().unwrap(),
                        dims: MemoryDimensions::D3 {
                            d0_size: d0_size.try_into().unwrap(),
                            d1_size: d1_size.try_into().unwrap(),
                            d2_size: d2_size.try_into().unwrap(),
                            d0_idx_size: d0_idx_size.try_into().unwrap(),
                            d1_idx_size: d1_idx_size.try_into().unwrap(),
                            d2_idx_size: d2_idx_size.try_into().unwrap(),
                        },
                    }
                }
                n @ ("comb_mem_d4" | "seq_mem_d4") => {
                    get_params![params;
                        width: "WIDTH",
                        d0_size: "D0_SIZE",
                        d1_size: "D1_SIZE",
                        d2_size: "D2_SIZE",
                        d3_size: "D3_SIZE",
                        d0_idx_size: "D0_IDX_SIZE",
                        d1_idx_size: "D1_IDX_SIZE",
                        d2_idx_size: "D2_IDX_SIZE",
                        d3_idx_size: "D3_IDX_SIZE"
                    ];

                    Self::Memory {
                        mem_type: if n == "comb_mem_d4" {
                            MemType::Std
                        } else {
                            MemType::Seq
                        },
                        width: width.try_into().unwrap(),
                        dims: MemoryDimensions::D4 {
                            d0_size: d0_size.try_into().unwrap(),
                            d1_size: d1_size.try_into().unwrap(),
                            d2_size: d2_size.try_into().unwrap(),
                            d3_size: d3_size.try_into().unwrap(),
                            d0_idx_size: d0_idx_size.try_into().unwrap(),
                            d1_idx_size: d1_idx_size.try_into().unwrap(),
                            d2_idx_size: d2_idx_size.try_into().unwrap(),
                            d3_idx_size: d3_idx_size.try_into().unwrap(),
                        },
                    }
                }
                n @ ("std_unsyn_mult" | "std_unsyn_div" | "std_unsyn_smult"
                | "std_unsyn_sdiv" | "std_unsyn_mod"
                | "std_unsyn_smod") => {
                    get_params![params; width: "WIDTH"];
                    Self::SingleWidth {
                        op: match n {
                            "std_unsyn_mult" => PrimType1::UnsynMult,
                            "std_unsyn_div" => PrimType1::UnsynDiv,
                            "std_unsyn_smult" => PrimType1::UnsynSMult,
                            "std_unsyn_sdiv" => PrimType1::UnsynSDiv,
                            "std_unsyn_mod" => PrimType1::UnsynMod,
                            _ => PrimType1::UnsynSMod,
                        },
                        width: width.try_into().unwrap(),
                    }
                }

                _ => CellPrototype::Unknown(
                    name.to_string(),
                    param_binding.clone(),
                ),
            }
        } else {
            unreachable!("construct_primitive called on non-primitive cell");
        }
    }

    /// Returns `true` if the cell prototype is [`Component`].
    ///
    /// [`Component`]: CellPrototype::Component
    #[must_use]
    pub fn is_component(&self) -> bool {
        matches!(self, Self::Component(..))
    }
}
