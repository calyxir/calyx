use calyx::{
    errors::{Error, FutilResult},
    frontend, ir,
    pass_manager::PassManager,
    utils::OutputFile,
};
use interp::environment;
use interp::interpreter::interpret_component;
use std::cell::RefCell;
use std::path::PathBuf;
use structopt::StructOpt;
use values::Value;

/// CLI Options
#[derive(Debug, StructOpt)]
#[structopt(name = "interpreter", about = "interpreter CLI")]
pub struct Opts {
    /// Input file
    #[structopt(parse(from_os_str))]
    pub file: Option<PathBuf>,

    /// Output file, default is stdout
    #[structopt(short = "o", long = "output", default_value)]
    pub output: OutputFile,

    /// Path to the primitives library
    #[structopt(long, short, default_value = "..")]
    pub lib_path: PathBuf,

    /// Component to interpret
    #[structopt(short = "c", long = "component", default_value = "main")]
    pub component: String,

    /// Group to interpret
    /// XX(karen): The user can specify a particular group to interpret,
    /// assuming the group is in `main` if not specified otherwise.
    #[structopt(short = "g", long = "group", default_value = "main")]
    pub group: String,
}

//first half of this is tests
/// Interpret a group from a Calyx program
fn main() -> FutilResult<()> {
    //make a Value with bitwidth >= # of bits in binnum of given u64
    let bit_width: usize = 5;
    let bit_value: usize = 12;
    let v1 = Value::from_init(bit_value, bit_width);
    println!("12 with bit width 5: {}", v1);
    let v2 = StdLsh::execute(v1);
    println!("12 shifted left: {}", v2);
    println!("5-bit v2 value as u64: {}", Value::as_u64(&v2));

    //make a Value with bitwdith < # of bits in binnum of given u64
    let v1 = Value::from_init(7 as usize, 3 as usize);
    println!("7 with bitwidth 3: {}", v1);
    let v2 = StdLsh::execute(v1);
    println!("7 shifted left: {}", v2);
    println!("3-bit v2 value as u64: {}", Value::as_u64(&v2));

    //PROBLEM: seems like we can only make values from usizes
    //for some reason the usize type annotation for bit_width is also affecting bit_value
    // StdAdd - does not overflow if done with 8 bits
    let add0 = Value::from_init(101 as usize, 8 as usize);
    let add1 = Value::from_init(99 as usize, 8 as usize);
    let res_add = StdAdd::execute(add0.clone(), add1.clone());
    println!("Result of adding {} and {}: {}", add0, add1, res_add);
    // StdSub
    let sub0 = Value::from_init(101 as usize, 7 as usize);
    let sub1 = Value::from_init(99 as usize, 7 as usize);
    let res_sub = StdSub::execute(sub0.clone(), sub1.clone());
    println!("Result of subtracting {} from {}: {}", sub1, sub0, res_sub);
    // StdSlice
    let slice = Value::from_init(101 as usize, 7 as usize);
    let width: usize = 4;
    let res_slice = StdSlice::execute(slice.clone(), width.clone());
    println!("Slicing {} to bitwidth {}: {}", slice, width, res_slice);
    // StdPad
    let pad = Value::from_init(101 as usize, 7 as usize);
    let width: usize = 9;
    let res_pad = StdPad::execute(pad.clone(), width.clone());
    println!("Padding {} to bitwidth {}: {}", pad, width, res_pad);

    //try out reading and writing to a register, and coordinating its read and write signals
    //remember that registers are mutable, not functional

    //try loading a register with a value of the right size, then too small, then too big
    let val = Value::from_init(16 as usize, 5 as usize);
    let mut reg1 = StdReg::new(5);
    reg1.set_write_en_high();
    reg1.load_value(val);
    reg1.set_write_en_low();
    reg1.set_done_high();
    println!(
        "just loaded reg1 (width 5) with val of 16, see val: {:?}",
        reg1.read_value()
    );
    reg1.set_write_en_high();
    reg1.load_u64(16);
    reg1.set_write_en_low();
    reg1.set_done_high();
    println!(
        "just loaded reg1 (width 5) with u64 16, see: {}",
        reg1.read_u64()
    );
    //same register, try loading while write en is low
    reg1.load_u64(15);
    println!(
        "just tried loading reg1 with 15 while write_en was low: {}",
        reg1.read_u64()
    );
    //now try loading in a value that is too big
    reg1.set_write_en_high();
    let val = Value::from_init(16 as usize, 4 as usize);
    reg1.load_value(val);
    reg1.set_write_en_low();
    reg1.set_done_high();
    println!(
        "just loaded reg1 (width 5) with val of 16 (width 4), see val: {:?}",
        reg1.read_value()
    );
    //now load a # that needs more than 5 bits, like 33:
    reg1.set_write_en_high();
    reg1.load_u64(33);
    println!(
        "just loaded reg1 (width 5) with u64 33, expecting [10001] see: {}",
        reg1.read_u64()
    );

    //test out the constants:
    let const_31 = StdConst::new_from_u64(5, 31);
    println!(
        "const_31 from u64 as u64: {} and as val: {:?}",
        const_31.read_u64(),
        const_31.read_val()
    );
    let val_31 = Value::from_init(31 as usize, 5 as usize);
    let const_31 = StdConst::new(5, val_31);
    println!(
        "const_31 from val as u64: {} and as val: {:?}",
        const_31.read_u64(),
        const_31.read_val()
    );
    // Logical Operators
    // StdNot
    let not0 = Value::from_init(10 as usize, 4 as usize);
    let res_not = StdNot::execute(not0.clone());
    println!("!{}: {}", not0, res_not);
    // StdAnd
    let and0 = Value::from_init(101 as usize, 7 as usize);
    let and1 = Value::from_init(78 as usize, 7 as usize);
    let res_and = StdAnd::execute(and0.clone(), and1.clone());
    println!("{} & {}: {}", and0, and1, res_and);
    // StdOr
    let or0 = Value::from_init(101 as usize, 7 as usize);
    let or1 = Value::from_init(78 as usize, 7 as usize);
    let res_or = StdOr::execute(or0.clone(), or1.clone());
    println!("{} | {}: {}", or0, or1, res_or);
    // StdXor
    let xor0 = Value::from_init(101 as usize, 7 as usize);
    let xor1 = Value::from_init(78 as usize, 7 as usize);
    let res_xor = StdXor::execute(xor0.clone(), xor1.clone());
    println!("{} ^ {}: {}", xor0, xor1, res_xor);

    // Comparison Operators
    // StdGt
    let gt0 = Value::from_init(101 as usize, 7 as usize);
    let gt1 = Value::from_init(78 as usize, 7 as usize);
    let res_gt = StdGt::execute(gt0.clone(), gt1.clone());
    println!("{} > {}: {}", gt0, gt1, res_gt);
    // StdLt
    let lt0 = Value::from_init(101 as usize, 7 as usize);
    let lt1 = Value::from_init(78 as usize, 7 as usize);
    let res_lt = StdLt::execute(lt0.clone(), lt1.clone());
    println!("{} < {}: {}", lt0, lt1, res_lt);
    // StdEq
    let eq0 = Value::from_init(101 as usize, 7 as usize);
    let eq1 = Value::from_init(78 as usize, 7 as usize);
    let res_eq = StdEq::execute(eq0.clone(), eq1.clone());
    println!("{} == {}: {}", eq0, eq1, res_eq);
    // StdNeq
    let neq0 = Value::from_init(101 as usize, 7 as usize);
    let neq1 = Value::from_init(78 as usize, 7 as usize);
    let res_neq = StdNeq::execute(neq0.clone(), neq1.clone());
    println!("{} != {}: {}", neq0, neq1, res_neq);
    // StdGe
    let ge0 = Value::from_init(78 as usize, 7 as usize);
    let ge1 = Value::from_init(78 as usize, 7 as usize);
    let res_ge = StdGe::execute(ge0.clone(), ge1.clone());
    println!("{} >= {}: {}", ge0, ge1, res_ge);
    // StdLe
    let le0 = Value::from_init(99 as usize, 7 as usize);
    let le1 = Value::from_init(101 as usize, 7 as usize);
    let res_le = StdLe::execute(le0.clone(), le1.clone());
    println!("{} <= {}: {}", le0, le1, res_le);

    let opts = Opts::from_args();

    // Construct IR
    let namespace = frontend::NamespaceDef::new(&opts.file, &opts.lib_path)?;
    let ir = ir::from_ast::ast_to_ir(namespace, false, false)?;

    let ctx = ir::RRC::new(RefCell::new(ir));

    let pm = PassManager::default_passes()?;

    pm.execute_plan(&mut ctx.borrow_mut(), &["validate".to_string()], &[])?;

    let env = environment::Environment::init(&ctx);

    // Get main component; assuming that opts.component is main
    // TODO: handle when component, group are not default values

    let ctx_ref: &ir::Context = &ctx.borrow();
    let main_component = ctx_ref
        .components
        .iter()
        .find(|&cm| cm.name == "main")
        .ok_or_else(|| {
            Error::Impossible("Cannot find main component".to_string())
        })?;

    match interpret_component(main_component, env) {
        Ok(e) => {
            e.print_env();
            Ok(())
        }
        Err(err) => FutilResult::Err(err),
    }
}
