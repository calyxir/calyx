//! Resource estimation backend for the Calyx compiler.
//! Transforms an [`ir::Context`](crate::ir::Context) into a CSV that
//! counts the number of different primitives in a program and loosely
//! estimates the total size of the generated hardware.

use std::collections::HashMap;
use std::fmt::Write;
use std::vec;

use crate::backend::traits::Backend;
use calyx_ir as ir;
use calyx_utils::{CalyxResult, OutputFile};

#[derive(Default)]
pub struct ResourcesBackend;

impl Backend for ResourcesBackend {
    fn name(&self) -> &'static str {
        "resources"
    }

    /// OK to run this analysis on any Calyx program
    fn validate(_ctx: &ir::Context) -> CalyxResult<()> {
        Ok(())
    }

    /// Don't need to take care of this for this pass
    fn link_externs(
        _ctx: &ir::Context,
        _file: &mut OutputFile,
    ) -> CalyxResult<()> {
        Ok(())
    }

    fn emit(ctx: &ir::Context, file: &mut OutputFile) -> CalyxResult<()> {
        let main_comp = ctx
            .components
            .iter()
            .find(|comp| comp.name == ctx.entrypoint)
            .unwrap();

        let count_map: &mut HashMap<(ir::Id, ir::Binding, bool), u32> =
            &mut HashMap::new();

        gen_count_map(ctx, main_comp, count_map);

        write_csv(count_map.clone(), file);

        estimated_size(count_map.clone());

        Ok(())
    }
}

/// Counts the number of each primitive with a given set of parameters
/// in the program with entrypoint `main_comp`.
fn gen_count_map(
    ctx: &ir::Context,
    main_comp: &ir::Component,
    count_map: &mut HashMap<(ir::Id, ir::Binding, bool), u32>,
) {
    for cell in main_comp.cells.iter() {
        let cell_ref = cell.borrow();
        match &cell_ref.prototype {
            ir::CellType::Primitive {
                name,
                param_binding,
                ..
            } => {
                *count_map
                    .entry((
                        *name,
                        (**param_binding).clone(),
                        cell_ref.get_attribute("external").is_some(),
                    ))
                    .or_insert(0) += 1;
            }
            ir::CellType::Component { name } => {
                let component = ctx
                    .components
                    .iter()
                    .find(|comp| comp.name == name)
                    .unwrap();
                gen_count_map(ctx, component, count_map);
            }
            _ => (),
        }
    }
}

/// Writes a CSV to stdout with primitive count information
/// generated by `gen_count_map`.
fn write_csv(
    count_map: HashMap<(ir::Id, ir::Binding, bool), u32>,
    file: &mut OutputFile,
) {
    let mut wtr = csv::Writer::from_writer(file.get_write());
    let header = vec!["Primitive", "Count", "External?", "Parameters"];
    wtr.write_record(header).unwrap();
    for ((name, params, is_external), count) in count_map {
        let mut result = vec![];
        result.push(name.id.to_string());
        result.push(count.to_string());
        result.push(if is_external {
            "yes".to_string()
        } else {
            "no".to_string()
        });
        let mut param_vals = String::new();
        for (id, val) in params {
            write!(param_vals, "{}: {}. ", &id.id, val).unwrap();
        }
        result.push(param_vals);
        wtr.write_record(result).unwrap();
    }
    wtr.flush().ok();
}

// Prints the estimated size (in bits) of the generated hardware along with a breakdown
// of which primitives contributed to the total number.
// TODO (priya): Add other primitives
fn estimated_size(count_map: HashMap<(ir::Id, ir::Binding, bool), u32>) {
    let mut estimated_size: u64 = 0;
    let mut estimated_external_size: u64 = 0;
    let mut add_size =
        |is_external: bool, count: u64, bitwidth: u64, slots: Option<u64>| {
            let size = count * bitwidth * slots.unwrap_or(1);
            if is_external {
                estimated_external_size += size;
            } else {
                estimated_size += size;
            }
        };
    let externalize_name = |name: ir::Id, is_external: bool| {
        let id = if is_external {
            format!("external {}", name)
        } else {
            name.to_string()
        };
        id
    };
    eprintln!("Summary of primitives:");
    for ((name, params, is_external), count) in count_map {
        match name.as_ref() {
            "std_reg" => {
                add_size(is_external, count as u64, params[0].1, None);
                eprintln!(
                    "{} {} primitive(s) with a bitwidth of {}.",
                    count,
                    externalize_name(name, is_external),
                    params[0].1
                )
            }
            "std_mem_d1" => {
                add_size(
                    is_external,
                    count as u64,
                    params[0].1,
                    Some(params[1].1),
                );
                eprintln!(
                    "{} {} primitive(s) with {} slot(s) of memory, each {} bit(s) wide.",
                    count, externalize_name(name, is_external),
                    params[1].1, params[0].1);
            }
            "std_mem_d2" => {
                add_size(
                    is_external,
                    count as u64,
                    params[0].1,
                    Some(params[1].1 * params[2].1),
                );
                eprintln!(
                    "{} {} primitive(s) with {} slot(s) of memory, each {} bit(s) wide.",
                    count, externalize_name(name, is_external),
                    (params[1].1 * params[2].1), params[0].1);
            }
            "std_mem_d3" => {
                add_size(
                    is_external,
                    count as u64,
                    params[0].1,
                    Some(params[1].1 * params[2].1 * params[3].1),
                );
                eprintln!(
                    "{} {} primitive(s) with {} slot(s) of memory, each {} bit(s) wide.",
                    count, externalize_name(name, is_external),
                    (params[1].1 * params[2].1 * params[3].1), params[0].1);
            }
            "std_mem_d4" => {
                add_size(
                    is_external,
                    count as u64,
                    params[0].1,
                    Some(params[1].1 * params[2].1 * params[3].1 * params[4].1),
                );
                eprintln!(
                    "{} {} primitive(s) with {} slot(s) of memory, each {} bit(s) wide.",
                    count, externalize_name(name, is_external),
                    (params[1].1 * params[2].1 * params[3].1 * params[4].1), params[0].1);
            }
            "seq_mem_d1" => {
                add_size(
                    is_external,
                    count as u64,
                    params[0].1,
                    Some(params[1].1),
                );
                eprintln!(
                    "{} {} primitive(s) with {} slot(s) of memory, each {} bit(s) wide.",
                    count, externalize_name(name, is_external),
                    params[1].1, params[0].1);
            }
            "seq_mem_d2" => {
                add_size(
                    is_external,
                    count as u64,
                    params[0].1,
                    Some(params[1].1 * params[2].1),
                );
                eprintln!(
                    "{} {} primitive(s) with {} slot(s) of memory, each {} bit(s) wide.",
                    count, externalize_name(name, is_external),
                    (params[1].1 * params[2].1), params[0].1);
            }
            "seq_mem_d3" => {
                add_size(
                    is_external,
                    count as u64,
                    params[0].1,
                    Some(params[1].1 * params[2].1 * params[3].1),
                );
                eprintln!(
                    "{} {} primitive(s) with {} slot(s) of memory, each {} bit(s) wide.",
                    count, externalize_name(name, is_external),
                    (params[1].1 * params[2].1 * params[3].1), params[0].1);
            }
            "seq_mem_d4" => {
                add_size(
                    is_external,
                    count as u64,
                    params[0].1,
                    Some(params[1].1 * params[2].1 * params[3].1 * params[4].1),
                );
                eprintln!(
                    "{} {} primitive(s) with {} slot(s) of memory, each {} bit(s) wide.",
                    count, externalize_name(name, is_external),
                    (params[1].1 * params[2].1 * params[3].1 * params[4].1), params[0].1);
            }
            _ => (),
        }
    }
    eprintln!("Estimated size in bit(s): {}", estimated_size);
    eprintln!(
        "Estimated external size in bit(s): {}",
        estimated_external_size
    );
}
