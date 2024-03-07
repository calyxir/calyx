//! FIRRTL backend for the Calyx compiler.
//!
//! Transforms an [`ir::Context`](crate::ir::Context) into a formatted string that represents a
//! valid FIRRTL program.

use crate::{traits::Backend, VerilogBackend};
use calyx_ir::{self as ir, Binding, RRC};
use calyx_utils::{CalyxResult, Id, OutputFile};
use ir::Port;
use std::cell::RefCell;
use std::collections::HashSet;
use std::io;

pub(super) const SPACING: &str = "    ";

/// Implements a simple FIRRTL backend. The backend only accepts Calyx programs with no control
/// and no groups.
#[derive(Default)]
pub struct FirrtlBackend;

impl Backend for FirrtlBackend {
    fn name(&self) -> &'static str {
        "firrtl"
    }

    fn link_externs(
        _prog: &calyx_ir::Context,
        _write: &mut calyx_utils::OutputFile,
    ) -> calyx_utils::CalyxResult<()> {
        Ok(()) // FIXME: Need to implement
    }

    fn validate(prog: &calyx_ir::Context) -> calyx_utils::CalyxResult<()> {
        VerilogBackend::validate(prog) // FIXME: would this work if we wanted to check for the same things?
    }

    fn emit(ctx: &ir::Context, file: &mut OutputFile) -> CalyxResult<()> {
        let out = &mut file.get_write();
        writeln!(out, "circuit {}:", ctx.entrypoint)?;
        if ctx.bc.emit_primitive_extmodules {
            emit_extmodules(ctx, out)?;
        }
        for comp in ctx.components.iter() {
            emit_component(comp, out)?
        }
        Ok(())
    }
}

fn emit_extmodules<F: io::Write>(
    ctx: &ir::Context,
    out: &mut F,
) -> Result<(), calyx_utils::Error> {
    let mut extmodule_set: HashSet<String> = HashSet::new();
    for comp in &ctx.components {
        for cell in comp.cells.iter() {
            let cell_borrowed = cell.as_ref().borrow();
            if let ir::CellType::Primitive {
                name,
                param_binding,
                ..
            } = &cell_borrowed.prototype
            {
                let curr_module_name =
                    get_primitive_module_name(name, param_binding);
                if extmodule_set.insert(curr_module_name.clone()) {
                    emit_primitive_extmodule(
                        cell.borrow().ports(),
                        &curr_module_name,
                        name,
                        param_binding,
                        out,
                    )?;
                }
            };
        }
    }
    Ok(())
}

// TODO: Ask about the other backend configurations in verilog.rs and see if I need any of it
fn emit_component<F: io::Write>(
    comp: &ir::Component,
    f: &mut F,
) -> io::Result<()> {
    let mut dst_set: HashSet<ir::Canonical> = HashSet::new();

    writeln!(f, "{}module {}:", SPACING, comp.name)?;

    // Inputs and Outputs
    let sig = comp.signature.borrow();
    for port_ref in &sig.ports {
        let port = port_ref.borrow();
        emit_port(port, true, f)?;
    }

    // write invalid statements for all output ports.
    for port_ref in sig.ports.iter() {
        let port = port_ref.as_ref();
        if port.borrow().direction == calyx_frontend::Direction::Input {
            write_invalid_initialization(port, f)?;
            dst_set.insert(port.borrow().canonical());
        }
    }

    // Add a COMPONENT START: <name> anchor before any code in the component
    writeln!(f, "{}; COMPONENT START: {}", SPACING.repeat(2), comp.name)?;

    // Cells
    for cell in comp.cells.iter() {
        let cell_borrowed = cell.as_ref().borrow();
        if cell_borrowed.type_name().is_some() {
            let module_name = match &cell_borrowed.prototype {
                ir::CellType::Primitive {
                    name,
                    param_binding,
                    is_comb: _,
                    latency: _,
                } => get_primitive_module_name(name, param_binding),
                ir::CellType::Component { name } => name.to_string(),
                _ => unreachable!(),
            };
            writeln!(
                f,
                "{}inst {} of {}",
                SPACING.repeat(2),
                cell_borrowed.name(),
                module_name
            )?;
        }
    }

    // Emit assignments
    for asgn in &comp.continuous_assignments {
        match asgn.guard.as_ref() {
            ir::Guard::True => {
                // Simple assignment with no guard
                let _ = write_assignment(asgn, f, 2);
            }
            _ => {
                let dst_canonical = asgn.dst.as_ref().borrow().canonical();
                if !dst_set.contains(&dst_canonical) {
                    // if we don't have a "is invalid" statement yet, then we have to write one.
                    // an alternative "eager" approach would be to instantiate all possible ports
                    // (our output ports + all children's input ports) up front.
                    write_invalid_initialization(&asgn.dst, f)?;
                    dst_set.insert(dst_canonical);
                }
                // need to write out the guard.
                let guard_string = get_guard_string(asgn.guard.as_ref());
                writeln!(f, "{}when {}:", SPACING.repeat(2), guard_string)?;
                write_assignment(asgn, f, 3)?;
            }
        }
    }

    // Add COMPONENT END: <name> anchor
    writeln!(f, "{}; COMPONENT END: {}\n", SPACING.repeat(2), comp.name)?;

    Ok(())
}

// creates a FIRRTL module name given the internal information of a primitive.
fn get_primitive_module_name(name: &Id, param_binding: &Binding) -> String {
    let mut primitive_string = name.to_string();
    for (_, size) in param_binding.as_ref().iter() {
        primitive_string.push('_');
        primitive_string.push_str(&size.to_string());
    }
    primitive_string
}

fn emit_primitive_extmodule<F: io::Write>(
    ports: &[RRC<Port>],
    curr_module_name: &String,
    name: &Id,
    param_binding: &Binding,
    f: &mut F,
) -> io::Result<()> {
    writeln!(f, "{}extmodule {}:", SPACING, curr_module_name)?;
    for port in ports {
        let port_borrowed = port.borrow();
        emit_port(port_borrowed, false, f)?;
    }
    writeln!(f, "{}defname = {}", SPACING.repeat(2), name)?;
    for (id, size) in param_binding.as_ref().iter() {
        writeln!(f, "{}parameter {} = {}", SPACING.repeat(2), id, size)?;
    }
    writeln!(f)?;
    Ok(())
}

fn emit_port<F: io::Write>(
    port: std::cell::Ref<'_, Port>,
    reverse_direction: bool,
    f: &mut F,
) -> Result<(), io::Error> {
    let direction_string = match port.direction {
        calyx_frontend::Direction::Input => {
            if reverse_direction {
                "output"
            } else {
                "input"
            }
        }
        calyx_frontend::Direction::Output => {
            if reverse_direction {
                "input"
            } else {
                "output"
            }
        }
        calyx_frontend::Direction::Inout => {
            panic!("Unexpected Inout port on Component: {}", port.name)
        }
    };
    if port.has_attribute(ir::BoolAttr::Clk) {
        writeln!(
            f,
            "{}{} {}: Clock",
            SPACING.repeat(2),
            direction_string,
            port.name
        )?;
    } else {
        writeln!(
            f,
            "{}{} {}: UInt<{}>",
            SPACING.repeat(2),
            direction_string,
            port.name,
            port.width
        )?;
    };
    Ok(())
}

// fn create_primitive_extmodule() {}

// recursive function that writes the FIRRTL representation for a guard.
fn get_guard_string(guard: &ir::Guard<ir::Nothing>) -> String {
    match guard {
        ir::Guard::Or(l, r) => {
            let l_str = get_guard_string(l.as_ref());
            let r_str = get_guard_string(r.as_ref());
            format!("or({}, {})", l_str, r_str)
        }
        ir::Guard::And(l, r) => {
            let l_str = get_guard_string(l.as_ref());
            let r_str = get_guard_string(r.as_ref());
            format!("and({}, {})", l_str, r_str)
        }
        ir::Guard::Not(g) => {
            let g_str = get_guard_string(g);
            format!("not({})", g_str)
        }
        ir::Guard::True => String::from(""),
        ir::Guard::CompOp(op, l, r) => {
            let l_str = get_port_string(&l.borrow(), false);
            let r_str = get_port_string(&r.borrow(), false);
            let op_str = match op {
                ir::PortComp::Eq => "eq",
                ir::PortComp::Neq => "neq",
                ir::PortComp::Gt => "gt",
                ir::PortComp::Lt => "lt",
                ir::PortComp::Geq => "geq",
                ir::PortComp::Leq => "leq",
            };
            format!("{}({}, {})", op_str, l_str, r_str)
        }
        ir::Guard::Port(port) => get_port_string(&port.borrow(), false),
        ir::Guard::Info(_) => {
            panic!("guard should not have info")
        }
    }
}

// returns the FIRRTL translation of a port.
// if is_dst is true, then the port is a destination of an assignment, and shouldn't be a constant.
fn get_port_string(port: &calyx_ir::Port, is_dst: bool) -> String {
    match &port.parent {
        ir::PortParent::Cell(cell) => {
            let parent_ref = cell.upgrade();
            let parent = parent_ref.borrow();
            match parent.prototype {
                ir::CellType::Constant { val, width: _ } => {
                    if !is_dst {
                        format!("UInt({})", val)
                    } else {
                        unreachable!()
                    }
                }
                ir::CellType::ThisComponent => String::from(port.name.as_ref()),
                _ => {
                    format!("{}.{}", parent.name().as_ref(), port.name.as_ref())
                }
            }
        }
        _ => {
            unreachable!("Groups should not be parents as this backend takes place after compiler passes.")
        }
    }
}

// variables that get set in assignments should get initialized to avoid the FIRRTL compiler from erroring.
fn write_invalid_initialization<F: io::Write>(
    port: &RefCell<ir::Port>,
    f: &mut F,
) -> io::Result<()> {
    let default_initialization_str = "; default initialization";
    let dst_string = get_port_string(&port.borrow(), true);
    if port.borrow().attributes.has(ir::BoolAttr::Control) {
        writeln!(
            f,
            "{}{} <= UInt(0) {}",
            SPACING.repeat(2),
            dst_string,
            default_initialization_str
        )
    } else {
        writeln!(
            f,
            "{}{} is invalid {}",
            SPACING.repeat(2),
            dst_string,
            default_initialization_str
        )?;
        writeln!(f, "{}{} <= UInt(0)", SPACING.repeat(2), dst_string)
    }
}

// Writes a FIRRTL assignment
fn write_assignment<F: io::Write>(
    asgn: &ir::Assignment<ir::Nothing>,
    f: &mut F,
    num_indent: usize,
) -> io::Result<()> {
    let dest_port = asgn.dst.borrow();
    let dest_string = get_port_string(&dest_port, true);
    let source_port = asgn.src.borrow();
    let src_string = get_port_string(&source_port, false);
    writeln!(
        f,
        "{}{} <= {}",
        SPACING.repeat(num_indent),
        dest_string,
        src_string
    )?;
    Ok(())
}
