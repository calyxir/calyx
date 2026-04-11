use std::collections::HashMap;

use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{
    self as ir, Assignment, Cell, GetAttributes, Id, LibrarySignatures, Nothing,
};
use calyx_utils::{CalyxResult, Error};

struct MemTransformInfo {
    pub dim_sizes: Vec<u64>, // d1, d2, ...
    pub width: u64,
    pub is_extern: bool,
    pub wrapper_name: String,
    pub mem_name: Id,
}

pub struct MemFlat {
    mems_to_proc: HashMap<Id, Vec<MemTransformInfo>>,
}

impl Named for MemFlat {
    fn name() -> &'static str {
        "mem-flat"
    }
    fn description() -> &'static str {
        "If the appropriate adapters are in the workspace, change all higher-dimensioned memories to 1D memories with address adapters."
    }
}

// TODO: could be reworked to use the utils stuff
// TODO: A_int may have some shadow 'uses'?
impl ConstructVisitor for MemFlat {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        let dimension_params = ["D0_SIZE", "D1_SIZE", "D2_SIZE", "D3_SIZE"];
        let mut to_constr = Self {
            mems_to_proc: HashMap::new(),
        };
        for comp in ctx.components.iter() {
            let mut e = None;
            let comp_mems: Vec<MemTransformInfo> = comp
                .cells
                .iter()
                .filter_map(|c| {
                    let subcomp = c.borrow();
                    let prot = &subcomp.prototype;
                    let n = prot.get_name()?;
                    if matches!(
                        n.as_ref(),
                        "seq_mem_d2" | "seq_mem_d3" | "seq_mem_d4"
                    ) {
                        let dim_sizes: Vec<u64> = dimension_params
                            .iter()
                            .filter_map(|param| subcomp.get_parameter(*param))
                            .collect();
                        let spec = dim_sizes
                            .iter()
                            .map(|i| i.to_string())
                            .collect::<Vec<String>>()
                            .join("x");
                        let sig = format!("fd{}_{spec}", dim_sizes.len());
                        let matching_wrapper =
                            ctx.components.iter().find(|c| c.name == sig);
                        let is_extern = subcomp
                            .get_attribute(ir::BoolAttr::External)
                            .is_some_and(|x| x == 1);
                        if matching_wrapper.is_some() {
                            return Some(MemTransformInfo {
                                dim_sizes,
                                width: subcomp.get_parameter("WIDTH").unwrap(),
                                is_extern,
                                wrapper_name: sig,
                                mem_name: subcomp.name(),
                            });
                        }
                        e = Some(Error::misc(format!(
                            "no wrapper for {n}, expected {sig}"
                        )));
                    }
                    None
                })
                .collect();
            if let Some(err) = e {
                return Err(err);
            }
            to_constr.mems_to_proc.insert(comp.name, comp_mems);
        }
        Ok(to_constr)
    }
    fn clear_data(&mut self) {}
}

impl Visitor for MemFlat {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        comps: &[ir::Component],
    ) -> VisResult {
        let Some(mems_to_process) = self.mems_to_proc.get(&comp.name) else {
            return Ok(Action::Continue);
        };

        let mut builder = ir::Builder::new(comp, sigs);
        for mem in mems_to_process.iter() {
            let new_size: u64 = mem.dim_sizes.iter().product();
            let address_width: u64 = (new_size as f64).log2().ceil() as u64;
            let new_mem_ref = builder.add_primitive(
                mem.mem_name,
                "seq_mem_d1",
                &[mem.width, new_size, address_width],
            );

            if mem.is_extern {
                new_mem_ref
                    .borrow_mut()
                    .add_attribute(ir::BoolAttr::External, 1);
                // unset external so it can be removed in dead cell removal
                let orig_instance =
                    builder.component.find_cell(mem.mem_name).unwrap();
                orig_instance
                    .borrow_mut()
                    .get_mut_attributes()
                    .remove(ir::BoolAttr::External);
            }

            let wrapper =
                comps.iter().find(|e| e.name == mem.wrapper_name).unwrap();
            let mut new_sig = wrapper.signature.borrow().get_signature();
            new_sig
                .iter_mut()
                .for_each(|pd| pd.direction = pd.direction.reverse());

            let wrapper_inst_ref = builder.add_component(
                format!("wrap_{}", mem.mem_name),
                wrapper.name.to_string(),
                new_sig,
            );
            let wrapper_inst = wrapper_inst_ref.borrow();

            fn subs_mem<T>(
                a: &mut Assignment<T>,
                name: Id,
                wrapper: &Cell,
                new_mem: &Cell,
            ) {
                // dest assigns
                if a.dst.borrow().get_parent_name() == name {
                    let portname = a.dst.borrow().name;
                    // address line fixup
                    if portname.to_string().contains("addr") {
                        let new_port = wrapper.get(portname).borrow().clone();
                        a.dst.replace(new_port);
                    } else {
                        // remaining inputs
                        let new_port = new_mem.get(portname).borrow().clone();
                        a.dst.replace(new_port);
                    }
                } else if a.src.borrow().get_parent_name() == name {
                    let portname = a.src.borrow().name;
                    let new_port = new_mem.get(portname).borrow().clone();
                    a.src.replace(new_port);
                }
            }

            let new_mem = new_mem_ref.borrow();

            // address line updates
            builder.component.for_each_assignment(|a| {
                subs_mem(a, mem.mem_name, &wrapper_inst, &new_mem)
            });
            builder.component.for_each_static_assignment(|a| {
                subs_mem(a, mem.mem_name, &wrapper_inst, &new_mem)
            });

            // tie wrapper address to new mem
            let a: Assignment<Nothing> = builder.build_assignment(
                new_mem.get("addr0"),
                wrapper_inst.get("addr_o"),
                ir::Guard::True,
            );
            builder.add_continuous_assignments(vec![a]);
        }
        Ok(Action::Continue)
    }
}
