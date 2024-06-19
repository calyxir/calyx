use crate::bvec::BitVector;
use crate::error;
use crate::error::InterpError;
use crate::shared_env::SharedEnvironment;
use btor2tools::Btor2Line;
use btor2tools::Btor2SortContent;
use btor2tools::Btor2SortTag;
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

pub fn interpret(
    mut prog_iterator: Iter<Btor2Line>,
    env: &mut SharedEnvironment,
) -> Result<(), InterpError> {
    prog_iterator.try_for_each(|line| {
        match line.tag() {
            // core
            btor2tools::Btor2Tag::Sort => Ok(()), // skip - sort information is handled by the parser
            btor2tools::Btor2Tag::Const => eval_const_op(env, line, 2),
            btor2tools::Btor2Tag::Constd => eval_const_op(env, line, 10),
            btor2tools::Btor2Tag::Consth => eval_const_op(env, line, 16),
            btor2tools::Btor2Tag::Input => Ok(()), // handled in parse_inputs
            btor2tools::Btor2Tag::Output => Ok(()), // handled in extract_output
            btor2tools::Btor2Tag::One => {
                eval_literals_op(env, line, SharedEnvironment::one)
            }
            btor2tools::Btor2Tag::Ones => {
                eval_literals_op(env, line, SharedEnvironment::ones)
            }
            btor2tools::Btor2Tag::Zero => {
                eval_literals_op(env, line, SharedEnvironment::zero)
            }

            // indexed
            btor2tools::Btor2Tag::Sext => {
                eval_unary_op(env, line, SharedEnvironment::sext)
            }
            btor2tools::Btor2Tag::Uext => {
                eval_unary_op(env, line, SharedEnvironment::uext)
            }
            btor2tools::Btor2Tag::Slice => eval_slice_op(env, line),

            // unary
            btor2tools::Btor2Tag::Not => {
                eval_unary_op(env, line, SharedEnvironment::not)
            }
            btor2tools::Btor2Tag::Inc => {
                eval_unary_op(env, line, SharedEnvironment::inc)
            }
            btor2tools::Btor2Tag::Dec => {
                eval_unary_op(env, line, SharedEnvironment::dec)
            }
            btor2tools::Btor2Tag::Neg => {
                eval_unary_op(env, line, SharedEnvironment::neg)
            }
            btor2tools::Btor2Tag::Redand => {
                eval_unary_op(env, line, SharedEnvironment::redand)
            }
            btor2tools::Btor2Tag::Redor => {
                eval_unary_op(env, line, SharedEnvironment::redor)
            }
            btor2tools::Btor2Tag::Redxor => {
                eval_unary_op(env, line, SharedEnvironment::redxor)
            }

            // binary - boolean
            btor2tools::Btor2Tag::Iff => {
                eval_binary_op(env, line, SharedEnvironment::iff)
            }
            btor2tools::Btor2Tag::Implies => {
                eval_binary_op(env, line, SharedEnvironment::implies)
            }
            btor2tools::Btor2Tag::Eq => {
                eval_binary_op(env, line, SharedEnvironment::eq)
            }
            btor2tools::Btor2Tag::Neq => {
                eval_binary_op(env, line, SharedEnvironment::neq)
            }

            // binary - (un)signed inequality
            btor2tools::Btor2Tag::Sgt => {
                eval_binary_op(env, line, SharedEnvironment::sgt)
            }
            btor2tools::Btor2Tag::Sgte => {
                eval_binary_op(env, line, SharedEnvironment::sgte)
            }
            btor2tools::Btor2Tag::Slt => {
                eval_binary_op(env, line, SharedEnvironment::slt)
            }
            btor2tools::Btor2Tag::Slte => {
                eval_binary_op(env, line, SharedEnvironment::slte)
            }
            btor2tools::Btor2Tag::Ugt => {
                eval_binary_op(env, line, SharedEnvironment::ugt)
            }
            btor2tools::Btor2Tag::Ugte => {
                eval_binary_op(env, line, SharedEnvironment::ugte)
            }
            btor2tools::Btor2Tag::Ult => {
                eval_binary_op(env, line, SharedEnvironment::ult)
            }
            btor2tools::Btor2Tag::Ulte => {
                eval_binary_op(env, line, SharedEnvironment::ulte)
            }

            // binary - bit-wise
            btor2tools::Btor2Tag::And => {
                eval_binary_op(env, line, SharedEnvironment::and)
            }
            btor2tools::Btor2Tag::Nand => {
                eval_binary_op(env, line, SharedEnvironment::nand)
            }
            btor2tools::Btor2Tag::Nor => {
                eval_binary_op(env, line, SharedEnvironment::nor)
            }
            btor2tools::Btor2Tag::Or => {
                eval_binary_op(env, line, SharedEnvironment::or)
            }
            btor2tools::Btor2Tag::Xnor => {
                eval_binary_op(env, line, SharedEnvironment::xnor)
            }
            btor2tools::Btor2Tag::Xor => {
                eval_binary_op(env, line, SharedEnvironment::xor)
            }

            // binary - rotate, shift
            btor2tools::Btor2Tag::Rol => {
                eval_binary_op(env, line, SharedEnvironment::rol)
            }
            btor2tools::Btor2Tag::Ror => {
                eval_binary_op(env, line, SharedEnvironment::ror)
            }
            btor2tools::Btor2Tag::Sll => {
                eval_binary_op(env, line, SharedEnvironment::sll)
            }
            btor2tools::Btor2Tag::Sra => {
                eval_binary_op(env, line, SharedEnvironment::sra)
            }
            btor2tools::Btor2Tag::Srl => {
                eval_binary_op(env, line, SharedEnvironment::srl)
            }

            // binary - arithmetic
            btor2tools::Btor2Tag::Add => {
                eval_binary_op(env, line, SharedEnvironment::add)
            }
            btor2tools::Btor2Tag::Mul => {
                eval_binary_op(env, line, SharedEnvironment::mul)
            }
            btor2tools::Btor2Tag::Sdiv => {
                eval_binary_op(env, line, SharedEnvironment::sdiv)
            }
            btor2tools::Btor2Tag::Udiv => {
                eval_binary_op(env, line, SharedEnvironment::udiv)
            }
            btor2tools::Btor2Tag::Smod => {
                eval_binary_op(env, line, SharedEnvironment::smod)
            }
            btor2tools::Btor2Tag::Srem => {
                eval_binary_op(env, line, SharedEnvironment::srem)
            }
            btor2tools::Btor2Tag::Urem => {
                eval_binary_op(env, line, SharedEnvironment::urem)
            }
            btor2tools::Btor2Tag::Sub => {
                eval_binary_op(env, line, SharedEnvironment::sub)
            }

            // binary - overflow
            btor2tools::Btor2Tag::Saddo => {
                eval_binary_op(env, line, SharedEnvironment::saddo)
            }
            btor2tools::Btor2Tag::Uaddo => {
                eval_binary_op(env, line, SharedEnvironment::uaddo)
            }
            btor2tools::Btor2Tag::Sdivo => {
                eval_binary_op(env, line, SharedEnvironment::sdivo)
            }
            // btor2tools::Btor2Tag::Udivo => Ok(()),    Unsigned division never overflows :D
            btor2tools::Btor2Tag::Smulo => {
                eval_binary_op(env, line, SharedEnvironment::smulo)
            }
            btor2tools::Btor2Tag::Umulo => {
                eval_binary_op(env, line, SharedEnvironment::umulo)
            }
            btor2tools::Btor2Tag::Ssubo => {
                eval_binary_op(env, line, SharedEnvironment::ssubo)
            }
            btor2tools::Btor2Tag::Usubo => {
                eval_binary_op(env, line, SharedEnvironment::usubo)
            }

            // binary - concat
            btor2tools::Btor2Tag::Concat => {
                eval_binary_op(env, line, SharedEnvironment::concat)
            }

            // ternary - conditional
            btor2tools::Btor2Tag::Ite => {
                eval_ternary_op(env, line, SharedEnvironment::ite)
            }

            // Unsupported: arrays, state, assertions
            btor2tools::Btor2Tag::Bad
            | btor2tools::Btor2Tag::Constraint
            | btor2tools::Btor2Tag::Fair
            | btor2tools::Btor2Tag::Init
            | btor2tools::Btor2Tag::Justice
            | btor2tools::Btor2Tag::Next
            | btor2tools::Btor2Tag::State
            | btor2tools::Btor2Tag::Read
            | btor2tools::Btor2Tag::Write => Err(
                error::InterpError::Unsupported(format!("{:?}", line.tag())),
            ),
        }
    })
}

/// Handles the `const`, `constd`, and `consth` statements.
fn eval_const_op(
    env: &mut SharedEnvironment,
    line: &btor2tools::Btor2Line,
    radix: u32,
) -> Result<(), error::InterpError> {
    match line.constant() {
        Some(cstr) => match cstr.to_str() {
            Ok(str) => {
                let nstring = str.to_string();
                let intval = BigInt::from_str_radix(&nstring, radix).unwrap();

                match line.sort().tag() {
                    Btor2SortTag::Bitvec => {
                        if let Btor2SortContent::Bitvec { width } =
                            line.sort().content()
                        {
                            let bool_vec = (0..width)
                                .map(|i| intval.bit(i as u64))
                                .collect::<Vec<_>>();

                            env.const_(line.id().try_into().unwrap(), bool_vec);
                        }
                        Ok(())
                    }
                    Btor2SortTag::Array => {
                        Err(error::InterpError::Unsupported(format!(
                            "{:?}",
                            line.sort().tag()
                        )))
                    }
                }
            }
            Err(_e) => Err(error::InterpError::BadFuncArgType(
                "Bad value in constant".to_string(),
            )),
        },
        None => Err(error::InterpError::BadFuncArgType(
            "No value in constant".to_string(),
        )),
    }
}

/// Handle the `one`, `ones` and `zero` statements.
fn eval_literals_op(
    env: &mut SharedEnvironment,
    line: &btor2tools::Btor2Line,
    literal_init: fn(&mut SharedEnvironment, i1: usize),
) -> Result<(), error::InterpError> {
    match line.sort().tag() {
        Btor2SortTag::Bitvec => {
            literal_init(env, line.id().try_into().unwrap());
            Ok(())
        }
        Btor2SortTag::Array => Err(error::InterpError::Unsupported(format!(
            "{:?}",
            line.sort().tag()
        ))),
    }
}

/// Handles the `slice` statements.
fn eval_slice_op(
    env: &mut SharedEnvironment,
    line: &btor2tools::Btor2Line,
) -> Result<(), error::InterpError> {
    let sort = line.sort();
    match sort.tag() {
        Btor2SortTag::Bitvec => {
            assert_eq!(line.args().len(), 3);
            let arg1_line = line.args()[0] as usize;
            let u = line.args()[1] as usize;
            let l = line.args()[2] as usize;
            if let Btor2SortContent::Bitvec { width } = line.sort().content() {
                if (u - l) + 1 != width as usize {
                    return Err(error::InterpError::Unsupported(format!(
                        "Slicing of {:?} is not supported",
                        arg1_line
                    )));
                }
                env.slice(u, l, arg1_line, line.id().try_into().unwrap());
                Ok(())
            } else {
                Err(error::InterpError::Unsupported(format!(
                    "Slicing of {:?} is not supported",
                    arg1_line
                )))
            }
        }
        Btor2SortTag::Array => Err(error::InterpError::Unsupported(format!(
            "{:?}",
            line.sort().tag()
        ))),
    }
}

/// Handle all the unary operators.
fn eval_unary_op(
    env: &mut SharedEnvironment,
    line: &btor2tools::Btor2Line,
    unary_fn: fn(&mut SharedEnvironment, usize, usize),
) -> Result<(), error::InterpError> {
    let sort = line.sort();
    match sort.tag() {
        Btor2SortTag::Bitvec => {
            assert_eq!(line.args().len(), 1);
            let arg1_line = line.args()[0] as usize;
            unary_fn(env, arg1_line, line.id().try_into().unwrap());
            Ok(())
        }
        Btor2SortTag::Array => Err(error::InterpError::Unsupported(format!(
            "{:?}",
            line.sort().tag()
        ))),
    }
}

/// Handles all the binary operators.
fn eval_binary_op(
    env: &mut SharedEnvironment,
    line: &btor2tools::Btor2Line,
    binary_fn: fn(&mut SharedEnvironment, usize, usize, usize),
) -> Result<(), error::InterpError> {
    let sort = line.sort();
    match sort.tag() {
        Btor2SortTag::Bitvec => {
            assert_eq!(line.args().len(), 2);
            let arg1_line = line.args()[0] as usize;
            let arg2_line = line.args()[1] as usize;

            binary_fn(env, arg1_line, arg2_line, line.id().try_into().unwrap());
            Ok(())
        }
        Btor2SortTag::Array => Err(error::InterpError::Unsupported(format!(
            "{:?}",
            line.sort().tag()
        ))),
    }
}

fn eval_ternary_op(
    env: &mut SharedEnvironment,
    line: &btor2tools::Btor2Line,
    ternary_fn: fn(&mut SharedEnvironment, usize, usize, usize, usize),
) -> Result<(), error::InterpError> {
    assert_eq!(line.args().len(), 3);
    let arg1_line = line.args()[0] as usize;
    let arg2_line = line.args()[1] as usize;
    let arg3_line = line.args()[2] as usize;
    ternary_fn(
        env,
        arg1_line,
        arg2_line,
        arg3_line,
        line.id().try_into().unwrap(),
    );
    Ok(())
}

// TODO: eventually remove pub and make a seperate pub function as a main entry point to the interpreter, for now this is main.rs
pub fn parse_inputs(
    env: &mut SharedEnvironment,
    lines: &[Btor2Line],
    inputs: &[String],
) -> Result<(), InterpError> {
    // create input name to line no. and sort map
    let mut input_map = HashMap::new();
    lines.iter().for_each(|line| {
        if let btor2tools::Btor2Tag::Input = line.tag() {
            let input_name =
                line.symbol().unwrap().to_string_lossy().into_owned();
            if let Btor2SortContent::Bitvec { width } = line.sort().content() {
                input_map.insert(
                    input_name,
                    (
                        usize::try_from(line.id()).unwrap(),
                        usize::try_from(width).unwrap(),
                    ),
                );
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
