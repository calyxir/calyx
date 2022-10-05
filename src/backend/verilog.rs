//! SystemVerilog backend for the Calyx compiler.
//!
//! Transforms an [`ir::Context`](crate::ir::Context) into a formatted string that represents a
//! valid SystemVerilog program.

use crate::backend::traits::Backend;
use calyx::{
    errors::{CalyxResult, Error},
    ir,
    utils::OutputFile,
};
use ir::{Control, Group, Guard, RRC};
use itertools::Itertools;
use std::fs::File;
use std::io;
use std::{collections::HashMap, rc::Rc};
use vast::v17::ast as v;

/// Implements a simple Verilog backend. The backend only accepts Calyx programs with no control
/// and no groups.
#[derive(Default)]
pub struct VerilogBackend;

// input string should be the cell type name of a memory cell. In other words one
// of "seq/std_mem_d_1/2/3/4". Becase we define seq_mem_d2/3/4 in terms of seq_mem_d1
// we need another layer of memory access to get the actual memory array in verilog
// for these mem types.
// In other words, for memories not defined in terms of another memory, we can just use
// "mem" to access them. But for memories defined in terms of another memory,
// which are seq_mem_d2/3/4, we need "mem.mem" to access them.
fn get_mem_str(mem_type: &str) -> &str {
    if mem_type.contains("d1") || mem_type.contains("std_mem") {
        "mem"
    } else {
        "mem.mem"
    }
}

/// Checks to make sure that there are no holes being
/// used in a guard.
fn validate_guard(guard: &ir::Guard) -> bool {
    match guard {
        Guard::Or(left, right) | Guard::And(left, right) => {
            validate_guard(left) && validate_guard(right)
        }
        Guard::CompOp(_, left, right) => {
            !left.borrow().is_hole() && !right.borrow().is_hole()
        }
        Guard::Not(inner) => validate_guard(inner),
        Guard::Port(port) => !port.borrow().is_hole(),
        Guard::True => true,
    }
}

/// Returns `Ok` if there are no groups defined.
fn validate_structure<'a, I>(groups: I) -> CalyxResult<()>
where
    I: Iterator<Item = &'a RRC<Group>>,
{
    for group in groups {
        for asgn in &group.borrow().assignments {
            let port = asgn.dst.borrow();
            // check if port is a hole
            if port.is_hole() {
                return Err(Error::malformed_structure(
                    "Groups / Holes can not be turned into Verilog".to_string(),
                )
                .with_pos(&port.attributes));
            }

            // validate guard
            if !validate_guard(&asgn.guard) {
                return Err(Error::malformed_structure(
                    "Groups / Holes can not be turned into Verilog".to_string(),
                )
                .with_pos(&port.attributes));
            };
        }
    }
    Ok(())
}

/// Returns `Ok` if the control for `comp` is either a single `enable`
/// or `empty`.
fn validate_control(ctrl: &ir::Control) -> CalyxResult<()> {
    match ctrl {
        Control::Empty(_) => Ok(()),
        c => Err(Error::malformed_structure(
            "Control must be empty".to_string(),
        )
        .with_pos(c)),
    }
}

impl Backend for VerilogBackend {
    fn name(&self) -> &'static str {
        "verilog"
    }

    fn validate(ctx: &ir::Context) -> CalyxResult<()> {
        for component in &ctx.components {
            validate_structure(component.groups.iter())?;
            validate_control(&component.control.borrow())?;
        }
        Ok(())
    }

    /// Generate a "fat" library by copy-pasting all of the extern files.
    /// A possible alternative in the future is to use SystemVerilog `include`
    /// statement.
    fn link_externs(
        ctx: &ir::Context,
        file: &mut OutputFile,
    ) -> CalyxResult<()> {
        let fw = &mut file.get_write();
        for extern_path in ctx.lib.extern_paths() {
            // The extern file is guaranteed to exist by the frontend.
            let mut ext = File::open(extern_path).unwrap();
            io::copy(&mut ext, fw).map_err(|err| {
                let std::io::Error { .. } = err;
                Error::write_error(format!(
                    "File not found: {}",
                    file.as_path_string()
                ))
            })?;
            // Add a newline after appending a library file
            writeln!(fw)?;
        }
        Ok(())
    }

    fn emit(ctx: &ir::Context, file: &mut OutputFile) -> CalyxResult<()> {
        let modules = &ctx
            .components
            .iter()
            .map(|comp| {
                emit_component(
                    comp,
                    ctx.bc.synthesis_mode,
                    ctx.bc.enable_verification,
                    ctx.bc.initialize_inputs,
                )
                .to_string()
            })
            .collect::<Vec<_>>();

        write!(file.get_write(), "{}", modules.join("\n")).map_err(|err| {
            let std::io::Error { .. } = err;
            Error::write_error(format!(
                "File not found: {}",
                file.as_path_string()
            ))
        })?;
        Ok(())
    }
}

fn emit_component(
    comp: &ir::Component,
    synthesis_mode: bool,
    enable_verification: bool,
    initialize_inputs: bool,
) -> v::Module {
    let mut module = v::Module::new(comp.name.as_ref());
    let sig = comp.signature.borrow();
    for port_ref in &sig.ports {
        let port = port_ref.borrow();
        // NOTE: The signature port definitions are reversed inside the component.
        match port.direction {
            ir::Direction::Input => {
                module.add_output(port.name.as_ref(), port.width);
            }
            ir::Direction::Output => {
                module.add_input(port.name.as_ref(), port.width);
            }
            ir::Direction::Inout => {
                panic!("Unexpected Inout port on Component: {}", port.name)
            }
        }
    }

    // Add a COMPONENT START: <name> anchor before any code in the component
    module.add_stmt(v::Stmt::new_rawstr(format!(
        "// COMPONENT START: {}",
        comp.name
    )));

    // Add memory initial and final blocks
    if !synthesis_mode {
        memory_read_write(comp).into_iter().for_each(|stmt| {
            module.add_stmt(stmt);
        });
    }

    let wires = comp
        .cells
        .iter()
        .flat_map(|cell| wire_decls(&cell.borrow()))
        .collect_vec();
    // structure wire declarations
    wires.iter().for_each(|(name, width, _)| {
        module.add_decl(v::Decl::new_logic(name, *width));
    });

    // Generate initial assignments for all input ports in defined cells.
    if initialize_inputs {
        let mut initial = v::ParallelProcess::new_initial();
        wires.iter().for_each(|(name, width, dir)| {
            if *dir == ir::Direction::Input {
                // HACK: this is not the right way to reset
                // registers. we should have real reset ports.
                let value = String::from("0");
                initial.add_seq(v::Sequential::new_blk_assign(
                    v::Expr::new_ref(name),
                    v::Expr::new_ulit_dec(*width as u32, &value),
                ));
            }
        });
        module.add_process(initial);
    }

    // cell instances
    comp.cells
        .iter()
        .filter_map(|cell| cell_instance(&cell.borrow()))
        .for_each(|instance| {
            module.add_instance(instance);
        });

    // gather assignments keyed by destination
    let mut map: HashMap<_, (RRC<ir::Port>, Vec<_>)> = HashMap::new();
    for asgn in &comp.continuous_assignments {
        map.entry(asgn.dst.borrow().canonical())
            .and_modify(|(_, v)| v.push(asgn))
            .or_insert((Rc::clone(&asgn.dst), vec![asgn]));
    }

    // Build a top-level always block to contain verilator checks for assignments
    let mut checks = v::ParallelProcess::new_always_comb();

    map.values()
        .sorted_by_key(|(port, _)| port.borrow().canonical())
        .for_each(|asgns| {
            module.add_stmt(v::Stmt::new_parallel(emit_assignment(asgns)));
            // If verification generation is enabled, emit disjointness check.
            if enable_verification {
                if let Some(check) = emit_guard_disjoint_check(asgns) {
                    checks.add_seq(check);
                };
            }
        });

    if !synthesis_mode {
        module.add_process(checks);
    }

    // Add COMPONENT END: <name> anchor
    module.add_stmt(v::Stmt::new_rawstr(format!(
        "// COMPONENT END: {}",
        comp.name
    )));

    module
}

fn wire_decls(cell: &ir::Cell) -> Vec<(String, u64, ir::Direction)> {
    cell.ports
        .iter()
        .filter_map(|port| match &port.borrow().parent {
            ir::PortParent::Cell(cell) => {
                let parent_ref = cell.upgrade();
                let parent = parent_ref.borrow();
                match parent.prototype {
                    ir::CellType::Component { .. }
                    | ir::CellType::Primitive { .. } => Some((
                        format!(
                            "{}_{}",
                            parent.name().as_ref(),
                            port.borrow().name.as_ref()
                        ),
                        port.borrow().width,
                        port.borrow().direction.clone(),
                    )),
                    _ => None,
                }
            }
            ir::PortParent::Group(_) => unreachable!(),
        })
        .collect()
}

fn cell_instance(cell: &ir::Cell) -> Option<v::Instance> {
    match cell.type_name() {
        Some(ty_name) => {
            let mut inst =
                v::Instance::new(cell.name().as_ref(), ty_name.as_ref());

            if let ir::CellType::Primitive {
                name,
                param_binding,
                ..
            } = &cell.prototype
            {
                if name == "std_const" {
                    let (wn, width) = &param_binding[0];
                    let (vn, value) = &param_binding[1];
                    inst.add_param(&wn.id, v::Expr::new_int(*width as i32));
                    inst.add_param(
                        &vn.id,
                        v::Expr::new_ulit_dec(
                            *width as u32,
                            &value.to_string(),
                        ),
                    );
                } else {
                    param_binding.iter().for_each(|(name, value)| {
                    if *value > (std::i32::MAX as u64) {
                        panic!(
                            "Parameter value {} for `{}` cannot be represented using 32 bits",
                            value,
                            name
                        )
                    }
                    inst.add_param(
                        name.as_ref(),
                        v::Expr::new_int(*value as i32),
                    )
                })
                }
            }

            for port in &cell.ports {
                inst.connect(port.borrow().name.as_ref(), port_to_ref(port));
            }
            Some(inst)
        }
        None => None,
    }
}

/// Generates an always block that checks of the guards are disjoint when the
/// length of assignments is greater than 1:
/// ```verilog
/// always_ff @(posedge clk) begin
///   if (!$onehot0({fsm_out < 1'd1 & go, fsm_out < 1'd1 & go})) begin
///     $error("Multiple assignments to r_in");
///   end
/// end
/// ```
fn emit_guard_disjoint_check(
    (dst_ref, assignments): &(RRC<ir::Port>, Vec<&ir::Assignment>),
) -> Option<v::Sequential> {
    if assignments.len() < 2 {
        return None;
    }
    // Construct concat with all guards.
    let mut concat = v::ExprConcat::default();
    assignments.iter().for_each(|assign| {
        concat.add_expr(guard_to_expr(&assign.guard));
    });

    let onehot0 = v::Expr::new_call("$onehot0", vec![v::Expr::Concat(concat)]);
    let not_onehot0 = v::Expr::new_not(onehot0);
    let mut check = v::SequentialIfElse::new(not_onehot0);

    // Generated error message
    let ir::Canonical(cell, port) = dst_ref.borrow().canonical();
    let msg = format!("Multiple assignment to port `{}.{}'.", cell, port);
    let err = v::Sequential::new_seqexpr(v::Expr::new_call(
        "$fatal",
        vec![v::Expr::new_int(2), v::Expr::Str(msg)],
    ));
    check.add_seq(err);
    Some(v::Sequential::If(check))
}

/// Checks if:
/// 1. The port is marked with `@data`
/// 2. The port's cell parent is marked with `@data`
fn is_data_port(pr: &RRC<ir::Port>) -> bool {
    let port = pr.borrow();
    if !port.attributes.has("data") {
        return false;
    }
    if let ir::PortParent::Cell(cwr) = &port.parent {
        let cr = cwr.upgrade();
        let cell = cr.borrow();
        if cell.attributes.has("data") {
            return true;
        }
    }
    false
}

/// Generates an assign statement that uses ternaries to select the correct
/// assignment to enable and adds a default assignment to 0 when none of the
/// guards are active.
///
/// Example:
/// ```
/// // Input Calyx code
/// a.in = foo ? 2'd0;
/// a.in = bar ? 2'd1;
/// ```
/// Into:
/// ```
/// assign a_in = foo ? 2'd0 : bar ? 2d'1 : 2'd0;
/// ```
fn emit_assignment(
    (dst_ref, assignments): &(RRC<ir::Port>, Vec<&ir::Assignment>),
) -> v::Parallel {
    // Mux over the assignment with the given default value.
    let fold_assigns = |init: v::Expr| -> v::Expr {
        assignments.iter().rfold(init, |acc, e| {
            let guard = guard_to_expr(&e.guard);
            let asgn = port_to_ref(&e.src);
            v::Expr::new_mux(guard, asgn, acc)
        })
    };

    // If this is a data port
    let rhs: v::Expr = if is_data_port(dst_ref) {
        if assignments.len() == 1 {
            // If there is exactly one guard, generate a continuous assignment.
            // This encodes the rewrite:
            // in = g ? out : 'x => in = out;
            // This is valid because 'x can be replaced with any value
            let assign = assignments[0];
            port_to_ref(&assign.src)
        } else {
            // Produce an assignment with 'x as the default case.
            fold_assigns(v::Expr::X)
        }
    } else {
        let init = v::Expr::new_ulit_dec(
            dst_ref.borrow().width as u32,
            &0.to_string(),
        );

        // Flatten the mux expression if there is exactly one assignment with a true guard.
        if assignments.len() == 1 {
            let assign = assignments[0];
            if assign.guard.is_true() {
                port_to_ref(&assign.src)
            } else if assign.src.borrow().is_constant(1, 1) {
                guard_to_expr(&assign.guard)
            } else {
                v::Expr::new_mux(
                    guard_to_expr(&assign.guard),
                    port_to_ref(&assign.src),
                    init,
                )
            }
        } else {
            fold_assigns(init)
        }
    };
    v::Parallel::ParAssign(port_to_ref(dst_ref), rhs)
}

fn port_to_ref(port_ref: &RRC<ir::Port>) -> v::Expr {
    let port = port_ref.borrow();
    match &port.parent {
        ir::PortParent::Cell(cell) => {
            let parent_ref = cell.upgrade();
            let parent = parent_ref.borrow();
            match parent.prototype {
                ir::CellType::Constant { val, width } => {
                    v::Expr::new_ulit_dec(width as u32, &val.to_string())
                }
                ir::CellType::ThisComponent => v::Expr::new_ref(&port.name),
                _ => v::Expr::Ref(format!(
                    "{}_{}",
                    parent.name().as_ref(),
                    port.name.as_ref()
                )),
            }
        }
        ir::PortParent::Group(_) => unreachable!(),
    }
}

fn guard_to_expr(guard: &ir::Guard) -> v::Expr {
    let op = |g: &ir::Guard| match g {
        Guard::Or(..) => v::Expr::new_bit_or,
        Guard::And(..) => v::Expr::new_bit_and,
        Guard::CompOp(op, ..) => match op {
            ir::PortComp::Eq => v::Expr::new_eq,
            ir::PortComp::Neq => v::Expr::new_neq,
            ir::PortComp::Gt => v::Expr::new_gt,
            ir::PortComp::Lt => v::Expr::new_lt,
            ir::PortComp::Geq => v::Expr::new_geq,
            ir::PortComp::Leq => v::Expr::new_leq,
        },
        Guard::Not(..) | Guard::Port(..) | Guard::True => unreachable!(),
    };

    match guard {
        Guard::And(l, r) | Guard::Or(l, r) => {
            op(guard)(guard_to_expr(l), guard_to_expr(r))
        }
        Guard::CompOp(_, l, r) => op(guard)(port_to_ref(l), port_to_ref(r)),
        Guard::Not(o) => v::Expr::new_not(guard_to_expr(o)),
        Guard::Port(p) => port_to_ref(p),
        Guard::True => v::Expr::new_ulit_bin(1, &1.to_string()),
    }
}

//==========================================
//        Memory input and output
//==========================================
/// Generates code of the form:
/// ```
/// string DATA;
/// int CODE;
/// initial begin
///   CODE = $value$plusargs("DATA=%s", DATA);
///   $display("DATA: %s", DATA);
///   $readmemh({DATA, "/<mem_name>.dat"}, <mem_name>.mem);
///   ...
/// end
/// final begin
///   $writememh({DATA, "/<mem_name>.out"}, <mem_name>.mem);
/// end
/// ```
fn memory_read_write(comp: &ir::Component) -> Vec<v::Stmt> {
    // Find all memories marked as @external
    let memories = comp
        .cells
        .iter()
        .filter_map(|cell| {
            let is_external = cell.borrow().get_attribute("external").is_some();
            if is_external
                && cell
                    .borrow()
                    .type_name()
                    .map(|proto| proto.id.contains("mem"))
                    .unwrap_or_default()
            {
                Some((
                    cell.borrow().name().id.clone(),
                    cell.borrow().type_name().unwrap_or_else(|| unreachable!("tried to add a memory cell but there was no type name")).clone(),
                ))
            } else {
                None
            }
        })
        .collect_vec();

    if memories.is_empty() {
        return vec![];
    }

    // Import futil helper library.
    let data_decl = v::Stmt::new_rawstr("string DATA;".to_string());
    let code_decl = v::Stmt::new_rawstr("int CODE;".to_string());

    let plus_args = v::Sequential::new_blk_assign(
        v::Expr::Ref("CODE".to_string()),
        v::Expr::new_call(
            "$value$plusargs",
            vec![v::Expr::new_str("DATA=%s"), v::Expr::new_ref("DATA")],
        ),
    );

    let mut initial_block = v::ParallelProcess::new_initial();
    initial_block
        // get the data
        .add_seq(plus_args)
        // log the path to the data
        .add_seq(v::Sequential::new_seqexpr(v::Expr::new_call(
            "$display",
            vec![
                v::Expr::new_str("DATA (path to meminit files): %s"),
                v::Expr::new_ref("DATA"),
            ],
        )));

    memories.iter().for_each(|(name, mem_type)| {
        let mem_access_str = get_mem_str(&mem_type.id);
        initial_block.add_seq(v::Sequential::new_seqexpr(v::Expr::new_call(
            "$readmemh",
            vec![
                v::Expr::Concat(v::ExprConcat {
                    exprs: vec![
                        v::Expr::new_str(&format!("/{}.dat", name)),
                        v::Expr::new_ref("DATA"),
                    ],
                }),
                v::Expr::new_ipath(&format!("{}.{}", name, mem_access_str)),
            ],
        )));
    });

    let mut final_block = v::ParallelProcess::new_final();
    memories.iter().for_each(|(name, mem_type)| {
        let mem_access_str = get_mem_str(&mem_type.id);

        final_block.add_seq(v::Sequential::new_seqexpr(v::Expr::new_call(
            "$writememh",
            vec![
                v::Expr::Concat(v::ExprConcat {
                    exprs: vec![
                        v::Expr::new_str(&format!("/{}.out", name)),
                        v::Expr::new_ref("DATA"),
                    ],
                }),
                v::Expr::new_ipath(&format!("{}.{}", name, mem_access_str)),
            ],
        )));
    });

    vec![
        data_decl,
        code_decl,
        v::Stmt::new_parallel(v::Parallel::new_process(initial_block)),
        v::Stmt::new_parallel(v::Parallel::new_process(final_block)),
    ]
}
