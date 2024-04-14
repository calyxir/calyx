use btor2tools::{Btor2Line, Btor2SortContent, Btor2Tag};

//make our own sort

// make an enum entry for each instruction: for all instructions
// with things in common, factor out separate fields.
// check out cell_prototype.rs for an example.

pub enum SortType {
    Array { index: i64, element: i64 },
    Bitvec { width: u32 },
}

impl From<Btor2SortContent> for SortType {
    fn from(content: Btor2SortContent) -> SortType {
        match content {
            Btor2SortContent::Array { index, element } => {
                SortType::Array { index, element }
            }
            Btor2SortContent::Bitvec { width } => SortType::Bitvec { width },
        }
    }
}

pub enum ConstantType {
    Const,
    Consth,
    Constd,
}

impl From<Btor2Tag> for ConstantType {
    fn from(tag: Btor2Tag) -> ConstantType {
        match tag {
            Btor2Tag::Const => ConstantType::Const,
            Btor2Tag::Constd => ConstantType::Constd,
            Btor2Tag::Consth => ConstantType::Consth,
            _ => panic!("Unknown Constant type: this error should not occur."),
        }
    }
}

pub enum LiteralType {
    One,
    Ones,
    Zero,
}

impl From<Btor2Tag> for LiteralType {
    fn from(tag: Btor2Tag) -> LiteralType {
        match tag {
            Btor2Tag::One => LiteralType::One,
            Btor2Tag::Ones => LiteralType::Ones,
            Btor2Tag::Zero => LiteralType::Zero,
            _ => panic!("Unknown Literal type: this error should not occur."),
        }
    }
}

pub enum UnOpType {
    Sext,
    Uext,
    Not,
    Inc,
    Dec,
    Neg,
    Redand,
    Redor,
    Redxor,
}

impl From<Btor2Tag> for UnOpType {
    fn from(tag: Btor2Tag) -> UnOpType {
        match tag {
            Btor2Tag::Sext => UnOpType::Sext,
            Btor2Tag::Uext => UnOpType::Uext,
            Btor2Tag::Not => UnOpType::Not,
            Btor2Tag::Inc => UnOpType::Inc,
            Btor2Tag::Dec => UnOpType::Dec,
            Btor2Tag::Neg => UnOpType::Neg,
            Btor2Tag::Redand => UnOpType::Redand,
            Btor2Tag::Redor => UnOpType::Redor,
            Btor2Tag::Redxor => UnOpType::Redxor,
            _ => panic!("Unknown UnOp type: this error should not occur."),
        }
    }
}

pub enum BinOpType {
    Iff,
    Implies,
    Eq,
    Neq,
    Sgt,
    Sgte,
    Slt,
    Slte,
    Ugt,
    Ugte,
    Ult,
    Ulte,
    And,
    Nand,
    Nor,
    Or,
    Xor,
    Xnor,
    Rol,
    Ror,
    Sll,
    Sra,
    Srl,
    Add,
    Mul,
    Sdiv,
    Udiv,
    Smod,
    Srem,
    Urem,
    Sub,
    Saddo,
    Uaddo,
    Sdivo,
    Smulo,
    Umulo,
    Concat,
}

impl From<Btor2Tag> for BinOpType {
    fn from(tag: Btor2Tag) -> BinOpType {
        match tag {
            Btor2Tag::Iff => BinOpType::Iff,
            Btor2Tag::Implies => BinOpType::Implies,
            Btor2Tag::Eq => BinOpType::Eq,
            Btor2Tag::Neq => BinOpType::Neq,
            Btor2Tag::Sgt => BinOpType::Sgt,
            Btor2Tag::Sgte => BinOpType::Sgte,
            Btor2Tag::Slt => BinOpType::Slt,
            Btor2Tag::Slte => BinOpType::Slte,
            Btor2Tag::Ugt => BinOpType::Ugt,
            Btor2Tag::Ugte => BinOpType::Ugte,
            Btor2Tag::Ult => BinOpType::Ult,
            Btor2Tag::Ulte => BinOpType::Ulte,
            Btor2Tag::And => BinOpType::And,
            Btor2Tag::Nand => BinOpType::Nand,
            Btor2Tag::Nor => BinOpType::Nor,
            Btor2Tag::Or => BinOpType::Or,
            Btor2Tag::Xor => BinOpType::Xor,
            Btor2Tag::Xnor => BinOpType::Xnor,
            Btor2Tag::Rol => BinOpType::Rol,
            Btor2Tag::Ror => BinOpType::Ror,
            Btor2Tag::Sll => BinOpType::Sll,
            Btor2Tag::Sra => BinOpType::Sra,
            Btor2Tag::Srl => BinOpType::Srl,
            Btor2Tag::Add => BinOpType::Add,
            Btor2Tag::Mul => BinOpType::Mul,
            Btor2Tag::Sdiv => BinOpType::Sdiv,
            Btor2Tag::Udiv => BinOpType::Udiv,
            Btor2Tag::Smod => BinOpType::Smod,
            Btor2Tag::Srem => BinOpType::Srem,
            Btor2Tag::Urem => BinOpType::Urem,
            Btor2Tag::Sub => BinOpType::Sub,
            Btor2Tag::Saddo => BinOpType::Saddo,
            Btor2Tag::Uaddo => BinOpType::Uaddo,
            Btor2Tag::Sdivo => BinOpType::Sdivo,
            Btor2Tag::Smulo => BinOpType::Smulo,
            Btor2Tag::Umulo => BinOpType::Umulo,
            Btor2Tag::Concat => BinOpType::Concat,
            _ => panic!("Unknown BinOp type: this error should not occur."),
        }
    }
}

// pub struct Btor2InstrSort {
//     pub id: i64,
//     pub tag: Btor2SortTag,
//     pub name: String,
// }

// pub struct Btor2Instr {
//     pub id: i64,
//     pub linenum: u64,
//     pub name: String,
//     pub tag: Btor2Tag,
//     pub sort: Btor2InstrSort,
//     pub init: i64,
//     pub next: u64,
//     pub constant: String,
//     pub symbol: String,
//     pub nargs: u32,
//     pub margs: u32,
//     pub args: Vec<i64>,
// }

pub enum Btor2InstrContents {
    Constant {
        constant: Option<String>,
        kind: ConstantType,
    },
    Literal {
        kind: LiteralType,
    },
    UnOp {
        arg1: i64,
        kind: UnOpType,
    },
    BinOp {
        arg1: i64,
        arg2: i64,
        kind: BinOpType,
    },
    Conditional {
        arg1: i64,
        arg2: i64,
        arg3: i64,
    },
    Slice {
        arg1: i64,
        u: i64,
        l: i64,
    },
    Input {
        name: String,
    },
    Output {
        name: String,
        arg1: i64,
    },
    Sort,
    Unknown, // TODO: this is very janky but will remove once we add interpreter support for state
}

pub struct Btor2Instr {
    pub id: i64,
    pub sort: SortType,
    pub contents: Btor2InstrContents,
}

impl From<&Btor2Line<'_>> for Btor2Instr {
    fn from(line: &Btor2Line) -> Btor2Instr {
        let id = line.id();
        let sort = SortType::from(line.sort().content());
        // eprintln!("{:?}", line);
        match line.tag() {
            // core
            btor2tools::Btor2Tag::Sort => Btor2Instr {
                id,
                sort,
                contents: Btor2InstrContents::Sort,
            }, // skip - sort information is handled by the parser
            btor2tools::Btor2Tag::Const => convert_const_op(line),
            btor2tools::Btor2Tag::Constd => convert_const_op(line),
            btor2tools::Btor2Tag::Consth => convert_const_op(line),
            btor2tools::Btor2Tag::Input => convert_input(line), // handled in parse_inputs
            btor2tools::Btor2Tag::Output => convert_output(line), // handled in extract_output
            btor2tools::Btor2Tag::One => convert_literal_op(line),
            btor2tools::Btor2Tag::Ones => convert_literal_op(line),
            btor2tools::Btor2Tag::Zero => convert_literal_op(line),

            // indexed
            btor2tools::Btor2Tag::Sext => convert_unary_op(line),
            btor2tools::Btor2Tag::Uext => convert_unary_op(line),
            btor2tools::Btor2Tag::Slice => convert_slice_op(line),

            // unary
            btor2tools::Btor2Tag::Not => convert_unary_op(line),
            btor2tools::Btor2Tag::Inc => convert_unary_op(line),
            btor2tools::Btor2Tag::Dec => convert_unary_op(line),
            btor2tools::Btor2Tag::Neg => convert_unary_op(line),
            btor2tools::Btor2Tag::Redand => convert_unary_op(line),
            btor2tools::Btor2Tag::Redor => convert_unary_op(line),
            btor2tools::Btor2Tag::Redxor => convert_unary_op(line),

            // binary - boolean
            btor2tools::Btor2Tag::Iff => convert_binary_op(line),
            btor2tools::Btor2Tag::Implies => convert_binary_op(line),
            btor2tools::Btor2Tag::Eq => convert_binary_op(line),
            btor2tools::Btor2Tag::Neq => convert_binary_op(line),

            // binary - (un)signed inequality
            btor2tools::Btor2Tag::Sgt => convert_binary_op(line),
            btor2tools::Btor2Tag::Sgte => convert_binary_op(line),
            btor2tools::Btor2Tag::Slt => convert_binary_op(line),
            btor2tools::Btor2Tag::Slte => convert_binary_op(line),
            btor2tools::Btor2Tag::Ugt => convert_binary_op(line),
            btor2tools::Btor2Tag::Ugte => convert_binary_op(line),
            btor2tools::Btor2Tag::Ult => convert_binary_op(line),
            btor2tools::Btor2Tag::Ulte => convert_binary_op(line),

            // binary - bit-wise
            btor2tools::Btor2Tag::And => convert_binary_op(line),
            btor2tools::Btor2Tag::Nand => convert_binary_op(line),
            btor2tools::Btor2Tag::Nor => convert_binary_op(line),

            btor2tools::Btor2Tag::Or => convert_binary_op(line),

            btor2tools::Btor2Tag::Xnor => convert_binary_op(line),

            btor2tools::Btor2Tag::Xor => convert_binary_op(line),

            // binary - rotate, shift
            btor2tools::Btor2Tag::Rol => convert_binary_op(line),

            btor2tools::Btor2Tag::Ror => convert_binary_op(line),

            btor2tools::Btor2Tag::Sll => convert_binary_op(line),

            btor2tools::Btor2Tag::Sra => convert_binary_op(line),

            btor2tools::Btor2Tag::Srl => convert_binary_op(line),

            // binary - arithmetic
            btor2tools::Btor2Tag::Add => convert_binary_op(line),

            btor2tools::Btor2Tag::Mul => convert_binary_op(line),

            btor2tools::Btor2Tag::Sdiv => convert_binary_op(line),

            btor2tools::Btor2Tag::Udiv => convert_binary_op(line),

            btor2tools::Btor2Tag::Smod => convert_binary_op(line),

            btor2tools::Btor2Tag::Srem => convert_binary_op(line),

            btor2tools::Btor2Tag::Urem => convert_binary_op(line),

            btor2tools::Btor2Tag::Sub => convert_binary_op(line),

            // binary - overflow
            btor2tools::Btor2Tag::Saddo => convert_binary_op(line),

            btor2tools::Btor2Tag::Uaddo => convert_binary_op(line),

            btor2tools::Btor2Tag::Sdivo => convert_binary_op(line),

            // btor2tools::Btor2Tag::Udivo => Ok(()),    Unsigned division never overflows :D
            btor2tools::Btor2Tag::Smulo => convert_binary_op(line),

            btor2tools::Btor2Tag::Umulo => convert_binary_op(line),

            btor2tools::Btor2Tag::Ssubo => convert_binary_op(line),

            btor2tools::Btor2Tag::Usubo => convert_binary_op(line),

            // binary - concat
            btor2tools::Btor2Tag::Concat => convert_binary_op(line),

            // ternary - conditional
            btor2tools::Btor2Tag::Ite => convert_conditional_op(line),

            // Unsupported: arrays, state, assertions
            btor2tools::Btor2Tag::Bad
            | btor2tools::Btor2Tag::Constraint
            | btor2tools::Btor2Tag::Fair
            | btor2tools::Btor2Tag::Init
            | btor2tools::Btor2Tag::Justice
            | btor2tools::Btor2Tag::Next
            | btor2tools::Btor2Tag::State
            | btor2tools::Btor2Tag::Read
            | btor2tools::Btor2Tag::Write => Btor2Instr {
                id,
                sort,
                contents: Btor2InstrContents::Unknown,
            },
        }
    }
}

pub fn convert_to_ir(btor2_lines: Vec<Btor2Line<'_>>) -> Vec<Btor2Instr> {
    btor2_lines.iter().map(Btor2Instr::from).collect()
}

fn convert_const_op(line: &Btor2Line) -> Btor2Instr {
    let nstr = match line.constant() {
        None => None,
        Some(cstr) => match cstr.to_str() {
            Ok(str) => Some(str.to_string()),
            Err(_) => None,
        },
    };
    Btor2Instr {
        id: line.id(),
        sort: SortType::from(line.sort().content()),
        contents: Btor2InstrContents::Constant {
            constant: nstr,
            kind: ConstantType::from(line.tag()),
        },
    }
}

fn convert_literal_op(line: &Btor2Line) -> Btor2Instr {
    Btor2Instr {
        id: line.id(),
        sort: SortType::from(line.sort().content()),
        contents: Btor2InstrContents::Literal {
            kind: LiteralType::from(line.tag()),
        },
    }
}

fn convert_unary_op(line: &Btor2Line) -> Btor2Instr {
    // eprintln!("{:?}", line);
    assert_eq!(line.args().len(), 1);
    Btor2Instr {
        id: line.id(),
        sort: SortType::from(line.sort().content()),
        contents: Btor2InstrContents::UnOp {
            arg1: line.args()[0],
            kind: UnOpType::from(line.tag()),
        },
    }
}

fn convert_binary_op(line: &Btor2Line) -> Btor2Instr {
    assert_eq!(line.args().len(), 2);
    Btor2Instr {
        id: line.id(),
        sort: SortType::from(line.sort().content()),
        contents: Btor2InstrContents::BinOp {
            arg1: line.args()[0],
            arg2: line.args()[1],
            kind: BinOpType::from(line.tag()),
        },
    }
}

fn convert_conditional_op(line: &Btor2Line) -> Btor2Instr {
    assert_eq!(line.args().len(), 3);
    Btor2Instr {
        id: line.id(),
        sort: SortType::from(line.sort().content()),
        contents: Btor2InstrContents::Conditional {
            arg1: line.args()[0],
            arg2: line.args()[1],
            arg3: line.args()[2],
        },
    }
}

fn convert_slice_op(line: &Btor2Line) -> Btor2Instr {
    // eprintln!("{:?}", line);
    assert_eq!(line.args().len(), 3);
    Btor2Instr {
        id: line.id(),
        sort: SortType::from(line.sort().content()),
        contents: Btor2InstrContents::Slice {
            arg1: line.args()[0],
            u: line.args()[1],
            l: line.args()[2],
        },
    }
}

fn convert_input(line: &Btor2Line) -> Btor2Instr {
    Btor2Instr {
        id: line.id(),
        sort: SortType::from(line.sort().content()),
        contents: Btor2InstrContents::Input {
            name: line.symbol().unwrap().to_string_lossy().into_owned(),
        },
    }
}

fn convert_output(line: &Btor2Line) -> Btor2Instr {
    assert_eq!(line.args().len(), 1);
    Btor2Instr {
        id: line.id(),
        sort: SortType::from(line.sort().content()),
        contents: Btor2InstrContents::Output {
            name: line.symbol().unwrap().to_string_lossy().into_owned(),
            arg1: line.args()[0],
        },
    }
}
