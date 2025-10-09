use calyx_ir::{self as cir, BoolAttr};
use smallvec::SmallVec;

use crate::{
    flatten::primitives::utils::get_params, serialization::Dimensions,
};

use super::prelude::ComponentIdx;

/// Whether a constant is a literal or a primitive
#[derive(Debug, Clone)]
pub enum ConstantType {
    /// A literal constant
    Literal,
    /// A use of the primitive `std_const`
    Primitive,
}

/// An enum for encoding primitive operator types with only one width parameter
#[derive(Debug, Clone)]
pub enum SingleWidthType {
    /// A register (`std_reg`)
    Reg,
    /// Bitwise not (`std_not`)
    Not,
    /// Bitwise and (`std_and`)
    And,
    /// Bitwise or (`std_or`)
    Or,
    /// Bitwise xor (`std_xor`)
    Xor,
    /// Addition (`std_add`)
    Add,
    /// Subtraction (`std_sub`)
    Sub,
    /// Greater than (`std_gt`)
    Gt,
    /// Less than (`std_lt`)
    Lt,
    /// Equality (`std_eq`)
    Eq,
    /// Inequality (`std_neq`)
    Neq,
    /// Greater than or equal to (`std_ge`)
    Ge,
    /// Less than or equal to (`std_le`)
    Le,
    /// Left shift (`std_lsh`)
    Lsh,
    /// Right shift (`std_rsh`)
    Rsh,
    /// Multiplexer (`std_mux`)
    Mux,
    /// Wire (`std_wire`)
    Wire,
    /// Signed addition (`std_sadd`)
    SignedAdd,
    /// Signed subtraction (`std_ssub`)
    SignedSub,
    /// Signed greater than (`std_sgt`)
    SignedGt,
    /// Signed less than (`std_slt`)
    SignedLt,
    /// Signed equality (`std_seq`)
    SignedEq,
    /// Signed inequality (`std_sneq`)
    SignedNeq,
    /// Signed greater than or equal to (`std_sge`)
    SignedGe,
    /// Signed less than or equal to (`std_sle`)
    SignedLe,
    /// Signed left shift (`std_slsh`)
    SignedLsh,
    /// Signed right shift (`std_srsh`)
    SignedRsh,
    /// Multiplication pipe (`std_mult_pipe`)
    MultPipe,
    /// Signed multiplication pipe (`std_signed_mult_pipe`)
    SignedMultPipe,
    /// Division pipe (`std_div_pipe`)
    DivPipe,
    /// Signed division pipe (`std_signed_div_pipe`)
    SignedDivPipe,
    /// Square root (`std_sqrt`)
    Sqrt,
    /// Unsynthesizeable multiplication (`std_unsyn_mult`)
    UnsynMult,
    /// Unsynthesizeable division (`std_unsyn_div`)
    UnsynDiv,
    /// Unsynthesizeable mod (`std_unsyn_mod`)
    UnsynMod,
    /// Unsynthesizeable signed multiplication (`std_unsyn_smult`)
    UnsynSMult,
    /// Unsynthesizeable signed division (`std_unsyn_sdiv`)
    UnsynSDiv,
    /// Unsynthesizeable signed mod (`std_unsyn_smod`)
    UnsynSMod,
    /// Unsynthesizable assertion (`assert`)
    UnsynAssert,
    /// Represents the `undef` primitive. Not to be confused with undefined
    /// port values during simulation.
    Undef,
}

/// An enum for encoding primitive operator types with two width parameters
#[derive(Debug, Clone)]
pub enum DoubleWidthType {
    /// 1: input width, 2: output width
    Slice,
    /// 1: input width, 2: output width
    Pad,
}

/// An enum for encoding primitive operator types with three width parameters
#[derive(Debug, Clone)]
pub enum TripleWidthType {
    /// 1: left width, 2: right width, 3: output width
    Cat,
    /// 1: start index, 2: end index, 3: output width
    BitSlice,
}

/// An enum for encoding FP primitives operator types
#[derive(Debug, Clone)]
pub enum FXType {
    /// Addition (`std_fp_add`)
    Add,
    /// Subtraction (`std_fp_sub`)
    Sub,
    /// Multiplication (`std_fp_mult`)
    Mult,
    /// Division (`std_fp_div`)
    Div,
    /// Signed addition (`std_fp_sadd`)
    SignedAdd,
    /// Signed subtraction (`std_fp_ssub`)
    SignedSub,
    /// Signed multiplication (`std_fp_smult`)
    SignedMult,
    /// Signed division (`std_fp_sdiv`)
    SignedDiv,
    /// Greater than (`std_fp_gt`)
    Gt,
    /// Signed greater than (`std_fp_sgt`)
    SignedGt,
    /// Signed less than (`std_fp_slt`)
    SignedLt,
    /// Square root (`std_fp_sqrt`)
    Sqrt,
}

/// An enum for encoding memory primitives operator types
#[derive(Debug, Clone, PartialEq)]
pub enum MemType {
    /// Sequential memory (`seq_mem_dX`)
    Seq,
    /// Combinational memory (`comb_mem_dX`)
    Std,
}

/// The dimensions of a memory primitive
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryDimensions {
    /// 1-dimensional memory
    D1 {
        /// Size of the first dimension
        d0_size: ParamWidth,
        /// Size of the first index
        d0_idx_size: ParamWidth,
    },
    /// 2-dimensional memory
    D2 {
        /// Size of the first dimension
        d0_size: ParamWidth,
        /// Size of the first index
        d0_idx_size: ParamWidth,
        /// Size of the second dimension
        d1_size: ParamWidth,
        /// Size of the second index
        d1_idx_size: ParamWidth,
    },
    /// 3-dimensional memory
    D3 {
        /// Size of the first dimension
        d0_size: ParamWidth,
        /// Size of the first index
        d0_idx_size: ParamWidth,
        /// Size of the second dimension
        d1_size: ParamWidth,
        /// Size of the second index
        d1_idx_size: ParamWidth,
        /// Size of the third dimension
        d2_size: ParamWidth,
        /// Size of the third index
        d2_idx_size: ParamWidth,
    },
    /// 4-dimensional memory
    D4 {
        /// Size of the first dimension
        d0_size: ParamWidth,
        /// Size of the first index
        d0_idx_size: ParamWidth,
        /// Size of the second dimension
        d1_size: ParamWidth,
        /// Size of the second index
        d1_idx_size: ParamWidth,
        /// Size of the third dimension
        d2_size: ParamWidth,
        /// Size of the third index
        d2_idx_size: ParamWidth,
        /// Size of the fourth dimension
        d3_size: ParamWidth,
        /// Size of the fourth index
        d3_idx_size: ParamWidth,
    },
}

impl MemoryDimensions {
    /// Returns the total number of entries in the memory
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

#[derive(Debug, Clone, PartialEq)]
/// This cell is a memory primitive. Either a combinational or sequential memory.
pub struct MemoryPrototype {
    /// The type of memory
    pub mem_type: MemType,
    /// The width of the values in the memory
    pub width: ParamWidth,
    /// The dimensions of the memory
    pub dims: MemoryDimensions,
    /// Is the memory external?
    pub is_external: bool,
}

impl MemoryPrototype {
    /// Checks equality between two memory prototypes but excludes the is
    /// external flag
    pub fn eq_minus_external(&self, other: &Self) -> bool {
        self.mem_type == other.mem_type
            && self.width == other.width
            && self.dims == other.dims
    }
}

/// Represents the type of a Calyx cell and contains its definition information
#[derive(Debug, Clone)]
pub enum CellPrototype {
    /// This cell is an instance of a Calyx component
    Component(ComponentIdx),
    /// This cell is a constant. Either constant literal or use of the primitive `std_const`
    Constant {
        /// The value of the constant
        value: u64,
        /// The width of the value
        width: ParamWidth,
        /// Whether the constant is a literal or a primitive
        c_type: ConstantType,
    },
    /// This cell is a primitive type that only has a single width parameter.
    /// See [`SingleWidthType`] for the list of primitives.
    SingleWidth {
        /// The operator
        op: SingleWidthType,
        /// The width parameter of the operator
        width: ParamWidth,
    },
    /// This cell is a primitive type that has two width parameters.
    /// See [`DoubleWidthType`] for the list of primitives.
    DoubleWidth {
        /// The operator
        op: DoubleWidthType,
        /// The first width parameter of the operator
        width1: ParamWidth,
        /// The second width parameter of the operator
        width2: ParamWidth,
    },
    /// This cell is a primitive type that has three width parameters.
    /// See [`TripleWidthType`] for the list of primitives.
    TripleWidth {
        /// The operator
        op: TripleWidthType,
        /// The first width parameter of the operator
        width1: ParamWidth,
        /// The second width parameter of the operator
        width2: ParamWidth,
        /// The third width parameter of the operator
        width3: ParamWidth,
    },
    /// This cell is a fixed point primitive. See [`FXType`] for the list of primitives.
    FixedPoint {
        /// Fixed point operator
        op: FXType,
        // TODO griffin: Consider deleting width
        /// The width of the fixed point
        width: ParamWidth,
        /// The width of the integer part
        int_width: ParamWidth,
        /// The width of the fractional part
        frac_width: ParamWidth,
    },
    Memory(MemoryPrototype),

    /// This cell is a primitive that lacks an implementation in Cider. Its name
    /// and parameter bindings are stored for use in error messages.
    Unknown(String, Box<cir::Binding>),
}

impl From<ComponentIdx> for CellPrototype {
    fn from(v: ComponentIdx) -> Self {
        Self::Component(v)
    }
}

impl CellPrototype {
    /// Returns the component index if this is a component otherwise `None`
    #[must_use]
    pub fn as_component(&self) -> Option<&ComponentIdx> {
        if let Self::Component(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Constructs a prototype for the given cell
    #[must_use]
    pub fn construct_prototype(cell: &cir::Cell) -> Self {
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
                        c_type: ConstantType::Primitive,
                    }
                }
                "std_float_const" => {
                    get_params![params;
                        value: "VALUE",
                        width: "WIDTH",
                        rep: "REP"
                    ];

                    debug_assert_eq!(
                        rep, 0,
                        "Only supported floating point representation is IEEE."
                    );
                    debug_assert!(
                        width == 32 || width == 64,
                        "Only 32 and 64 bit floats are supported."
                    );

                    // we can treat floating point constants like any other constant since the
                    // frontend already converts the number to bits for us
                    Self::Constant {
                        value,
                        width: width.try_into().unwrap(),
                        c_type: ConstantType::Primitive,
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

                    Self::DoubleWidth {
                        op: DoubleWidthType::Slice,
                        width1: in_width.try_into().unwrap(),
                        width2: out_width.try_into().unwrap(),
                    }
                }
                "std_pad" => {
                    get_params![params;
                        in_width: "IN_WIDTH",
                        out_width: "OUT_WIDTH"
                    ];

                    Self::DoubleWidth {
                        op: DoubleWidthType::Pad,
                        width1: in_width.try_into().unwrap(),
                        width2: out_width.try_into().unwrap(),
                    }
                }
                "std_cat" => {
                    get_params![params;
                        left_width: "LEFT_WIDTH",
                        right_width: "RIGHT_WIDTH",
                        out_width: "OUT_WIDTH"
                    ];
                    Self::TripleWidth {
                        op: TripleWidthType::Cat,
                        width1: left_width.try_into().unwrap(),
                        width2: right_width.try_into().unwrap(),
                        width3: out_width.try_into().unwrap(),
                    }
                }
                n @ ("comb_mem_d1" | "seq_mem_d1") => {
                    get_params![params;
                        width: "WIDTH",
                        size: "SIZE",
                        idx_size: "IDX_SIZE"
                    ];
                    Self::Memory(MemoryPrototype {
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
                    })
                }
                n @ ("comb_mem_d2" | "seq_mem_d2") => {
                    get_params![params;
                        width: "WIDTH",
                        d0_size: "D0_SIZE",
                        d1_size: "D1_SIZE",
                        d0_idx_size: "D0_IDX_SIZE",
                        d1_idx_size: "D1_IDX_SIZE"
                    ];
                    Self::Memory(MemoryPrototype {
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
                    })
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
                    Self::Memory(MemoryPrototype {
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
                    })
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

                    Self::Memory(MemoryPrototype {
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
                    })
                }
                n @ ("std_unsyn_mult" | "std_unsyn_div" | "std_unsyn_smult"
                | "std_unsyn_sdiv" | "std_unsyn_mod" | "std_assert"
                | "std_unsyn_smod") => {
                    get_params![params; width: "WIDTH"];
                    Self::SingleWidth {
                        op: match n {
                            "std_unsyn_mult" => SingleWidthType::UnsynMult,
                            "std_unsyn_div" => SingleWidthType::UnsynDiv,
                            "std_unsyn_smult" => SingleWidthType::UnsynSMult,
                            "std_unsyn_sdiv" => SingleWidthType::UnsynSDiv,
                            "std_unsyn_mod" => SingleWidthType::UnsynMod,
                            "std_assert" => SingleWidthType::UnsynAssert,
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
                    Self::TripleWidth {
                        op: TripleWidthType::BitSlice,
                        width1: start_idx.try_into().unwrap(),
                        width2: end_idx.try_into().unwrap(),
                        width3: out_width.try_into().unwrap(),
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

    /// Returns `true` if the cell prototype is a [`Constant`] constructed from
    /// a literal.
    ///
    /// [`Constant`]: CellPrototype::Constant
    pub fn is_literal_constant(&self) -> bool {
        matches!(
            self,
            Self::Constant {
                c_type: ConstantType::Literal,
                ..
            }
        )
    }

    #[must_use]
    pub fn as_memory(&self) -> Option<&MemoryPrototype> {
        if let Self::Memory(v) = self {
            Some(v)
        } else {
            None
        }
    }
}
