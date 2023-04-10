use crate::{self as ir, RRC};
use std::collections::HashMap;
use std::rc::Rc;

/// A rewrite map from [ir::Id] to [T].
pub type RewriteMap<T> = HashMap<ir::Id, RRC<T>>;

/// Map to rewrite port uses. Maps the canonical name of an old port (generated using
/// [ir::Port::canonical]) to the new [ir::Port] instance.
pub type PortRewriteMap = HashMap<ir::Canonical, RRC<ir::Port>>;

/// A structure to track rewrite maps for ports. Stores both cell rewrites and direct port
/// rewrites. Attempts to apply port rewrites first before trying the cell
/// rewrite.
pub struct Rewriter<'a> {
    cell_map: &'a RewriteMap<ir::Cell>,
    port_map: &'a PortRewriteMap,
}

impl<'a> Rewriter<'a> {
    pub fn new(
        cell_map: &'a RewriteMap<ir::Cell>,
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
            self.cell_map.get(&cell.name())
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

    // =========== Control Rewriting Methods =============
    /// Rewrite a `invoke` node using a [RewriteMap<ir::Cell>] and a [RewriteMap<ir::CombGroup>]
    pub fn rewrite_invoke(
        &self,
        inv: &mut ir::Invoke,
        comb_group_map: &RewriteMap<ir::CombGroup>,
    ) {
        // Rewrite the name of the cell
        let name = inv.comp.borrow().name();
        if let Some(new_cell) = &self.get_cell_rewrite(&name) {
            inv.comp = Rc::clone(new_cell);
        }

        // Rewrite the combinational group
        if let Some(cg_ref) = &inv.comb_group {
            let cg = cg_ref.borrow().name();
            if let Some(new_cg) = &comb_group_map.get(&cg) {
                inv.comb_group = Some(Rc::clone(new_cg));
            }
        }

        // Rewrite the parameters
        inv.inputs
            .iter_mut()
            .chain(inv.outputs.iter_mut())
            .for_each(|(_, port)| {
                if let Some(new_port) = self.get(&*port) {
                    *port = new_port;
                }
            });
    }

    /// Given a control program, rewrite all uses of cells, groups, and comb groups using the given
    /// rewrite maps.
    pub fn rewrite_static_control(
        &self,
        sc: &mut ir::StaticControl,
        group_map: &RewriteMap<ir::Group>,
        comb_group_map: &RewriteMap<ir::CombGroup>,
        static_group_map: &RewriteMap<ir::StaticGroup>,
    ) {
        match sc {
            ir::StaticControl::Enable(sen) => {
                let g = &sen.group.borrow().name();
                if let Some(new_group) = static_group_map.get(g) {
                    sen.group = Rc::clone(new_group);
                }
            }
            ir::StaticControl::Repeat(rep) => self.rewrite_static_control(
                &mut rep.body,
                group_map,
                comb_group_map,
                static_group_map,
            ),
        }
    }

    /// Given a control program, rewrite all uses of cells, groups, and comb groups using the given
    /// rewrite maps.
    pub fn rewrite_control(
        &self,
        c: &mut ir::Control,
        group_map: &RewriteMap<ir::Group>,
        comb_group_map: &RewriteMap<ir::CombGroup>,
        static_group_map: &RewriteMap<ir::StaticGroup>,
    ) {
        match c {
            ir::Control::Empty(_) => (),
            ir::Control::Enable(en) => {
                let g = &en.group.borrow().name();
                if let Some(new_group) = group_map.get(g) {
                    en.group = Rc::clone(new_group);
                }
            }
            ir::Control::StaticEnable(en) => {
                let g = &en.group.borrow().name();
                if let Some(new_group) = static_group_map.get(g) {
                    en.group = Rc::clone(new_group);
                }
            }
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                stmts.iter_mut().for_each(|c| {
                    self.rewrite_control(
                        c,
                        group_map,
                        comb_group_map,
                        static_group_map,
                    )
                })
            }
            ir::Control::If(ife) => {
                // Rewrite port use
                if let Some(new_port) = self.get(&ife.port) {
                    ife.port = new_port;
                }
                // Rewrite conditional comb group if defined
                if let Some(cg_ref) = &ife.cond {
                    let cg = cg_ref.borrow().name();
                    if let Some(new_cg) = &comb_group_map.get(&cg) {
                        ife.cond = Some(Rc::clone(new_cg));
                    }
                }
                // rewrite branches
                self.rewrite_control(
                    &mut ife.tbranch,
                    group_map,
                    comb_group_map,
                    static_group_map,
                );
                self.rewrite_control(
                    &mut ife.fbranch,
                    group_map,
                    comb_group_map,
                    static_group_map,
                );
            }
            ir::Control::While(wh) => {
                // Rewrite port use
                if let Some(new_port) = self.get(&wh.port) {
                    wh.port = new_port;
                }
                // Rewrite conditional comb group if defined
                if let Some(cg_ref) = &wh.cond {
                    let cg = cg_ref.borrow().name();
                    if let Some(new_cg) = &comb_group_map.get(&cg) {
                        wh.cond = Some(Rc::clone(new_cg));
                    }
                }
                // rewrite body
                self.rewrite_control(
                    &mut wh.body,
                    group_map,
                    comb_group_map,
                    static_group_map,
                );
            }
            ir::Control::Invoke(inv) => {
                self.rewrite_invoke(inv, comb_group_map)
            }
            ir::Control::Static(s) => self.rewrite_static_control(
                s,
                group_map,
                comb_group_map,
                static_group_map,
            ),
        }
    }
}
