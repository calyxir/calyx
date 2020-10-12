//! Implements a formatter for the in-memory representation of Components.
//! The printing operation clones inner nodes and doesn't perform any mutation
//! to the Component.
use crate::ir;
use std::fmt;

/// Printer for the IR.
pub struct IRPrinter {}

impl IRPrinter {
    /// Format a given Component into a printable string.
    pub fn print(comp: &ir::Component) -> fmt::Result {
        unimplemented!()
    }

    /// Format a given cell into a printable string.
    pub fn print_cell(cell: &ir::Cell) -> fmt::Result {
        unimplemented!()
    }

    /// Format a given assignment into a printable string.
    pub fn print_assignment(assign: &ir::Assignment) -> fmt::Result {
        unimplemented!()
    }

    /// Format a given group into a printable string.
    pub fn print_group(group: &ir::Group) -> fmt::Result {
        unimplemented!()
    }

    /// Format a control program into a printable string.
    pub fn print_control(control: &ir::Control) -> fmt::Result {
        unimplemented!()
    }
}
