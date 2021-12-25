use crate::ir::{self, RRC};
use std::collections::HashMap;
use std::rc::Rc;

/// IR Rewriter. Defines methods to rewrite various parts of the IR using
/// rewrite maps.
pub struct Rewriter;

/// Map to rewrite cell uses. Maps name of the old cell to the new [ir::Cell]
/// instance.
pub type CellRewriteMap = HashMap<ir::Id, RRC<ir::Cell>>;

/// Map to rewrite port uses. Maps the canonical name of an old port (generated using
/// [ir::Port::canonical]) to the new [ir::Port] instance.
pub type PortRewriteMap = HashMap<(ir::Id, ir::Id), RRC<ir::Port>>;

impl Rewriter {
    /// Get [ir::Port] with the same name as `port` from `cell`.
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
}
