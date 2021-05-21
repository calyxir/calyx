use super::{
    Assignment, Attributes, Builder, CellType, Component, Context, Control,
    Direction, GetAttributes, Guard, Id, LibrarySignatures, Port, PortDef,
    Width, RRC,
};
use crate::{
    errors::{Error, FutilResult},
    frontend::ast,
    utils::NameGenerator,
};
use linked_hash_map::LinkedHashMap;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;
use std::rc::Rc;

/// Context to store the signature information for all defined primitives and
/// components.
#[derive(Default)]
struct SigCtx {
    /// Mapping from component names to signatures
    comp_sigs: HashMap<Id, Vec<PortDef>>,

    /// Mapping from library functions to signatures
    lib: LibrarySignatures,
}

/// Validates a component signature to make sure there are not duplicate ports.
fn check_signature(sig: &[PortDef]) -> FutilResult<()> {
    let mut inputs: HashSet<&Id> = HashSet::new();
    let mut outputs: HashSet<&Id> = HashSet::new();
    for pd in sig {
        // check for uniqueness
        match pd.direction {
            Direction::Input => {
                if !inputs.contains(&pd.name) {
                    inputs.insert(&pd.name);
                } else {
                    return Err(Error::AlreadyBound(
                        pd.name.clone(),
                        "component".to_string(),
                    ));
                }
            }
            Direction::Output => {
                if !outputs.contains(&pd.name) {
                    outputs.insert(&pd.name);
                } else {
                    return Err(Error::AlreadyBound(
                        pd.name.clone(),
                        "component".to_string(),
                    ));
                }
            }
            Direction::Inout => {
                panic!("Components shouldn't have inout ports.")
            }
        }
    }
    Ok(())
}

/// Definition of special interface ports.
const INTERFACE_PORTS: [(&str, u64, Direction); 4] = [
    ("go", 1, Direction::Input),
    ("clk", 1, Direction::Input),
    ("reset", 1, Direction::Input),
    ("done", 1, Direction::Output),
];

/// Extend the signature with magical ports.
fn extend_signature(sig: &mut Vec<PortDef>) {
    let port_names: HashSet<_> =
        sig.iter().map(|pd| pd.name.to_string()).collect();
    let mut namegen = NameGenerator::with_prev_defined_names(port_names);
    for (name, width, direction) in INTERFACE_PORTS.iter() {
        if sig.iter().find(|pd| pd.attributes.has(name)).is_none() {
            let mut attributes = Attributes::default();
            attributes.insert(name, 1);
            sig.push(PortDef {
                name: namegen.gen_name(name.to_string()),
                width: Width::Const { value: *width },
                direction: direction.clone(),
                attributes,
            });
        }
    }
}

/// Construct an IR representation using a parsed AST and command line options.
pub fn ast_to_ir(
    mut namespace: ast::NamespaceDef,
    debug_mode: bool,
    synthesis_mode: bool,
) -> FutilResult<Context> {
    let mut all_names: HashSet<&Id> = HashSet::with_capacity(
        namespace.components.len() + namespace.externs.len(),
    );

    let prim_names = namespace
        .externs
        .iter()
        .flat_map(|(_, prims)| prims.iter().map(|prim| &prim.name));
    let comp_names = namespace.components.iter().map(|comp| &comp.name);

    for bound in prim_names.chain(comp_names) {
        if all_names.contains(bound) {
            return Err(Error::AlreadyBound(
                bound.clone(),
                "component or primitive".to_string(),
            ));
        }
        all_names.insert(bound);
    }

    // Build the signature context
    let mut sig_ctx = SigCtx {
        lib: namespace.externs.into(),
        ..Default::default()
    };

    // Add component signatures to context
    for comp in &mut namespace.components {
        check_signature(&comp.signature)?;
        // extend the signature
        extend_signature(&mut comp.signature);
        sig_ctx
            .comp_sigs
            .insert(comp.name.clone(), comp.signature.clone());
    }

    let comps = namespace
        .components
        .into_iter()
        .map(|comp| build_component(comp, &sig_ctx))
        .collect::<Result<_, _>>()?;

    Ok(Context {
        components: comps,
        lib: sig_ctx.lib,
        imports: namespace.imports,
        debug_mode,
        synthesis_mode,
    })
}

fn validate_component(
    comp: &ast::ComponentDef,
    sig_ctx: &SigCtx,
) -> FutilResult<()> {
    let mut cells = HashSet::new();
    let mut groups = HashSet::new();

    for cell in &comp.cells {
        if cells.contains(&cell.name) {
            return Err(Error::AlreadyBound(
                cell.name.clone(),
                "cell".to_string(),
            ));
        }
        cells.insert(cell.name.clone());

        let proto_name = &cell.prototype.name;

        if sig_ctx.lib.find_primitive(&proto_name).is_none()
            && !sig_ctx.comp_sigs.contains_key(&proto_name)
        {
            return Err(Error::Undefined(
                proto_name.clone(),
                "primitive or component".to_string(),
            ));
        }
    }

    for group in &comp.groups {
        let name = &group.name;
        if groups.contains(name) {
            return Err(Error::AlreadyBound(name.clone(), "group".to_string()));
        }
        if cells.contains(name) {
            return Err(Error::AlreadyBound(name.clone(), "cell".to_string()));
        }
        groups.insert(name.clone());
    }

    Ok(())
}

/// Build an `ir::component::Component` using an `frontend::ast::ComponentDef`.
fn build_component(
    comp: ast::ComponentDef,
    sig_ctx: &SigCtx,
) -> FutilResult<Component> {
    // Validate the component before building it.
    validate_component(&comp, sig_ctx)?;

    // Components don't have any parameter information.
    let fake_binding = LinkedHashMap::with_capacity(0);
    let mut ir_component = Component::new(
        comp.name,
        comp.signature
            .into_iter()
            .map(|pd| {
                pd.resolve(&fake_binding)
                    .map(|(n, w, attrs)| (n, w, pd.direction, attrs))
            })
            .collect::<Result<_, _>>()?,
    );
    let mut builder = Builder::new(&mut ir_component, &sig_ctx.lib);

    // For each ast::Cell, add a Cell that contains all the
    // required information.
    comp.cells
        .into_iter()
        .for_each(|cell| add_cell(cell, &sig_ctx, &mut builder));

    comp.groups
        .into_iter()
        .try_for_each(|g| add_group(g, &mut builder))?;

    let continuous_assignments = comp
        .continuous_assignments
        .into_iter()
        .map(|w| build_assignment(w, &mut builder))
        .collect::<FutilResult<Vec<_>>>()?;
    builder.component.continuous_assignments = continuous_assignments;

    // Build the Control ast using ast::Control.
    let control =
        Rc::new(RefCell::new(build_control(comp.control, &mut builder)?));
    builder.component.control = control;

    ir_component.attributes = comp.attributes;

    Ok(ir_component)
}

///////////////// Cell Construction /////////////////////////

fn add_cell(cell: ast::Cell, sig_ctx: &SigCtx, builder: &mut Builder) {
    let proto_name = &cell.prototype.name;

    let res = if sig_ctx.lib.find_primitive(proto_name).is_some() {
        builder.add_primitive(cell.name, proto_name, &cell.prototype.params)
    } else {
        // Validator ensures that if the protoype is not a primitive, it
        // is a component.
        let name = builder.component.generate_name(cell.name);
        let sig = &sig_ctx.comp_sigs[proto_name];
        let typ = CellType::Component {
            name: proto_name.clone(),
        };
        // Components do not have any bindings for parameters
        let fake_binding = LinkedHashMap::with_capacity(0);
        let cell = Builder::cell_from_signature(
            name,
            typ,
            sig.iter()
                .cloned()
                .map(|pd| {
                    pd.resolve(&fake_binding)
                        .map(|(n, w, attrs)| (n, w, pd.direction, attrs))
                })
                .collect::<Result<Vec<_>, _>>()
                .expect("Failed to build component"),
        );
        builder.component.cells.add(Rc::clone(&cell));
        cell
    };

    // Add attributes to the built cell
    res.borrow_mut().attributes = cell.attributes;
}

///////////////// Group Construction /////////////////////////

/// Build an IR group using the AST Group.
fn add_group(group: ast::Group, builder: &mut Builder) -> FutilResult<()> {
    let ir_group = builder.add_group(group.name);
    ir_group.borrow_mut().attributes = group.attributes;

    // Add assignemnts to the group
    for wire in group.wires {
        let assign = build_assignment(wire, builder)?;
        ir_group.borrow_mut().assignments.push(assign)
    }

    Ok(())
}

///////////////// Assignment Construction /////////////////////////

/// Get the pointer to the Port represented by `port`.
fn get_port_ref(port: ast::Port, comp: &Component) -> FutilResult<RRC<Port>> {
    match port {
        ast::Port::Comp { component, port } => comp
            .find_cell(&component)
            .ok_or_else(|| {
                Error::Undefined(component.clone(), "cell".to_string())
            })?
            .borrow()
            .find(&port)
            .ok_or_else(|| Error::Undefined(port, "port".to_string())),
        ast::Port::This { port } => {
            comp.signature.borrow().find(&port).ok_or_else(|| {
                Error::Undefined(port, "component port".to_string())
            })
        }
        ast::Port::Hole { group, name: port } => comp
            .find_group(&group)
            .ok_or_else(|| Error::Undefined(group, "group".to_string()))?
            .borrow()
            .find(&port)
            .ok_or_else(|| Error::Undefined(port, "hole".to_string())),
    }
}

/// Get an port using an ast::Atom.
/// If the atom is a number and the context doesn't already contain a cell
/// for this constant, instantiate the constant node and get the "out" port
/// from it.
fn atom_to_port(
    atom: ast::Atom,
    builder: &mut Builder,
) -> FutilResult<RRC<Port>> {
    match atom {
        ast::Atom::Num(n) => {
            let port = builder.add_constant(n.val, n.width).borrow().get("out");
            Ok(Rc::clone(&port))
        }
        ast::Atom::Port(p) => get_port_ref(p, &builder.component),
    }
}

/// Build an ir::Assignment from ast::Wire.
/// The Assignment contains pointers to the relevant ports.
fn build_assignment(
    wire: ast::Wire,
    builder: &mut Builder,
) -> FutilResult<Assignment> {
    let src_port: RRC<Port> = atom_to_port(wire.src.expr, builder)?;
    let dst_port: RRC<Port> = get_port_ref(wire.dest, &builder.component)?;
    let guard = match wire.src.guard {
        Some(g) => build_guard(g, builder)?,
        None => Guard::True,
    };

    Ok(builder.build_assignment(dst_port, src_port, guard))
}

/// Transform an ast::GuardExpr to an ir::Guard.
fn build_guard(guard: ast::GuardExpr, bd: &mut Builder) -> FutilResult<Guard> {
    use ast::GuardExpr as GE;

    let into_box_guard = |g: Box<GE>, bd: &mut Builder| -> FutilResult<_> {
        Ok(Box::new(build_guard(*g, bd)?))
    };

    Ok(match guard {
        GE::Atom(atom) => Guard::port(atom_to_port(atom, bd)?),
        GE::Or(l, r) => Guard::or(build_guard(*l, bd)?, build_guard(*r, bd)?),
        GE::And(l, r) => Guard::and(build_guard(*l, bd)?, build_guard(*r, bd)?),
        GE::Not(g) => Guard::Not(into_box_guard(g, bd)?),
        GE::Eq(l, r) => Guard::Eq(atom_to_port(l, bd)?, atom_to_port(r, bd)?),
        GE::Neq(l, r) => Guard::Neq(atom_to_port(l, bd)?, atom_to_port(r, bd)?),
        GE::Gt(l, r) => Guard::Gt(atom_to_port(l, bd)?, atom_to_port(r, bd)?),
        GE::Lt(l, r) => Guard::Lt(atom_to_port(l, bd)?, atom_to_port(r, bd)?),
        GE::Geq(l, r) => Guard::Geq(atom_to_port(l, bd)?, atom_to_port(r, bd)?),
        GE::Leq(l, r) => Guard::Leq(atom_to_port(l, bd)?, atom_to_port(r, bd)?),
    })
}

///////////////// Control Construction /////////////////////////

/// Transform ast::Control to ir::Control.
fn build_control(
    control: ast::Control,
    builder: &mut Builder,
) -> FutilResult<Control> {
    Ok(match control {
        ast::Control::Enable {
            comp: component,
            attributes,
        } => {
            let mut en = Control::enable(Rc::clone(
                &builder.component.find_group(&component).ok_or_else(|| {
                    Error::Undefined(component.clone(), "group".to_string())
                })?,
            ));
            *(en.get_mut_attributes().unwrap()) = attributes;
            en
        }
        ast::Control::Invoke {
            comp: component,
            inputs,
            outputs,
            attributes,
        } => {
            let cell = Rc::clone(
                &builder.component.find_cell(&component).ok_or_else(|| {
                    Error::Undefined(component.clone(), "cell".to_string())
                })?,
            );
            let inps = inputs
                .into_iter()
                .map(|(id, port)| atom_to_port(port, builder).map(|p| (id, p)))
                .collect::<Result<_, _>>()?;
            let outs = outputs
                .into_iter()
                .map(|(id, port)| atom_to_port(port, builder).map(|p| (id, p)))
                .collect::<Result<_, _>>()?;
            let mut inv = Control::invoke(cell, inps, outs);
            *(inv.get_mut_attributes().unwrap()) = attributes;
            inv
        }
        ast::Control::Seq { stmts, attributes } => {
            let mut s = Control::seq(
                stmts
                    .into_iter()
                    .map(|c| build_control(c, builder))
                    .collect::<FutilResult<Vec<_>>>()?,
            );
            *(s.get_mut_attributes().unwrap()) = attributes;
            s
        }
        ast::Control::Par { stmts, attributes } => {
            let mut p = Control::par(
                stmts
                    .into_iter()
                    .map(|c| build_control(c, builder))
                    .collect::<FutilResult<Vec<_>>>()?,
            );
            *(p.get_mut_attributes().unwrap()) = attributes;
            p
        }
        ast::Control::If {
            port,
            cond,
            tbranch,
            fbranch,
            attributes,
        } => {
            let mut con = Control::if_(
                get_port_ref(port, builder.component)?,
                Rc::clone(&builder.component.find_group(&cond).ok_or_else(
                    || Error::Undefined(cond.clone(), "group".to_string()),
                )?),
                Box::new(build_control(*tbranch, builder)?),
                Box::new(build_control(*fbranch, builder)?),
            );
            *(con.get_mut_attributes().unwrap()) = attributes;
            con
        }
        ast::Control::While {
            port,
            cond,
            body,
            attributes,
        } => {
            let mut con = Control::while_(
                get_port_ref(port, builder.component)?,
                Rc::clone(&builder.component.find_group(&cond).ok_or_else(
                    || Error::Undefined(cond.clone(), "group".to_string()),
                )?),
                Box::new(build_control(*body, builder)?),
            );
            *(con.get_mut_attributes().unwrap()) = attributes;
            con
        }
        ast::Control::Empty { .. } => Control::empty(),
    })
}
