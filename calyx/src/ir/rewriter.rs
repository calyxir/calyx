use crate::ir::{self, RRC};
use std::collections::HashMap;
use std::rc::Rc;

use super::CloneName;

/// Map to rewrite cell uses. Maps name of the old cell to the new [ir::Cell]
/// instance.
type CellRewriteMap = HashMap<ir::Id, RRC<ir::Cell>>;

/// Map to rewrite port uses. Maps the canonical name of an old port (generated using
/// [ir::Port::canonical]) to the new [ir::Port] instance.
type PortRewriteMap = HashMap<(ir::Id, ir::Id), RRC<ir::Port>>;

/// Map name of old group to new group
type GroupRewriteMap = HashMap<ir::Id, RRC<ir::Group>>;
/// Map name of old combination group to new combinational group
type CombGroupRewriteMap = HashMap<ir::Id, RRC<ir::CombGroup>>;

/// A structure to track rewrite maps for ports. Stores both cell rewrites and direct port
/// rewrites. Attempts to apply port rewrites first before trying the cell
/// rewrite.
pub struct PortRewrite<'a> {
    cell_map: &'a CellRewriteMap,
    port_map: &'a PortRewriteMap,
}

impl<'a> PortRewrite<'a> {
    pub fn new(
        cell_map: &'a CellRewriteMap,
        port_map: &'a PortRewriteMap,
    ) -> Self {
        Self { cell_map, port_map }
    }

    /// Return the rewrite for a cell
    pub fn get_cell_rewrite(&self, cell: &ir::Id) -> Option<RRC<ir::Cell>> {
        self.cell_map.get(cell).map(Rc::clone)
    }

    /// Return a cell rewrite for the given port. A cell rewrite will attempt
    /// to give the port with the same name on the new cell.
    ///
    /// For example, given with `cell = a` and `new_cell = b`
    /// ```
    /// a.in = a.done ? a.out;
    /// ```
    /// is rewritten to
    /// ```
    /// b.in = b.done ? b.out;
    /// ```
    #[inline]
    fn get_cell_port_rewrite(
        &self,
        port_ref: &RRC<ir::Port>,
    ) -> Option<RRC<ir::Port>> {
        if self.cell_map.is_empty() {
            return None;
        }

        let port = port_ref.borrow();
        let new_cell = if let ir::PortParent::Cell(cell_wref) = &port.parent {
            let cell_ref = cell_wref.upgrade();
            let cell = cell_ref.borrow();
            self.cell_map.get(cell.name())
        } else {
            None
        };
        // Return port with the same name on the new_cell.
        new_cell.map(|new_cell| Rc::clone(&new_cell.borrow().get(&port.name)))
    }

    /// Return a port rewrite if present.
    #[inline]
    fn get_port_rewrite(
        &self,
        port_ref: &RRC<ir::Port>,
    ) -> Option<RRC<ir::Port>> {
        if self.port_map.is_empty() {
            return None;
        }

        let port = port_ref.borrow();
        self.port_map.get(&port.canonical()).map(Rc::clone)
    }

    /// Get any port rewrite defined for the given port.
    #[inline]
    pub fn get(&self, port_ref: &RRC<ir::Port>) -> Option<RRC<ir::Port>> {
        self.get_port_rewrite(port_ref)
            .or_else(|| self.get_cell_port_rewrite(port_ref))
    }

    /* /// Returns ownership of the underlying rewrite maps
    pub fn take(self) -> (CellRewriteMap, PortRewriteMap) {
        (self.cell_map, self.port_map)
    } */
}

/// IR Rewriter. Defines methods to rewrite various parts of the IR using
/// rewrite maps.
pub struct Rewriter;

impl Rewriter {
    /// Return a port rewrite if it is defeind in the set of rewrites.
    #[inline]
    pub fn get_port_rewrite(
        rewrites: &CellRewriteMap,
        port: &RRC<ir::Port>,
    ) -> Option<RRC<ir::Port>> {
        let rewrite =
            if let ir::PortParent::Cell(cell_wref) = &port.borrow().parent {
                let cell_ref = cell_wref.upgrade();
                let cell_name = cell_ref.borrow();
                rewrites.get(cell_name.name())
            } else {
                None
            };
        rewrite.map(|new_cell| {
            Rc::clone(&new_cell.borrow().get(&port.borrow().name))
        })
    }

    /// Rewrite a `invoke` node using a [CellRewriteMap] and a [CombGroupRewriteMap]
    pub fn rewrite_invoke(
        inv: &mut ir::Invoke,
        port_rewrite: &PortRewrite,
        comb_group_map: &CombGroupRewriteMap,
    ) {
        // Rewrite the name of the cell
        let name = inv.comp.borrow().clone_name();
        if let Some(new_cell) = &port_rewrite.get_cell_rewrite(&name) {
            inv.comp = Rc::clone(new_cell);
        }

        // Rewrite the combinational group
        if let Some(cg_ref) = &inv.comb_group {
            let cg = cg_ref.borrow().clone_name();
            if let Some(new_cg) = &comb_group_map.get(&cg) {
                inv.comb_group = Some(Rc::clone(new_cg));
            }
        }

        // Rewrite the parameters
        inv.inputs
            .iter_mut()
            .chain(inv.outputs.iter_mut())
            .for_each(|(_, port)| {
                if let Some(new_port) = port_rewrite.get(&*port) {
                    *port = new_port;
                }
            });
    }

    /// Given a control program, rewrite all uses of cells, groups, and comb groups using the given
    /// rewrite maps.
    pub fn rewrite_control(
        c: &mut ir::Control,
        port_rewrite: &PortRewrite,
        group_map: &GroupRewriteMap,
        comb_group_map: &CombGroupRewriteMap,
    ) {
        match c {
            ir::Control::Empty(_) => (),
            ir::Control::Enable(en) => {
                let g = &en.group.borrow().clone_name();
                if let Some(new_group) = group_map.get(g) {
                    en.group = Rc::clone(new_group);
                }
            }
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                stmts.iter_mut().for_each(|c| {
                    Self::rewrite_control(
                        c,
                        port_rewrite,
                        group_map,
                        comb_group_map,
                    )
                })
            }
            ir::Control::If(ife) => {
                // Rewrite port use
                if let Some(new_port) = port_rewrite.get(&ife.port) {
                    ife.port = new_port;
                }
                // Rewrite conditional comb group if defined
                if let Some(cg_ref) = &ife.cond {
                    let cg = cg_ref.borrow().clone_name();
                    if let Some(new_cg) = &comb_group_map.get(&cg) {
                        ife.cond = Some(Rc::clone(new_cg));
                    }
                }
                // rewrite branches
                Self::rewrite_control(
                    &mut ife.tbranch,
                    port_rewrite,
                    group_map,
                    comb_group_map,
                );
                Self::rewrite_control(
                    &mut ife.fbranch,
                    port_rewrite,
                    group_map,
                    comb_group_map,
                );
            }
            ir::Control::While(wh) => {
                // Rewrite port use
                if let Some(new_port) = port_rewrite.get(&wh.port) {
                    wh.port = new_port;
                }
                // Rewrite conditional comb group if defined
                if let Some(cg_ref) = &wh.cond {
                    let cg = cg_ref.borrow().clone_name();
                    if let Some(new_cg) = &comb_group_map.get(&cg) {
                        wh.cond = Some(Rc::clone(new_cg));
                    }
                }
                // rewrite body
                Self::rewrite_control(
                    &mut wh.body,
                    port_rewrite,
                    group_map,
                    comb_group_map,
                );
            }
            ir::Control::Invoke(inv) => {
                Self::rewrite_invoke(inv, port_rewrite, comb_group_map)
            }
        }
    }
}
