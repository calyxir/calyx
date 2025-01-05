//! SystemVerilog backend for the Calyx compiler.
//!
//! Transforms an [`ir::Context`](crate::ir::Context) into a formatted string that represents a
//! valid SystemVerilog program.

use crate::traits::Backend;
use calyx_ir::{self as ir, Control, FlatGuard, Group, Guard, GuardRef, RRC};
use calyx_utils::{CalyxResult, Error, OutputFile};
use ir::Nothing;
use itertools::Itertools;
use morty::{FileBundle, LibraryBundle};
use std::io;
use std::io::{Read, Seek, SeekFrom, Write};
use std::{collections::HashMap, collections::HashSet, path::PathBuf, rc::Rc};
use std::{fs::File, time::Instant};
use tempfile::NamedTempFile;
use vast::v17::ast as v;

/// Implements a simple Verilog backend. The backend only accepts Calyx programs with no control
/// and no groups.
#[derive(Default)]
pub struct VerilogBackend;

// input string should be the cell type name of a memory cell. In other words one
// of "seq/comb_mem_d_1/2/3/4". Becase we define seq_mem_d2/3/4 in terms of seq_mem_d1
// we need another layer of memory access to get the actual memory array in verilog
// for these mem types.
// In other words, for memories not defined in terms of another memory, we can just use
// "mem" to access them. But for memories defined in terms of another memory,
// which are seq_mem_d2/3/4, we need "mem.mem" to access them.
fn get_mem_str(mem_type: &str) -> &str {
    if mem_type.contains("d1") || mem_type.contains("comb_mem") {
        "mem"
    } else {
        "mem.mem"
    }
}

/// Checks to make sure that there are no holes being
/// used in a guard.
fn validate_guard(guard: &ir::Guard<Nothing>) -> bool {
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
        Guard::Info(_) => true,
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

/// Each external library has its own specific handler to build all information that Morty needs to do pickling. We can implement each command line argument (see https://github.com/pulp-platform/morty/blob/master/src/main.rs for available arguments) as a trait's method.
trait LibraryHandlerTrait {
    /// Add search path(s) for SystemVerilog includes to build a LibraryBundle
    fn add_incs(&self) -> CalyxResult<Vec<PathBuf>>;
    /// Directory(s) to search for SystemVerilog modules
    fn add_library_dirs(&self) -> CalyxResult<Vec<PathBuf>>;
    /// Define preprocesor macro(s)
    fn add_defs(&self) -> CalyxResult<HashMap<String, Option<String>>>;
    /// Add search path(s) for SystemVerilog includes to build a FileBundle
    fn add_stdin_incdirs(&self) -> CalyxResult<Vec<PathBuf>>;
    /// Add export include directories
    fn add_export_incdirs(&self) -> CalyxResult<HashMap<String, Vec<String>>>;
    /// Create a map from module name to file path
    fn map_module_names_to_file_paths(
        &self,
    ) -> CalyxResult<HashMap<String, PathBuf>> {
        // a hashmap from 'module name' to 'path' for all libraries.
        let mut library_files = HashMap::new();
        // a list of paths for all library files
        let mut library_paths: Vec<PathBuf> = Vec::new();

        // we first accumulate all library files from the 'library_dir' and 'library_file' options into
        // a vector of paths, and then construct the library hashmap.
        let library_dirs: Vec<PathBuf> = self.add_library_dirs()?;
        for dir in library_dirs {
            let entries = std::fs::read_dir(&dir).map_err(|e| {
                Error::invalid_file(format!(
                    "Error accessing library directory `{:?}`: {}",
                    dir, e
                ))
            })?;

            for entry in entries {
                let entry = entry.map_err(|e| {
                    Error::invalid_file(format!(
                        "Error reading entry in directory `{:?}`: {}",
                        dir, e
                    ))
                })?;
                library_paths.push(entry.path());
            }
        }

        for p in &library_paths {
            // Must have the library extension (.v or .sv).
            if morty::has_libext(p) {
                if let Some(m) = morty::lib_module(p) {
                    library_files.insert(m, p.to_owned());
                }
            }
        }

        Ok(library_files)
    }
}

/// Check if any special library is needed
fn check_library_needed(ctx: &ir::Context) -> bool {
    ctx.lib
        .extern_paths()
        .iter()
        .any(|path| path.to_string_lossy().contains("float"))
}

/// Collect all included files specified by the Calyx source file
fn collect_included_files(ctx: &ir::Context) -> Vec<String> {
    ctx.lib
        .extern_paths()
        .into_iter()
        .map(|pb| pb.to_string_lossy().into_owned())
        .collect()
}

/// Build the library bundle for all libraries needed for pickling
fn build_library_bundle(
    ctx: &ir::Context,
    calyx_emitted_file: &NamedTempFile,
    handlers: &Vec<Box<dyn LibraryHandlerTrait>>,
) -> CalyxResult<LibraryBundle> {
    let mut included_files = collect_included_files(ctx);
    included_files
        .push(calyx_emitted_file.path().to_string_lossy().to_string()); // `calyx_emitted_file` is used as part of the source files
    let mut include_dirs = Vec::new();
    let mut defines = HashMap::new();
    let mut files = HashMap::new();
    for handler in handlers {
        include_dirs.extend(handler.add_incs()?);
        defines.extend(handler.add_defs()?);
        files.extend(handler.map_module_names_to_file_paths()?);
    }
    let main_module = String::from(ctx.entrypoint.id.as_str());
    files.insert(main_module, calyx_emitted_file.path().to_path_buf());

    let include_dirs: Vec<String> = include_dirs
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    Ok(LibraryBundle {
        include_dirs,
        defines,
        files,
    })
}

/// Build a list of `FileBundle`s needed for building the syntax tree
fn derive_file_list(
    ctx: &ir::Context,
    file: &NamedTempFile,
    handlers: &Vec<Box<dyn LibraryHandlerTrait>>,
) -> CalyxResult<Vec<FileBundle>> {
    let mut file_bundles = Vec::new();
    for handler in handlers {
        let stdin_incdirs = handler.add_stdin_incdirs()?;
        let include_dirs: Vec<String> = stdin_incdirs
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        let export_incdirs = handler.add_export_incdirs()?;
        let defines = handler.add_defs()?;
        let mut files = handler.map_module_names_to_file_paths()?;

        let main_module = String::from(ctx.entrypoint.id.as_str());
        files.insert(main_module, file.path().to_path_buf());

        let mut included_files = collect_included_files(ctx);
        for lib_file in files.values() {
            included_files.push(String::from(lib_file.to_str().unwrap()));
        }

        file_bundles.push(FileBundle {
            include_dirs,
            export_incdirs,
            defines,
            files: included_files,
        });
    }
    Ok(file_bundles)
}

impl Backend for VerilogBackend {
    fn name(&self) -> &'static str {
        "verilog"
    }

    fn validate(ctx: &ir::Context) -> CalyxResult<()> {
        for component in &ctx.components {
            validate_structure(component.get_groups().iter())?;
            validate_control(&component.control.borrow())?;
        }
        Ok(())
    }

    /// If no special libraries are needed, generate a "fat" library by copy-pasting all of the extern files.
    fn link_externs(
        ctx: &ir::Context,
        file: &mut OutputFile,
    ) -> CalyxResult<()> {
        // If we need special libraries (like HardFloat), run Morty to pickle the files in the `emit` stage. We postpone linking extern special libraries because Morty needs all emitted information to do pickle. We could soley use Morty, but currently it eliminates the body inside `ifndef-endif`. Also discussed in here: https://github.com/pulp-platform/morty/issues/49
        if !check_library_needed(ctx) {
            let fw = &mut file.get_write();
            for extern_path in &ctx.lib.extern_paths() {
                let mut ext = File::open(extern_path).unwrap();
                io::copy(&mut ext, fw).map_err(|err| {
                    let std::io::Error { .. } = err;
                    Error::write_error(format!(
                        "File not found: {}",
                        file.as_path_string()
                    ))
                })?;
                writeln!(fw)?;
            }
        }

        Ok(())
    }

    fn emit(ctx: &ir::Context, file: &mut OutputFile) -> CalyxResult<()> {
        // Create a temporary file as an intermediate storage to emit inline primtives and components to. This temporary file will be used as one of the source SystemVerilog file for Morty to do pickle.
        let temp_file = tempfile::NamedTempFile::new().map_err(|_| {
            Error::write_error("Failed to create a temporary file".to_string())
        })?;
        let mut temp_writer = temp_file.as_file();

        // Write inline primitives
        for (prim, _) in ctx.lib.prim_inlines() {
            emit_prim_inline(prim, &mut temp_writer)?;
        }

        let comps = ctx.components.iter().try_for_each(|comp| {
            // Time the generation of the component.
            let time = Instant::now();
            let out = emit_component(
                comp,
                ctx.bc.synthesis_mode,
                ctx.bc.enable_verification,
                ctx.bc.flat_assign,
                &mut temp_writer,
            );
            log::info!("Generated `{}` in {:?}", comp.name, time.elapsed());
            out
        });
        comps.map_err(|err| {
            let std::io::Error { .. } = err;
            Error::write_error(format!(
                "File not found: {}",
                file.as_path_string()
            ))
        })?;

        if check_library_needed(ctx) {
            let handlers: Vec<Box<dyn LibraryHandlerTrait>> = Vec::new();
            // Special libraries (like HardFloat) are needed, run Morty to pickle the files
            let library_bundle =
                build_library_bundle(ctx, &temp_file, &handlers)?;

            let file_list = derive_file_list(ctx, &temp_file, &handlers)?;
            let syntax_trees = morty::build_syntax_tree(
                &file_list, false, // By default, don't strip comments
                false, // By default, don't ignore unparseable
                true, // By default, propagate defines from first files to the following files
                false, // By default, don't force sequential
            )
            .unwrap();
            let top_module = ctx.entrypoint.to_string();
            let _pickled = morty::do_pickle(
                None::<&String>, // By default, don't need to prepend a name to all global names
                None::<&String>, // By default, don't need to append a name to all global names
                HashSet::new(), // By default, don't specify module, interface, package that should not be renamed; instead, let morty to dictate the behavior
                HashSet::new(), // By default, don't specify module, interface, package that shouldn't be included in the pickled file list; instead, we let morty to dictate the behavior
                library_bundle,
                syntax_trees,
                Box::new(temp_file.reopen()?) as Box<dyn Write>,
                Some(&top_module),
                true, // By default, keep defines to prevent removal of `define` statements
                true, // By default, propagate defines from first files to the following files
                false, // By default, keep the time units
            )
            .map_err(|err| {
                Error::write_error(format!("{}", err.to_string()))
            })?;
        }
        // Rewind to the start of the temporary file so that we can read the content
        temp_writer.seek(SeekFrom::Start(0)).map_err(|_| {
            Error::write_error(
                "Failed to rewind the temporary file".to_string(),
            )
        })?;
        // Read from the temporary file and write to the user-specified output `file`
        let mut temp_content = String::new();
        temp_writer.read_to_string(&mut temp_content).map_err(|_| {
            Error::write_error("Failed to read from temporary file".to_string())
        })?;

        let mut final_writer = file.get_write();
        final_writer
            .write_all(temp_content.as_bytes())
            .map_err(|err| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Write failed: {}", err),
                )
            })?;
        Ok(())
    }
}

// takes an inlined primitive and emits the corresponding verilog
// note that this means that prim *must* have Some body
fn emit_prim_inline<F: io::Write>(
    prim: &ir::Primitive,
    f: &mut F,
) -> CalyxResult<()> {
    write!(f, "module {}", prim.name)?;
    if !prim.params.is_empty() {
        writeln!(f, " #(")?;
        for (idx, param) in prim.params.iter().enumerate() {
            write!(f, "    parameter {} = 32", param)?;
            if idx != prim.params.len() - 1 {
                writeln!(f, ",")?;
            } else {
                writeln!(f)?;
            }
        }
        write!(f, ")")?;
    }
    writeln!(f, " (")?;
    for (idx, port) in prim.signature.iter().enumerate() {
        // NOTE: The signature port definitions are reversed inside the component.
        match port.direction {
            ir::Direction::Input => {
                write!(f, "   input wire")?;
            }
            ir::Direction::Output => {
                write!(f, "   output")?;
            }
            ir::Direction::Inout => {
                panic!("Unexpected Inout port on Component: {}", port.name())
            }
        }
        match port.width {
            ir::Width::Const { value } => {
                if value == 1 {
                    write!(f, " logic {}", port.name())?;
                } else {
                    write!(f, " logic [{}:0] {}", value - 1, port.name())?;
                }
            }
            ir::Width::Param { value } => {
                write!(f, " logic [{}-1:0] {}", value, port.name())?;
            }
        }
        if idx == prim.signature.len() - 1 {
            writeln!(f)?;
        } else {
            writeln!(f, ",")?;
        }
    }
    writeln!(f, ");")?;

    writeln!(
        f,
        "{}",
        prim.body.as_ref().unwrap_or_else(|| panic!(
            "expected primitive {} to have a body",
            prim.name
        ))
    )?;

    writeln!(f, "endmodule")?;
    writeln!(f)?;

    Ok(())
}

fn emit_component<F: io::Write>(
    comp: &ir::Component,
    synthesis_mode: bool,
    enable_verification: bool,
    flat_assign: bool,
    f: &mut F,
) -> io::Result<()> {
    writeln!(f, "module {}(", comp.name)?;

    let sig = comp.signature.borrow();
    for (idx, port_ref) in sig.ports.iter().enumerate() {
        let port = port_ref.borrow();
        // NOTE: The signature port definitions are reversed inside the component.
        match port.direction {
            ir::Direction::Input => {
                write!(f, "  output")?;
            }
            ir::Direction::Output => {
                write!(f, "  input")?;
            }
            ir::Direction::Inout => {
                panic!("Unexpected Inout port on Component: {}", port.name)
            }
        }
        if port.width == 1 {
            write!(f, " logic {}", port.name)?;
        } else {
            write!(f, " logic [{}:0] {}", port.width - 1, port.name)?;
        }
        if idx == sig.ports.len() - 1 {
            writeln!(f)?;
        } else {
            writeln!(f, ",")?;
        }
    }
    writeln!(f, ");")?;

    // Add a COMPONENT START: <name> anchor before any code in the component
    writeln!(f, "// COMPONENT START: {}", comp.name)?;

    // Add memory initial and final blocks
    if !synthesis_mode {
        memory_read_write(comp)
            .into_iter()
            .try_for_each(|stmt| writeln!(f, "{}", stmt))?;
    }

    let cells = comp
        .cells
        .iter()
        .flat_map(|cell| wire_decls(&cell.borrow()))
        .collect_vec();
    // structure wire declarations
    cells.iter().try_for_each(|(name, width, _)| {
        let decl = v::Decl::new_logic(name, *width);
        writeln!(f, "{};", decl)
    })?;

    // cell instances
    comp.cells
        .iter()
        .filter_map(|cell| cell_instance(&cell.borrow()))
        .try_for_each(|instance| writeln!(f, "{instance}"))?;

    // gather assignments keyed by destination
    let mut map: HashMap<_, (RRC<ir::Port>, Vec<_>)> = HashMap::new();
    for asgn in &comp.continuous_assignments {
        map.entry(asgn.dst.borrow().canonical())
            .and_modify(|(_, v)| v.push(asgn))
            .or_insert((Rc::clone(&asgn.dst), vec![asgn]));
    }

    // Flatten all the guard expressions.
    let mut pool = ir::GuardPool::new();
    let grouped_asgns: Vec<_> = map
        .values()
        .sorted_by_key(|(port, _)| port.borrow().canonical())
        .map(|(dst, asgns)| {
            let flat_asgns: Vec<_> = asgns
                .iter()
                .map(|asgn| {
                    let guard = pool.flatten(&asgn.guard);
                    (asgn.src.clone(), guard)
                })
                .collect();
            (dst, flat_asgns)
        })
        .collect();

    if flat_assign {
        // Emit "flattened" assignments as ANF statements.
        // Emit Verilog for the flattened guards.
        for (idx, guard) in pool.iter() {
            write!(f, "wire {} = ", VerilogGuardRef(idx))?;
            emit_guard(guard, f)?;
            writeln!(f, ";")?;
        }

        // Emit assignments using these guards.
        for (dst, asgns) in &grouped_asgns {
            emit_assignment_flat(dst, asgns, f)?;

            if enable_verification {
                if let Some(check) =
                    emit_guard_disjoint_check(dst, asgns, &pool, true)
                {
                    writeln!(f, "always_comb begin")?;
                    writeln!(f, "  {check}")?;
                    writeln!(f, "end")?;
                }
            }
        }
    } else {
        // Build a top-level always block to contain verilator checks for assignments
        let mut checks = v::ParallelProcess::new_always_comb();

        // Emit nested assignments.
        for (dst, asgns) in grouped_asgns {
            let stmt =
                v::Stmt::new_parallel(emit_assignment(dst, &asgns, &pool));
            writeln!(f, "{stmt}")?;

            if enable_verification {
                if let Some(check) =
                    emit_guard_disjoint_check(dst, &asgns, &pool, false)
                {
                    checks.add_seq(check);
                }
            }
        }

        if !synthesis_mode {
            writeln!(f, "{checks}")?;
        }
    }

    // Add COMPONENT END: <name> anchor
    writeln!(f, "// COMPONENT END: {}\nendmodule", comp.name)?;
    Ok(())
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
            ir::PortParent::StaticGroup(_) => unreachable!(),
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
                    inst.add_param(
                        wn.id.as_str(),
                        v::Expr::new_int(*width as i32),
                    );
                    inst.add_param(
                        vn.id.as_str(),
                        v::Expr::new_ulit_dec(
                            *width as u32,
                            &value.to_string(),
                        ),
                    );
                } else {
                    param_binding.iter().for_each(|(name, value)| {
                    if *value > (i32::MAX as u64) {
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
    dst: &RRC<ir::Port>,
    assignments: &[(RRC<ir::Port>, GuardRef)],
    pool: &ir::GuardPool,
    flat: bool,
) -> Option<v::Sequential> {
    if assignments.len() < 2 {
        return None;
    }
    // Construct concat with all guards.
    let mut concat = v::ExprConcat::default();
    assignments.iter().for_each(|(_, gr)| {
        let expr = if flat {
            v::Expr::new_ref(VerilogGuardRef(*gr).to_string())
        } else {
            let guard = pool.get(*gr);
            guard_to_expr(guard, pool)
        };
        concat.add_expr(expr);
    });

    let onehot0 = v::Expr::new_call("$onehot0", vec![v::Expr::Concat(concat)]);
    let not_onehot0 = v::Expr::new_not(onehot0);
    let mut check = v::SequentialIfElse::new(not_onehot0);

    // Generated error message
    let ir::Canonical { cell, port } = dst.borrow().canonical();
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
    assert_eq!(ir::Direction::Input, pr.borrow().direction);
    let port = pr.borrow();
    if !port.attributes.has(ir::BoolAttr::Data) {
        return false;
    }
    if let ir::PortParent::Cell(cwr) = &port.parent {
        let cr = cwr.upgrade();
        let cell = cr.borrow();
        // For cell.is_this() ports that were externalized, we already checked
        // that the parent cell had the `@data` attribute.
        if cell.attributes.has(ir::BoolAttr::Data) || cell.is_this() {
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
    dst: &RRC<ir::Port>,
    assignments: &[(RRC<ir::Port>, GuardRef)],
    pool: &ir::GuardPool,
) -> v::Parallel {
    // Mux over the assignment with the given default value.
    let fold_assigns = |init: v::Expr| -> v::Expr {
        assignments.iter().rfold(init, |acc, (src, gr)| {
            let guard = pool.get(*gr);
            let asgn = port_to_ref(src);
            v::Expr::new_mux(guard_to_expr(guard, pool), asgn, acc)
        })
    };

    // If this is a data port
    let rhs: v::Expr = if is_data_port(dst) {
        if assignments.len() == 1 {
            // If there is exactly one guard, generate a continuous assignment.
            // This encodes the rewrite:
            // in = g ? out : 'x => in = out;
            // This is valid because 'x can be replaced with any value
            let (dst, _) = &assignments[0];
            port_to_ref(dst)
        } else {
            // Produce an assignment with 'x as the default case.
            fold_assigns(v::Expr::X)
        }
    } else {
        let init =
            v::Expr::new_ulit_dec(dst.borrow().width as u32, &0.to_string());

        // Flatten the mux expression if there is exactly one assignment with a true guard.
        if assignments.len() == 1 {
            let (src, gr) = &assignments[0];
            if gr.is_true() {
                port_to_ref(src)
            } else if src.borrow().is_constant(1, 1) {
                let guard = pool.get(*gr);
                guard_to_expr(guard, pool)
            } else {
                let guard = pool.get(*gr);
                v::Expr::new_mux(
                    guard_to_expr(guard, pool),
                    port_to_ref(src),
                    init,
                )
            }
        } else {
            fold_assigns(init)
        }
    };
    v::Parallel::ParAssign(port_to_ref(dst), rhs)
}

fn emit_assignment_flat<F: io::Write>(
    dst: &RRC<ir::Port>,
    assignments: &[(RRC<ir::Port>, GuardRef)],
    f: &mut F,
) -> io::Result<()> {
    let data = is_data_port(dst);

    // Simple optimizations for 1-guard cases.
    if assignments.len() == 1 {
        let (src, guard) = &assignments[0];
        if data {
            // For data ports (for whom unassigned values are undefined), we can drop the guard
            // entirely and assume it is always true (because it would be UB if it were ever false).
            return writeln!(
                f,
                "assign {} = {};",
                VerilogPortRef(dst),
                VerilogPortRef(src)
            );
        } else {
            // For non-data ("control") ports, we have special cases for true guards and constant-1 RHSes.
            if guard.is_true() {
                return writeln!(
                    f,
                    "assign {} = {};",
                    VerilogPortRef(dst),
                    VerilogPortRef(src)
                );
            } else if src.borrow().is_constant(1, 1) {
                return writeln!(
                    f,
                    "assign {} = {};",
                    VerilogPortRef(dst),
                    VerilogGuardRef(*guard)
                );
            }
        }
    }

    // Use a cascade of ternary expressions to assign the right RHS to dst.
    writeln!(f, "assign {} =", VerilogPortRef(dst))?;
    for (src, guard) in assignments {
        writeln!(
            f,
            "  {} ? {} :",
            VerilogGuardRef(*guard),
            VerilogPortRef(src)
        )?;
    }

    // The default value depends on whether we are assigning to a data or control port.
    if data {
        writeln!(f, "  'x;")
    } else {
        writeln!(f, "  {}'d0;", dst.borrow().width)
    }
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
                ir::CellType::ThisComponent => v::Expr::new_ref(port.name),
                _ => v::Expr::Ref(format!(
                    "{}_{}",
                    parent.name().as_ref(),
                    port.name.as_ref()
                )),
            }
        }
        ir::PortParent::Group(_) => unreachable!(),
        ir::PortParent::StaticGroup(_) => unreachable!(),
    }
}

fn guard_to_expr(guard: &ir::FlatGuard, pool: &ir::GuardPool) -> v::Expr {
    let op = |g: &ir::FlatGuard| match g {
        FlatGuard::Or(..) => v::Expr::new_bit_or,
        FlatGuard::And(..) => v::Expr::new_bit_and,
        FlatGuard::CompOp(op, ..) => match op {
            ir::PortComp::Eq => v::Expr::new_eq,
            ir::PortComp::Neq => v::Expr::new_neq,
            ir::PortComp::Gt => v::Expr::new_gt,
            ir::PortComp::Lt => v::Expr::new_lt,
            ir::PortComp::Geq => v::Expr::new_geq,
            ir::PortComp::Leq => v::Expr::new_leq,
        },
        FlatGuard::Not(..) | FlatGuard::Port(..) | FlatGuard::True => {
            unreachable!()
        }
    };

    match guard {
        FlatGuard::And(l, r) | FlatGuard::Or(l, r) => {
            let lg = pool.get(*l);
            let rg = pool.get(*r);
            op(guard)(guard_to_expr(lg, pool), guard_to_expr(rg, pool))
        }
        FlatGuard::CompOp(_, l, r) => op(guard)(port_to_ref(l), port_to_ref(r)),
        FlatGuard::Not(r) => {
            let g = pool.get(*r);
            v::Expr::new_not(guard_to_expr(g, pool))
        }
        FlatGuard::Port(p) => port_to_ref(p),
        FlatGuard::True => v::Expr::new_ulit_bin(1, &1.to_string()),
    }
}

/// A little newtype wrapper for GuardRefs that makes it easy to format them as Verilog variables.
struct VerilogGuardRef(GuardRef);

impl std::fmt::Display for VerilogGuardRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "_guard{}", self.0.index())
    }
}

/// Similarly, a little wrapper for PortRefs that makes it easy to format them as Verilog variables.
struct VerilogPortRef<'a>(&'a RRC<ir::Port>);

impl<'a> std::fmt::Display for VerilogPortRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let port = self.0.borrow();
        match &port.parent {
            ir::PortParent::Cell(cell) => {
                let parent_ref = cell.upgrade();
                let parent = parent_ref.borrow();
                match parent.prototype {
                    ir::CellType::Constant { val, width } => {
                        write!(f, "{width}'d{val}")
                    }
                    ir::CellType::ThisComponent => {
                        write!(f, "{}", port.name)
                    }
                    _ => {
                        write!(
                            f,
                            "{}_{}",
                            parent.name().as_ref(),
                            port.name.as_ref()
                        )
                    }
                }
            }
            ir::PortParent::Group(_) => unreachable!(),
            ir::PortParent::StaticGroup(_) => unreachable!(),
        }
    }
}

fn emit_guard<F: std::io::Write>(
    guard: &ir::FlatGuard,
    f: &mut F,
) -> io::Result<()> {
    let gr = VerilogGuardRef;
    match guard {
        FlatGuard::Or(l, r) => write!(f, "{} | {}", gr(*l), gr(*r)),
        FlatGuard::And(l, r) => write!(f, "{} & {}", gr(*l), gr(*r)),
        FlatGuard::CompOp(op, l, r) => {
            let op = match op {
                ir::PortComp::Eq => "==",
                ir::PortComp::Neq => "!=",
                ir::PortComp::Gt => ">",
                ir::PortComp::Lt => "<",
                ir::PortComp::Geq => ">=",
                ir::PortComp::Leq => "<=",
            };
            write!(f, "{} {} {}", VerilogPortRef(l), op, VerilogPortRef(r))
        }
        FlatGuard::Not(g) => write!(f, "~{}", gr(*g)),
        FlatGuard::True => write!(f, "1"),
        FlatGuard::Port(p) => write!(f, "{}", VerilogPortRef(p)),
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
            let is_external = cell.borrow().get_attribute(ir::BoolAttr::External).is_some();
            if is_external
                && cell
                    .borrow()
                    .type_name()
                    // HACK: Check if the name of the primitive contains the string "mem"
                    .map(|proto| proto.id.as_str().contains("mem"))
                    .unwrap_or_default()
            {
                Some((
                    cell.borrow().name().id,
                    cell.borrow().type_name().unwrap_or_else(|| unreachable!("tried to add a memory cell but there was no type name")),
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
        let mem_access_str = get_mem_str(mem_type.id.as_str());
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
        let mem_access_str = get_mem_str(mem_type.id.as_str());

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
