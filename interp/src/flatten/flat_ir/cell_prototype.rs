use calyx_ir::{self as cir, BoolAttr};
use smallvec::SmallVec;

use crate::{
    flatten::primitives::utils::get_params, serialization::Dimensions,
};

use super::prelude::ComponentIdx;

#[derive(Debug, Clone)]
pub enum LiteralOrPrimitive {
    Literal,
    Primitive,
}

/// An enum for encoding primitive operator types with only one width parameter
#[derive(Debug, Clone)]
pub enum SingleWidthType {
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
    Undef,
}

/// An enum for encoding FP primitives operator types
#[derive(Debug, Clone)]
pub enum FXType {
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
        d0_size: ParamWidth,
        d0_idx_size: ParamWidth,
    },
    D2 {
        d0_size: ParamWidth,
        d0_idx_size: ParamWidth,
        d1_size: ParamWidth,
        d1_idx_size: ParamWidth,
    },
    D3 {
        d0_size: ParamWidth,
        d0_idx_size: ParamWidth,
        d1_size: ParamWidth,
        d1_idx_size: ParamWidth,
        d2_size: ParamWidth,
        d2_idx_size: ParamWidth,
    },
    D4 {
        d0_size: ParamWidth,
        d0_idx_size: ParamWidth,
        d1_size: ParamWidth,
        d1_idx_size: ParamWidth,
        d2_size: ParamWidth,
        d2_idx_size: ParamWidth,
        d3_size: ParamWidth,
        d3_idx_size: ParamWidth,
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
pub type ParamWidth = u32;

#[derive(Debug, Clone)]
pub enum CellPrototype {
    Component(ComponentIdx),
    Constant {
        value: u64,
        width: ParamWidth,
        c_type: LiteralOrPrimitive,
    },
    SingleWidth {
        op: SingleWidthType,
        width: ParamWidth,
    },
    FixedPoint {
        op: FXType,
        width: ParamWidth,
        int_width: ParamWidth,
        frac_width: ParamWidth,
    },
    // The awkward three that don't fit the other patterns
    Slice {
        in_width: ParamWidth,
        out_width: ParamWidth,
    },
    Pad {
        in_width: ParamWidth,
        out_width: ParamWidth,
    },
    Cat {
        left: ParamWidth,
        right: ParamWidth,
        out: ParamWidth,
    },
    BitSlice {
        start_idx: ParamWidth,
        end_idx: ParamWidth,
        out_width: ParamWidth,
    },
    // Memories
    Memory {
        mem_type: MemType,
        width: ParamWidth,
        dims: MemoryDimensions,
        is_external: bool,
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
    pub fn construct_primitive(cell: &cir::Cell) -> Self {
        if let cir::CellType::Primitive {
            name,
            param_binding,
            ..
        } = &cell.prototype
        {
            let name: &str = name.as_ref();
            let params: &SmallVec<_> = param_binding;

            match name {
                "std_reg" => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: SingleWidthType::Reg,
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
                            SingleWidthType::Add
                        } else {
                            SingleWidthType::SignedAdd
                        },
                        width: width.try_into().unwrap(),
                    }
                }
                n @ ("std_sub" | "std_ssub") => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: if n == "std_sub" {
                            SingleWidthType::Sub
                        } else {
                            SingleWidthType::SignedSub
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
                            FXType::Add
                        } else {
                            FXType::SignedAdd
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
                            FXType::Sub
                        } else {
                            FXType::SignedSub
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
                            SingleWidthType::MultPipe
                        } else {
                            SingleWidthType::SignedMultPipe
                        },
                        width: width.try_into().unwrap(),
                    }
                }
                n @ ("std_div_pipe" | "std_sdiv_pipe") => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: if n == "std_div_pipe" {
                            SingleWidthType::DivPipe
                        } else {
                            SingleWidthType::SignedDivPipe
                        },
                        width: width.try_into().unwrap(),
                    }
                }
                "sqrt" => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: SingleWidthType::Sqrt,
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
                        op: FXType::Sqrt,
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
                            "std_fp_mult_pipe" => FXType::Mult,
                            "std_fp_smult_pipe" => FXType::SignedMult,
                            "std_fp_div_pipe" => FXType::Div,
                            _ => FXType::SignedDiv,
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
                            "std_lsh" => SingleWidthType::Lsh,
                            "std_rsh" => SingleWidthType::Rsh,
                            "std_lrsh" => SingleWidthType::SignedLsh,
                            _ => SingleWidthType::SignedRsh,
                        },
                        width: width.try_into().unwrap(),
                    }
                }
                n @ ("std_and" | "std_or" | "std_xor" | "std_not") => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: match n {
                            "std_and" => SingleWidthType::And,
                            "std_or" => SingleWidthType::Or,
                            "std_xor" => SingleWidthType::Xor,
                            _ => SingleWidthType::Not,
                        },
                        width: width.try_into().unwrap(),
                    }
                }
                "std_wire" => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: SingleWidthType::Wire,
                        width: width.try_into().unwrap(),
                    }
                }
                n @ ("std_eq" | "std_neq" | "std_lt" | "std_le" | "std_gt"
                | "std_ge") => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: match n {
                            "std_eq" => SingleWidthType::Eq,
                            "std_neq" => SingleWidthType::Neq,
                            "std_lt" => SingleWidthType::Lt,
                            "std_le" => SingleWidthType::Le,
                            "std_gt" => SingleWidthType::Gt,
                            _ => SingleWidthType::Ge,
                        },
                        width: width.try_into().unwrap(),
                    }
                }

                n @ ("std_sge" | "std_sle" | "std_sgt" | "std_slt"
                | "std_seq" | "std_sneq") => {
                    get_params![params; width: "WIDTH"];

                    Self::SingleWidth {
                        op: match n {
                            "std_sge" => SingleWidthType::SignedGe,
                            "std_sle" => SingleWidthType::SignedLe,
                            "std_sgt" => SingleWidthType::SignedGt,
                            "std_slt" => SingleWidthType::SignedLt,
                            "std_seq" => SingleWidthType::SignedEq,
                            _ => SingleWidthType::SignedNeq,
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
                            FXType::Gt
                        } else if n == "std_fp_sgt" {
                            FXType::SignedGt
                        } else {
                            FXType::SignedLt
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
                        is_external: cell
                            .get_attribute(BoolAttr::External)
                            .is_some(),
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
                        is_external: cell
                            .get_attribute(BoolAttr::External)
                            .is_some(),
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
                        is_external: cell
                            .get_attribute(BoolAttr::External)
                            .is_some(),
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
                        is_external: cell
                            .get_attribute(BoolAttr::External)
                            .is_some(),
                    }
                }
                n @ ("std_unsyn_mult" | "std_unsyn_div" | "std_unsyn_smult"
                | "std_unsyn_sdiv" | "std_unsyn_mod"
                | "std_unsyn_smod") => {
                    get_params![params; width: "WIDTH"];
                    Self::SingleWidth {
                        op: match n {
                            "std_unsyn_mult" => SingleWidthType::UnsynMult,
                            "std_unsyn_div" => SingleWidthType::UnsynDiv,
                            "std_unsyn_smult" => SingleWidthType::UnsynSMult,
                            "std_unsyn_sdiv" => SingleWidthType::UnsynSDiv,
                            "std_unsyn_mod" => SingleWidthType::UnsynMod,
                            _ => SingleWidthType::UnsynSMod,
                        },
                        width: width.try_into().unwrap(),
                    }
                }

                "undef" => {
                    get_params![params; width: "WIDTH"];
                    Self::SingleWidth {
                        op: SingleWidthType::Undef,
                        width: width.try_into().unwrap(),
                    }
                }

                "std_bit_slice" => {
                    get_params![params;
                        start_idx: "START_IDX",
                        end_idx: "END_IDX",
                        out_width: "OUT_WIDTH"
                    ];
                    Self::BitSlice {
                        start_idx: start_idx.try_into().unwrap(),
                        end_idx: end_idx.try_into().unwrap(),
                        out_width: out_width.try_into().unwrap(),
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
