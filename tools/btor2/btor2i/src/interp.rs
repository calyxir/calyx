use crate::bvec::BitVector;
use crate::error;
use crate::error::InterpError;
use crate::ir::{
    BinOpType, Btor2Instr, Btor2InstrContents, ConstantType, LiteralType,
    SortType, UnOpType,
};
use crate::shared_env::SharedEnvironment;

use num_bigint::BigInt;
use num_traits::Num;
use std::collections::HashMap;
use std::fmt;
use std::slice::Iter;
use std::vec;

// TODO: eventually remove pub and make a seperate pub function as a main entry point to the interpreter, for now this is main.rs
#[derive(Debug)]
pub struct Environment {
    // Maps sid/nid to value
    // TODO: valid programs should not have the same identifier in both sets, but we don't currently check that
    // TODO: perhaps could opportunistically free mappings if we know they won't be used again
    // TODO: consider indirect mapping of output string -> id in env
    env: Vec<Value>,
    args: HashMap<String, usize>,
    output: HashMap<String, Value>,
}

impl Environment {
    pub fn new(size: usize) -> Self {
        Self {
            // Allocate a larger stack size so the interpreter needs to allocate less often
            env: vec![Value::default(); size],
            args: HashMap::new(),
            output: HashMap::new(),
        }
    }

    pub fn get(&self, idx: usize) -> &Value {
        // A BTOR2 program is well formed when, dynamically, every variable is defined before its use.
        // If this is violated, this will return Value::Uninitialized and the whole interpreter will come crashing down.
        self.env.get(idx).unwrap()
    }

    pub fn set(&mut self, idx: usize, val: Value) {
        self.env[idx] = val;
    }

    pub fn get_output(&self) -> &HashMap<String, Value> {
        &self.output
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // iterate over self.args in order and print them

        writeln!(f, "Arguments:")?;
        let mut sorted_args = self.args.iter().collect::<Vec<_>>();
        sorted_args.sort_by(|(name1, _), (name2, _)| name1.cmp(name2));
        sorted_args.iter().try_for_each(|(name, val)| {
            writeln!(f, "{}: {}", name, val)?;
            Ok(())
        })?;

        write!(f, "\nEnvironment:\n")?;

        // don't print uninitialized values
        self.env.iter().enumerate().try_for_each(|(idx, val)| {
            writeln!(f, "{}: {}", idx, val)?;
            Ok(())
        })?;

        write!(f, "\nOutput:\n")?;
        self.output.iter().try_for_each(|(name, val)| {
            writeln!(f, "{}: {}", name, val)?;
            Ok(())
        })?;

        Ok(())
    }
}

// TODO: eventually remove pub and make a seperate pub function as a main entry point to the interpreter, for now this is main.rs
#[derive(Debug, Default, Clone)]
pub enum Value {
    BitVector(BitVector),
    // TODO: Add support for <STATE>
    // TODO: Add support for <ARRAY>
    #[default]
    Uninitialized,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::BitVector(bv) => write!(f, "{}", bv.to_usize()),
            Value::Uninitialized => write!(f, "_"),
        }
    }
}

fn get_binary_fn(
    kind: &BinOpType,
) -> fn(&mut SharedEnvironment, usize, usize, usize) {
    match kind {
        BinOpType::Add => SharedEnvironment::add,
        BinOpType::And => SharedEnvironment::and,
        BinOpType::Concat => SharedEnvironment::concat,
        BinOpType::Eq => SharedEnvironment::eq,
        BinOpType::Iff => SharedEnvironment::iff,
        BinOpType::Implies => SharedEnvironment::implies,
        BinOpType::Mul => SharedEnvironment::mul,
        BinOpType::Nand => SharedEnvironment::nand,
        BinOpType::Neq => SharedEnvironment::neq,
        BinOpType::Nor => SharedEnvironment::nor,
        BinOpType::Or => SharedEnvironment::or,
        BinOpType::Rol => SharedEnvironment::rol,
        BinOpType::Ror => SharedEnvironment::ror,
        BinOpType::Saddo => SharedEnvironment::saddo,
        BinOpType::Sdiv => SharedEnvironment::sdiv,
        BinOpType::Sdivo => SharedEnvironment::sdivo,
        BinOpType::Sgt => SharedEnvironment::sgt,
        BinOpType::Sgte => SharedEnvironment::sgte,
        BinOpType::Sll => SharedEnvironment::sll,
        BinOpType::Slt => SharedEnvironment::slt,
        BinOpType::Slte => SharedEnvironment::slte,
        BinOpType::Smod => SharedEnvironment::smod,
        BinOpType::Smulo => SharedEnvironment::smulo,
        BinOpType::Sra => SharedEnvironment::sra,
        BinOpType::Srem => SharedEnvironment::srem,
        BinOpType::Srl => SharedEnvironment::srl,
        BinOpType::Sub => SharedEnvironment::sub,
        BinOpType::Uaddo => SharedEnvironment::uaddo,
        BinOpType::Udiv => SharedEnvironment::udiv,
        BinOpType::Ugt => SharedEnvironment::ugt,
        BinOpType::Ugte => SharedEnvironment::ugte,
        BinOpType::Ult => SharedEnvironment::ult,
        BinOpType::Ulte => SharedEnvironment::ulte,
        BinOpType::Umulo => SharedEnvironment::umulo,
        BinOpType::Urem => SharedEnvironment::urem,
        BinOpType::Xnor => SharedEnvironment::xnor,
        BinOpType::Xor => SharedEnvironment::xor,
    }
}

fn get_unary_fn(kind: &UnOpType) -> fn(&mut SharedEnvironment, usize, usize) {
    match kind {
        UnOpType::Dec => SharedEnvironment::dec,
        UnOpType::Inc => SharedEnvironment::inc,
        UnOpType::Neg => SharedEnvironment::neg,
        UnOpType::Not => SharedEnvironment::not,
        UnOpType::Redand => SharedEnvironment::redand,
        UnOpType::Redor => SharedEnvironment::redor,
        UnOpType::Redxor => SharedEnvironment::redxor,
        UnOpType::Sext => SharedEnvironment::sext,
        UnOpType::Uext => SharedEnvironment::uext,
    }
}

fn get_literal_fn(kind: &LiteralType) -> fn(&mut SharedEnvironment, usize) {
    match kind {
        LiteralType::One => SharedEnvironment::one,
        LiteralType::Ones => SharedEnvironment::ones,
        LiteralType::Zero => SharedEnvironment::zero,
    }
}

pub fn interpret(
    prog_iterator: Iter<Btor2Instr>,
    env: &mut SharedEnvironment,
) -> Result<(), InterpError> {
    for line in prog_iterator {
        match &line.contents {
            Btor2InstrContents::BinOp { kind, .. } => {
                eval_binary_op(env, line, get_binary_fn(kind))
            }
            Btor2InstrContents::Conditional { .. } => {
                eval_ternary_op(env, line, SharedEnvironment::ite)
            }
            Btor2InstrContents::Constant { kind, .. } => match kind {
                ConstantType::Constd => eval_const_op(env, line, 10),
                ConstantType::Consth => eval_const_op(env, line, 16),
                ConstantType::Const => eval_const_op(env, line, 2),
            },
            Btor2InstrContents::Input { .. } => Ok(()),
            Btor2InstrContents::Literal { kind } => {
                eval_literals_op(env, line, get_literal_fn(kind))
            }
            Btor2InstrContents::Output { .. } => Ok(()),
            Btor2InstrContents::Slice { .. } => eval_slice_op(env, line),
            Btor2InstrContents::Sort => Ok(()),
            Btor2InstrContents::UnOp { kind, .. } => {
                eval_unary_op(env, line, get_unary_fn(kind))
            }
            Btor2InstrContents::Unknown => {
                Err(error::InterpError::Unsupported("".to_string()))
            }
        }?;
    }

    Ok(())

    // prog_iterator.try_for_each(|line| match &line.contents {
    //     Btor2InstrContents::BinOp {
    //         kind, ..
    //     } => eval_binary_op(env, line, get_binary_fn(kind)),
    //     Btor2InstrContents::Conditional {
    //         ..
    //     } => eval_ternary_op(env, line, SharedEnvironment::ite),
    //     Btor2InstrContents::Constant { kind, .. } => match kind {
    //         ConstantType::Constd => eval_const_op(env, line, 10),
    //         ConstantType::Consth => eval_const_op(env, line, 16),
    //         ConstantType::Const => eval_const_op(env, line, 2),
    //     },
    //     Btor2InstrContents::Input { .. } => Ok(()),
    //     Btor2InstrContents::Literal { kind }
    //     => eval_literals_op(env, line, get_literal_fn(kind)),
    //     Btor2InstrContents::Output { .. } => Ok(()),
    //     Btor2InstrContents::Slice {
    //         ..
    //     } => eval_slice_op(env, line),
    //     Btor2InstrContents::Sort => Ok(()),
    //     Btor2InstrContents::UnOp { kind, .. } => eval_unary_op(env, line, get_unary_fn(kind)),
    //     Btor2InstrContents::Unknown => {
    //         Err(error::InterpError::Unsupported("".to_string()))
    //     }
    // })
}

/// Handles the `const`, `constd`, and `consth` statements.
fn eval_const_op(
    env: &mut SharedEnvironment,
    line: &Btor2Instr,
    radix: u32,
) -> Result<(), error::InterpError> {
    if let Btor2InstrContents::Constant { constant, kind: _ } = &line.contents {
        match constant {
            Some(str) => {
                let nstring = str.to_string();
                let intval = BigInt::from_str_radix(&nstring, radix).unwrap();

                match line.sort {
                    SortType::Bitvec { width } => {
                        let bool_vec = (0..width)
                            .map(|i| intval.bit(i as u64))
                            .collect::<Vec<_>>();

                        env.const_(line.id, bool_vec);
                        Ok(())
                    }
                    SortType::Array {
                        index: _,
                        element: _,
                    } => Err(error::InterpError::Unsupported(
                        "Array".to_string(),
                    )),
                }
            }
            None => Err(error::InterpError::BadFuncArgType(
                "No value in constant".to_string(),
            )),
        }
    } else {
        Err(error::InterpError::Unsupported("".to_string()))
    }
}

/// Handle the `one`, `ones` and `zero` statements.
fn eval_literals_op(
    env: &mut SharedEnvironment,
    line: &Btor2Instr,
    literal_init: fn(&mut SharedEnvironment, i1: usize),
) -> Result<(), error::InterpError> {
    if let Btor2InstrContents::Literal { kind: _ } = &line.contents {
        match line.sort {
            SortType::Bitvec { width: _ } => {
                literal_init(env, line.id);
                Ok(())
            }
            SortType::Array {
                index: _,
                element: _,
            } => Err(error::InterpError::Unsupported("Array".to_string())),
        }
    } else {
        Err(error::InterpError::Unsupported("".to_string()))
    }
}

/// Handles the `slice` statements.
fn eval_slice_op(
    env: &mut SharedEnvironment,
    line: &Btor2Instr,
) -> Result<(), error::InterpError> {
    if let Btor2InstrContents::Slice { arg1, u, l } = line.contents {
        match line.sort {
            SortType::Bitvec { width } => {
                if (u - l) + 1 != width {
                    return Err(error::InterpError::Unsupported(format!(
                        "Slicing of {:?} is not supported",
                        arg1
                    )));
                }
                env.slice(u, l, arg1, line.id);
                Ok(())
            }
            SortType::Array {
                index: _,
                element: _,
            } => Err(error::InterpError::Unsupported("Array".to_string())),
        }
    } else {
        Err(error::InterpError::Unsupported("".to_string()))
    }
}

/// Handle all the unary operators.
fn eval_unary_op(
    env: &mut SharedEnvironment,
    line: &Btor2Instr,
    unary_fn: fn(&mut SharedEnvironment, usize, usize),
) -> Result<(), error::InterpError> {
    if let Btor2InstrContents::UnOp { arg1, kind: _ } = &line.contents {
        match line.sort {
            SortType::Bitvec { width: _ } => {
                unary_fn(env, *arg1, line.id);
                Ok(())
            }
            SortType::Array {
                index: _,
                element: _,
            } => Err(error::InterpError::Unsupported("Array".to_string())),
        }
    } else {
        Err(error::InterpError::Unsupported("".to_string()))
    }
}

/// Handles all the binary operators.
fn eval_binary_op(
    env: &mut SharedEnvironment,
    line: &Btor2Instr,
    binary_fn: fn(&mut SharedEnvironment, usize, usize, usize),
) -> Result<(), error::InterpError> {
    if let Btor2InstrContents::BinOp {
        arg1,
        arg2,
        kind: _,
    } = &line.contents
    {
        match line.sort {
            SortType::Bitvec { width: _ } => {
                binary_fn(env, *arg1, *arg2, line.id);
                Ok(())
            }
            SortType::Array {
                index: _,
                element: _,
            } => Err(error::InterpError::Unsupported("Array".to_string())),
        }
    } else {
        Err(error::InterpError::Unsupported("".to_string()))
    }
}

fn eval_ternary_op(
    env: &mut SharedEnvironment,
    line: &Btor2Instr,
    ternary_fn: fn(&mut SharedEnvironment, usize, usize, usize, usize),
) -> Result<(), error::InterpError> {
    if let Btor2InstrContents::Conditional { arg1, arg2, arg3 } = line.contents
    {
        ternary_fn(env, arg1, arg2, arg3, line.id);
        Ok(())
    } else {
        Err(error::InterpError::Unsupported("".to_string()))
    }
}

// TODO: eventually remove pub and make a seperate pub function as a main entry point to the interpreter, for now this is main.rs
pub fn parse_inputs(
    env: &mut SharedEnvironment,
    lines: &[Btor2Instr],
    inputs: &[String],
) -> Result<(), InterpError> {
    // create input name to line no. and sort map
    let mut input_map = HashMap::new();
    lines.iter().for_each(|line| {
        if let Btor2InstrContents::Input { name } = &line.contents {
            let input_name = name.clone();
            if let SortType::Bitvec { width } = line.sort {
                input_map.insert(input_name, (line.id, width));
            }
        }
    });

    if input_map.is_empty() && inputs.is_empty() {
        Ok(())
    } else if inputs.len() != input_map.len() {
        Err(InterpError::BadNumFuncArgs(input_map.len(), inputs.len()))
    } else {
        inputs.iter().try_for_each(|input| {
            // arg in the form "x=1", extract variable name and value
            let mut split = input.split('=');
            let arg_name = split.next().unwrap();
            let arg_val = split.next().unwrap();

            if !input_map.contains_key(arg_name) {
                return Err(InterpError::BadFuncArgName(arg_name.to_string()));
            }

            let (idx, width) = input_map.get(arg_name).unwrap();

            // input must begins with 0b
            if arg_val.starts_with("0b") {
                let arg_as_bin = arg_val
                    .trim_start_matches("0b")
                    .chars()
                    .map(|c| c == '1')
                    .collect::<Vec<_>>();

                if arg_as_bin.len() > *width {
                    return Err(InterpError::BadFuncArgWidth(
                        arg_name.to_string(),
                        *width,
                        arg_as_bin.len(),
                    ));
                }

                // pad with 0s if necessary
                let arg_as_bin = if arg_as_bin.len() < *width {
                    let mut arg_as_bin = arg_as_bin;
                    arg_as_bin.resize(*width, false);
                    arg_as_bin
                } else {
                    arg_as_bin
                };

                env.set_vec(*idx, arg_as_bin);

                Ok(())
            } else {
                Err(InterpError::BadFuncArgType(
                    "Input must be in binary format".to_string(),
                ))
            }
        })
    }
}

pub fn parse_states(
    env: &mut SharedEnvironment,
    lines: &[Btor2Instr],
    inputs: &[String],
) -> Result<(), InterpError> {
    todo!()
}
