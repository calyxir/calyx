use crate::frontend::library::ast as lib;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use std::collections::HashMap;

/// Replaces "external" memory cells with internal memories.
pub struct RemoveExternalMemories<'a> {
    changeable: HashMap<&'a str, &'a str>,
}

impl Default for RemoveExternalMemories<'_> {
    fn default() -> Self {
        let changeable: HashMap<&'static str, &'static str> = vec![
            ("std_mem_d1_ext", "std_mem_d1"),
            ("std_mem_d2_ext", "std_mem_d2"),
            ("std_mem_d3_ext", "std_mem_d3"),
        ]
        .into_iter()
        .collect();
        Self { changeable }
    }
}

impl Named for RemoveExternalMemories<'_> {
    fn name() -> &'static str {
        "remove-external-memories"
    }

    fn description() -> &'static str {
        "Replace external memory primitives with internal memory primitives"
    }
}

impl Visitor<()> for RemoveExternalMemories<'_> {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _c: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        for cell_ref in &comp.cells {
            let mut cell = cell_ref.borrow_mut();
            if let ir::CellType::Primitive { name, .. } = &mut cell.prototype {
                if let Some(&new_name) = self.changeable.get(name.id.as_str()) {
                    // Simply change the name of the primitive.
                    *name = new_name.into()
                }
            }
        }

        Ok(Action::stop_default())
    }
}
