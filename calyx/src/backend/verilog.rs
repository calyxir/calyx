//! SystemVerilog backend for the Calyx compiler.
//!
//! Transforms an [`ir::Context`](crate::ir::Context) into a formatted string that represents a
//! valid SystemVerilog program.

use crate::{
    backend::traits::Backend,
    errors::{Error, FutilResult},
    ir,
    utils::OutputFile,
};
use ir::{Control, Group, Guard, RRC};
use itertools::Itertools;
use std::fs::File;
use std::io;
use std::{collections::HashMap, rc::Rc};
use vast::v17::ast as v;

/// Implements a simple Verilog backend. The backend
/// only accepts Futil programs with no control and no groups.
#[derive(Default)]
pub struct VerilogBackend;

/// Checks to make sure that there are no holes being
/// used in a guard.
fn validate_guard(guard: &ir::Guard) -> bool {
    match guard {
        Guard::Or(left, right) | Guard::And(left, right) => {
            validate_guard(left) && validate_guard(right)
        }
        Guard::Eq(left, right)
        | Guard::Neq(left, right)
        | Guard::Gt(left, right)
        | Guard::Lt(left, right)
        | Guard::Geq(left, right)
        | Guard::Leq(left, right) => {
            !left.borrow().is_hole() && !right.borrow().is_hole()
        }
        Guard::Not(inner) => validate_guard(inner),
        Guard::Port(port) => !port.borrow().is_hole(),
        Guard::True => true,
    }
}

/// Returns `Ok` if there are no groups defined.
fn validate_structure(groups: &[RRC<Group>]) -> FutilResult<()> {
    for group in groups {
        for asgn in &group.borrow().assignments {
            let port = asgn.dst.borrow();
            // check if port is a hole
            if port.is_hole() {
                return Err(Error::MalformedStructure(
                    "Groups / Holes can not be turned into Verilog".to_string(),
                ));
            }

            // validate guard
            if !validate_guard(&asgn.guard) {
                return Err(Error::MalformedStructure(
                    "Groups / Holes can not be turned into Verilog".to_string(),
                ));
            };
        }
    }
    Ok(())
}

/// Returns `Ok` if the control for `comp` is either a single `enable`
/// or `empty`.
fn validate_control(ctrl: &ir::Control) -> FutilResult<()> {
    match ctrl {
        Control::Empty(_) => Ok(()),
        _ => Err(Error::MalformedControl("Control must be empty".to_string())),
    }
}

impl Backend for VerilogBackend {
    fn name(&self) -> &'static str {
        "verilog"
    }

    fn validate(ctx: &ir::Context) -> FutilResult<()> {
        for component in &ctx.components {
            validate_structure(&component.groups)?;
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
    ) -> FutilResult<()> {
        for extern_path in &ctx.lib.paths {
            let mut ext = File::open(extern_path).map_err(|err| {
                let std::io::Error { .. } = err;
                Error::WriteError(format!("File not found: {}", extern_path))
            })?;
            io::copy(&mut ext, &mut file.get_write()).map_err(|err| {
                let std::io::Error { .. } = err;
                Error::WriteError(format!(
                    "File not found: {}",
                    file.as_path_string()
                ))
            })?;
        }
        Ok(())
    }

    fn emit(ctx: &ir::Context, file: &mut OutputFile) -> FutilResult<()> {
        let modules = &ctx
            .components
            .iter()
            .map(|comp| emit_component(&comp, !ctx.synthesis_mode).to_string())
            .collect::<Vec<_>>();

        write!(file.get_write(), "{}", modules.join("\n")).map_err(|err| {
            let std::io::Error { .. } = err;
            Error::WriteError(format!(
                "File not found: {}",
                file.as_path_string()
            ))
        })?;
        Ok(())
    }
}

fn emit_component(comp: &ir::Component, memory_simulation: bool) -> v::Module {
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

    // Add memory initial and final blocks
    if memory_simulation {
        memory_read_write(&comp).into_iter().for_each(|stmt| {
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
    let mut initial = v::ParallelProcess::new_initial();
    wires.iter().for_each(|(name, width, dir)| {
        if *dir == ir::Direction::Input {
            // HACK: this is not the right way to reset
            // registers. we should have real reset ports.
            let value = String::from("0");
            // let value = if name.contains("write_en") {
            //     String::from("0")
            // } else {
            //     String::from("0")
            // };
            initial.add_seq(v::Sequential::new_blk_assign(
                v::Expr::new_ref(name),
                v::Expr::new_ulit_dec(*width as u32, &value),
            ));
        }
    });
    module.add_process(initial);

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

    map.values()
        .sorted_by_key(|(port, _)| port.borrow().canonical())
        .for_each(|asgns| {
            module.add_stmt(v::Stmt::new_parallel(emit_assignment(asgns)));
        });

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

            if let ir::CellType::Primitive { param_binding, .. } =
                &cell.prototype
            {
                param_binding.iter().for_each(|(name, width)| {
                    inst.add_param(
                        name.as_ref(),
                        v::Expr::new_int(*width as i32),
                    )
                })
            }

            for port in &cell.ports {
                inst.connect(
                    port.borrow().name.as_ref(),
                    port_to_ref(Rc::clone(port)),
                );
            }
            Some(inst)
        }
        None => None,
    }
}

fn emit_assignment(
    (dst_ref, assignments): &(RRC<ir::Port>, Vec<&ir::Assignment>),
) -> v::Parallel {
    let dst = dst_ref.borrow();
    let init = v::Expr::new_ulit_dec(dst.width as u32, &0.to_string());
    let rhs = assignments.iter().rfold(init, |acc, e| {
        let guard = guard_to_expr(&e.guard);
        let asgn = port_to_ref(Rc::clone(&e.src));
        v::Expr::new_mux(guard, asgn, acc)
    });
    v::Parallel::ParAssign(port_to_ref(Rc::clone(dst_ref)), rhs)
}

fn port_to_ref(port_ref: RRC<ir::Port>) -> v::Expr {
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
        Guard::Eq(..) => v::Expr::new_eq,
        Guard::Neq(..) => v::Expr::new_neq,
        Guard::Gt(..) => v::Expr::new_gt,
        Guard::Lt(..) => v::Expr::new_lt,
        Guard::Geq(..) => v::Expr::new_geq,
        Guard::Leq(..) => v::Expr::new_leq,
        Guard::Not(..) | Guard::Port(..) | Guard::True => unreachable!(),
    };

    match guard {
        Guard::And(l, r) | Guard::Or(l, r) => {
            op(guard)(guard_to_expr(l), guard_to_expr(r))
        }
        Guard::Neq(l, r)
        | Guard::Eq(l, r)
        | Guard::Gt(l, r)
        | Guard::Lt(l, r)
        | Guard::Geq(l, r)
        | Guard::Leq(l, r) => {
            op(guard)(port_to_ref(Rc::clone(l)), port_to_ref(Rc::clone(r)))
        }
        Guard::Not(o) => v::Expr::new_not(guard_to_expr(o)),
        Guard::Port(p) => port_to_ref(Rc::clone(p)),
        Guard::True => v::Expr::new_ulit_bin(1, &1.to_string()),
    }
}

//==========================================
//        Memory input and output
//==========================================
/// Generates code of the form:
/// ```
/// import "DPI-C" function string futil_getenv (input string env_var);
/// string DATA;
/// initial begin
///   DATA = futil_getenv("DATA");
///   $fdisplay(2, "DATA: %s", DATA);
///   $readmemh({DATA, "/<mem_name>.dat"}, <mem_name>.mem);
///   ...
/// end
/// final begin
///   $writememh({DATA, "/<mem_name>.out"}, <mem_name>.mem);
/// end
/// ```
fn memory_read_write(comp: &ir::Component) -> Vec<v::Stmt> {
    // Import futil helper library.
    let import_stmt = v::Stmt::new_rawstr(
        "import \"DPI-C\" function string futil_getenv (input string env_var);"
            .to_string(),
    );
    let data_decl = v::Stmt::new_rawstr("string DATA;".to_string());

    let mut initial_block = v::ParallelProcess::new_initial();
    initial_block
        // get the data
        .add_seq(v::Sequential::new_blk_assign(
            v::Expr::new_ref("DATA"),
            v::Expr::new_call("futil_getenv", vec![v::Expr::new_str("DATA")]),
        ))
        // log the path to the data
        .add_seq(v::Sequential::new_seqexpr(v::Expr::new_call(
            "$fdisplay",
            vec![
                v::Expr::new_int(2),
                v::Expr::new_str("DATA (path to meminit files): %s"),
                v::Expr::new_ref("DATA"),
            ],
        )));

    let memories = comp.cells.iter().filter_map(|cell| {
        let is_external = cell.borrow().get_attribute("external").is_some();
        if is_external
            && cell
                .borrow()
                .type_name()
                .map(|proto| proto.id.contains("mem"))
                .unwrap_or_default()
        {
            Some(cell.borrow().name.id.clone())
        } else {
            None
        }
    });

    memories.clone().for_each(|name| {
        initial_block.add_seq(v::Sequential::new_seqexpr(v::Expr::new_call(
            "$readmemh",
            vec![
                v::Expr::Concat(v::ExprConcat {
                    exprs: vec![
                        v::Expr::new_str(&format!("/{}.dat", name)),
                        v::Expr::new_ref("DATA"),
                    ],
                }),
                v::Expr::new_ipath(&format!("{}.mem", name)),
            ],
        )));
    });

    let mut final_block = v::ParallelProcess::new_final();
    memories.for_each(|name| {
        final_block.add_seq(v::Sequential::new_seqexpr(v::Expr::new_call(
            "$writememh",
            vec![
                v::Expr::Concat(v::ExprConcat {
                    exprs: vec![
                        v::Expr::new_str(&format!("/{}.out", name)),
                        v::Expr::new_ref("DATA"),
                    ],
                }),
                v::Expr::new_ipath(&format!("{}.mem", name)),
            ],
        )));
    });

    vec![
        import_stmt,
        data_decl,
        v::Stmt::new_parallel(v::Parallel::new_process(initial_block)),
        v::Stmt::new_parallel(v::Parallel::new_process(final_block)),
    ]
}
