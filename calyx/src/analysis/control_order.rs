use crate::ir;
use crate::analysis::ReadWriteSet;

/// Extract the dependency order of a list of control programs.
/// Dependencies are defined using read/write sets used in the control program.
///
/// For example, if we have control programs C1 and C2 with read sets R1 and
/// R2 and write sets W1 and W2 respectively, we can define an order relationship:
///
/// C1 < C2 if (R1 subset of W2) and (R2 disjoint W1)
/// C1 > C2 if (R2 subset of W1) and (R1 disjoint W2)
/// C1 =!= if (R1 subset of W2) and (R2 subset of W1)
struct ControlOrder;

impl ControlOrder {
    /// Return a total order for the control programs.
    /// If there is a cycle, then returns the indices of the cyclical dependencies.
    pub fn get_total_order(
        stmts: Vec<ir::Control>,
    ) -> Result<Vec<usize>, Vec<usize>> {
        todo!()
    }
}
