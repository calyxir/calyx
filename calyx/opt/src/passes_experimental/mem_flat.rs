use std::collections::HashMap;

use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{
    self as ir, Assignment, Attributes, Cell, CellType, GetAttributes, Id,
    LibrarySignatures, Nothing, NumAttr::WriteTogether, PortDef,
    utils::GetMemInfo,
};
use calyx_utils::CalyxResult;

trait FlatTransform {
    fn make_signature(&self, awidth: u64) -> Vec<PortDef<u64>>;
    fn wrapper_name(&self) -> String;
}

impl FlatTransform for calyx_ir::utils::MemInfo {
    fn make_signature(&self, awidth: u64) -> Vec<PortDef<u64>> {
        self.dimension_sizes
            .iter()
            .enumerate()
            .map(|(idx, w)| {
                let mut attr = Attributes::default();
                attr.insert(WriteTogether, 1);
                PortDef::new(
                    format!("addr{}", idx),
                    *w,
                    calyx_ir::Direction::Input,
                    attr,
                )
            })
            .chain(std::iter::once(PortDef::new(
                "addr_o",
                awidth,
                calyx_ir::Direction::Output,
                Attributes::default(),
            )))
            .collect()
    }
    fn wrapper_name(&self) -> String {
        let spec = self
            .dimension_sizes
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<String>>()
            .join("x");
        format!("fd{}_{spec}", self.dimensions)
    }
}

pub struct MemFlat {
    mems_to_proc: HashMap<Id, Vec<calyx_ir::utils::MemInfo>>,

    /// for each component, track which ref comps were renamed
    invoke_renames: HashMap<Id, Vec<Id>>,

    // memories which have only been renamed within current component
    local_renames: Vec<Id>,
}

impl Named for MemFlat {
    fn name() -> &'static str {
        "mem-flat"
    }
    fn description() -> &'static str {
        "If the appropriate adapters are in the workspace, change all higher-dimensioned memories to 1D memories with address adapters."
    }
}

// TODO: A_int may have some shadow 'uses'?
impl ConstructVisitor for MemFlat {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        let mut to_constr = Self {
            mems_to_proc: HashMap::new(),
            invoke_renames: HashMap::new(),
            local_renames: Vec::new(),
        };
        for comp in ctx.components.iter() {
            let comp_mem_cells =
                calyx_ir::utils::external_and_ref_memories_cells(comp);

            let comp_mem_info: Vec<calyx_ir::utils::MemInfo> = comp_mem_cells
                .get_mem_info()
                .into_iter()
                .filter(|m| m.dimensions > 1)
                .collect();

            to_constr.mems_to_proc.insert(comp.name, comp_mem_info);
        }
        Ok(to_constr)
    }
    fn clear_data(&mut self) {
        self.local_renames = Vec::new();
    }
}

// TODO: skip if nothing to do. shouldn't actually have any effect, just. wastes cycles
// TODO: creates excess wrappers on a component if it only uses its mems for invokes. the wrappers are within the invoked component anyway, so no need.

impl Visitor for MemFlat {
    fn iteration_order() -> crate::traversal::Order
    where
        Self: Sized,
    {
        crate::traversal::Order::Post
    }
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let Some(mems_to_process) = self.mems_to_proc.get(&comp.name) else {
            return Ok(Action::Continue);
        };

        let mut builder = ir::Builder::new(comp, sigs);
        let mut renames: Vec<Id> = Vec::new();
        for mem in mems_to_process.iter() {
            let new_size: u64 = mem.dimension_sizes.iter().product();
            let address_width = calyx_utils::bits_needed_for(new_size);
            let new_mem_ref = builder.add_primitive(
                format!("flat_{}", mem.name),
                "seq_mem_d1",
                &[mem.data_width, new_size, address_width],
            );

            let orig_instance =
                builder.component.find_cell(mem.name.as_str()).unwrap();

            if mem.is_extern {
                new_mem_ref
                    .borrow_mut()
                    .add_attribute(ir::BoolAttr::External, 1);
                // unset external so it can be removed in dead cell removal
                orig_instance
                    .borrow_mut()
                    .get_mut_attributes()
                    .remove(ir::BoolAttr::External);
            }
            if mem.is_ref {
                new_mem_ref.borrow_mut().set_reference(true);
                orig_instance.borrow_mut().set_reference(false);
                renames.push(Id::from(mem.name.as_str()));
            } else {
                self.local_renames.push(Id::from(mem.name.as_str()));
            }

            let new_sig = mem.make_signature(address_width);

            let wrapper_inst_ref = builder.add_component(
                format!("wrap_{}", mem.name),
                mem.wrapper_name(),
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
                subs_mem(
                    a,
                    Id::from(mem.name.as_str()),
                    &wrapper_inst,
                    &new_mem,
                )
            });
            builder.component.for_each_static_assignment(|a| {
                subs_mem(
                    a,
                    Id::from(mem.name.as_str()),
                    &wrapper_inst,
                    &new_mem,
                )
            });

            // tie wrapper address to new mem
            let a: Assignment<Nothing> = builder.build_assignment(
                new_mem.get("addr0"),
                wrapper_inst.get("addr_o"),
                ir::Guard::True,
            );
            builder.add_continuous_assignments(vec![a]);
        }
        log::info!("inserting renames for {}: {:?}", comp.name, renames);
        self.invoke_renames.insert(comp.name, renames);
        Ok(Action::Continue)
    }
    fn invoke(
        &mut self,
        s: &mut calyx_ir::Invoke,
        comp: &mut calyx_ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        // handle renames local to this component

        for (_, cr) in s.ref_cells.iter_mut() {
            let curr_name = &cr.borrow().name();
            if self.local_renames.contains(curr_name) {
                let corr_cell =
                    comp.find_cell(format!("flat_{}", curr_name)).unwrap();
                *cr = corr_cell.clone();
            }
        }

        // potentially fix rewrites in other components
        let CellType::Component { name: cname } = s.comp.borrow().prototype
        else {
            return Ok(Action::Continue);
        };
        let Some(rn) = self.invoke_renames.get(&cname) else {
            return Ok(Action::Continue);
        };

        for (n, _) in s.ref_cells.iter_mut() {
            if rn.contains(n) {
                *n = Id::from(format!("flat_{}", n));
            }
        }

        Ok(Action::Continue)
    }
}
