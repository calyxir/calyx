use crate::ir::{self, RRC};
use std::collections::HashMap;
use std::rc::Rc;

use super::CloneName;

/// IR Rewriter. Defines methods to rewrite various parts of the IR using
/// rewrite maps.
pub struct Rewriter;

/// Map to rewrite cell uses. Maps name of the old cell to the new [ir::Cell]
/// instance.
pub type CellRewriteMap = HashMap<ir::Id, RRC<ir::Cell>>;

/// Map to rewrite port uses. Maps the canonical name of an old port (generated using
/// [ir::Port::canonical]) to the new [ir::Port] instance.
pub type PortRewriteMap = HashMap<(ir::Id, ir::Id), RRC<ir::Port>>;

/// Map name of old group to new group
type GroupRewriteMap = HashMap<ir::Id, RRC<ir::Group>>;
/// Map name of old combination group to new combinational group
type CombGroupRewriteMap = HashMap<ir::Id, RRC<ir::CombGroup>>;

impl Rewriter {
    /// Get [ir::Port] with the same name as `port` from `cell`.
    /// Panics if a port with the same name doesn't exist.
    fn get_port(port: &RRC<ir::Port>, cell: &RRC<ir::Cell>) -> RRC<ir::Port> {
        Rc::clone(&cell.borrow().get(&port.borrow().name))
    }

    /// Return a port rewrite if it is defeind in the set of rewrites.
    fn get_port_rewrite(
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
        rewrite.map(|new_cell| Self::get_port(port, new_cell))
    }

    /// Rewrite reads and writes from `cell` in the given assingments to
    /// the same ports on `new_cell`.
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
    pub fn rename_cell_use(
        rewrites: &CellRewriteMap,
        assign: &mut ir::Assignment,
    ) {
        if let Some(new_port) = Self::get_port_rewrite(rewrites, &assign.src) {
            assign.src = new_port;
        }
        if let Some(new_port) = Self::get_port_rewrite(rewrites, &assign.dst) {
            assign.dst = new_port;
        }
        assign.guard.for_each(&|port| {
            Self::get_port_rewrite(rewrites, &port).map(ir::Guard::port)
        });
    }

    /// Convinience wrapper around [Self::rename_cell_use] that operates
    /// over given set of assignments.
    pub fn rename_cell_uses(
        rewrites: &CellRewriteMap,
        assigns: &mut Vec<ir::Assignment>,
    ) {
        for assign in assigns {
            Self::rename_cell_use(rewrites, assign)
        }
    }

    /// Rename uses of specific ports if they are defined within `rewrites`
    /// to the mapped port.
    /// Uses [ir::Port::canonical] values as the key.
    #[inline]
    pub fn rename_port_use(
        rewrites: &PortRewriteMap,
        assign: &mut ir::Assignment,
    ) {
        let new_src = rewrites
            .get(&assign.src.borrow().canonical())
            .map(Rc::clone);
        if let Some(src) = new_src {
            assign.src = src;
        }

        let new_dst = rewrites
            .get(&assign.dst.borrow().canonical())
            .map(Rc::clone);
        if let Some(dst) = new_dst {
            assign.dst = dst;
        }

        assign.guard.for_each(&|port| {
            rewrites
                .get(&port.borrow().canonical())
                .map(|p| ir::Guard::port(Rc::clone(p)))
        });
    }

    /// Rewrite a `invoke` node using a [CellRewriteMap] and a [CombGroupRewriteMap]
    pub fn rewrite_invoke(
        inv: &mut ir::Invoke,
        cell_map: &CellRewriteMap,
        comb_group_map: &CombGroupRewriteMap,
    ) {
        // Rewrite the name of the cell
        let name = inv.comp.borrow().clone_name();
        if let Some(new_cell) = &cell_map.get(&name) {
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
                if let Some(new_port) = Self::get_port_rewrite(cell_map, &*port)
                {
                    *port = new_port;
                }
            });
    }

    /// Given a control program, rewrite all uses of cells, groups, and comb groups using the given
    /// rewrite maps.
    pub fn rewrite_control(
        c: &mut ir::Control,
        cell_map: &CellRewriteMap,
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
                        cell_map,
                        group_map,
                        comb_group_map,
                    )
                })
            }
            ir::Control::If(ife) => {
                // Rewrite port use
                if let Some(new_port) =
                    Self::get_port_rewrite(cell_map, &ife.port)
                {
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
                    cell_map,
                    group_map,
                    comb_group_map,
                );
                Self::rewrite_control(
                    &mut ife.fbranch,
                    cell_map,
                    group_map,
                    comb_group_map,
                );
            }
            ir::Control::While(wh) => {
                // Rewrite port use
                if let Some(new_port) =
                    Self::get_port_rewrite(cell_map, &wh.port)
                {
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
                    cell_map,
                    group_map,
                    comb_group_map,
                );
            }
            ir::Control::Invoke(inv) => {
                Self::rewrite_invoke(inv, cell_map, comb_group_map)
            }
        }
    }
}
