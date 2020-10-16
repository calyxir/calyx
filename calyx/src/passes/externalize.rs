//! Pass to externalize input/output ports for "external" cells.
//! The input ports of these cells are exposed through the ports of the
//! component containing them.
//! For example:
//! ```
//! component main() -> () {
//!     cells {
//!         // Inputs: addr0, write_data, write_en
//!         // Outputs: read_data, done
//!         m1 = prim std_mem_d1_ext(32, 10, 4);
//!     }
//! }
//! ```
//! is transformed into:
//! ```
//! component main(m1_add0, m1_write_data, m1_write_en) -> (m1_read_data, m1_done) {
//!     cells {
//!         mem1 = prim std_mem_d1_ext(32, 10, 4);
//!     }
//! }
//! ```
use crate::frontend::library::ast as lib;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};

#[derive(Default)]
pub struct Externalize;

impl Named for Externalize {
    fn name() -> &'static str {
        "externalize"
    }

    fn description() -> &'static str {
        "Externalize the interfaces of _ext memories"
    }
}

impl Externalize {
    /// Is this primitive and external
    pub fn is_external_cell(name: &str) -> bool {
        name.starts_with("std_mem") && name.ends_with("ext")
    }

    /// Generate a string given the name of the component and the port.
    pub fn port_name(comp: &str, port: &str) -> ir::Id {
        format!("{}_{}", comp, port).into()
    }
}

impl Visitor for Externalize {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _c: &lib::LibrarySignatures,
    ) -> VisResult {
        //let st = &mut comp.structure;

        /*for cell_ref in comp.cells {
            let cell = cell_ref.borrow_mut();
            if let ir::CellType::Primitive { name, .. } = cell.prototype {
                // If this cell is an "external" cell, expose the ports
                // on the component signatures.
                if name.starts_with("std_mem") && name.ends_with("ext") {}
            }
        }*/

        /*for (idx, node) in indicies {
            for portdef in &node.signature.inputs {
                let portname =
                    format!("{}_{}", node.name.as_ref(), portdef.name.as_ref());
                let new_portdef = Portdef {
                    name: portname.into(),
                    width: portdef.width,
                };
                st.insert_output_port(&new_portdef);
                for edidx in st
                    .edge_idx()
                    .with_node(idx)
                    .with_port(portdef.name.to_string())
                    .with_direction(DataDirection::Write)
                    .detach()
                {
                    let edge = st.get_edge(edidx).clone();
                    let src_port = edge.src.port_name().clone();
                    let (src_node, _) = st.endpoints(edidx);

                    st.insert_edge(
                        (src_node, src_port),
                        (st.get_this_idx(), new_portdef.name.clone()),
                        edge.group,
                        edge.guard,
                    )?;

                    st.remove_edge(edidx);
                }
            }

            for portdef in &node.signature.outputs {
                let portname =
                    format!("{}_{}", node.name.as_ref(), portdef.name.as_ref());
                let new_portdef = Portdef {
                    name: portname.into(),
                    width: portdef.width,
                };
                st.insert_input_port(&new_portdef);
                for edidx in st
                    .edge_idx()
                    .with_node(idx)
                    .with_port(portdef.name.to_string())
                    .with_direction(DataDirection::Read)
                    .detach()
                {
                    let edge = st.get_edge(edidx).clone();
                    let dest_port = edge.dest.port_name().clone();
                    let (_, dest_node) = st.endpoints(edidx);

                    st.insert_edge(
                        (st.get_this_idx(), new_portdef.name.clone()),
                        (dest_node, dest_port),
                        edge.group,
                        edge.guard,
                    )?;

                    st.remove_edge(edidx);
                }
            }

            st.remove_node(idx);
        }*/

        // Stop traversal, we don't need to traverse over control ast
        Ok(Action::Stop)
    }
}
