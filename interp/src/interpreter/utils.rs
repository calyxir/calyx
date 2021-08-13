use crate::values::Value;
use calyx::ir;
use calyx::ir::RRC;
use std::cell::Ref;
use std::collections::HashSet;
use std::ops::Deref;
pub type ConstPort = *const ir::Port;
pub type ConstCell = *const ir::Cell;

pub fn get_done_port(group: &ir::Group) -> RRC<ir::Port> {
    group.get(&"done")
}

pub fn is_signal_high(done: &Value) -> bool {
    done.as_u64() == 1
}

pub fn assignment_to_string(assignment: &ir::Assignment) -> String {
    let mut str = vec![];
    ir::IRPrinter::write_assignment(assignment, 0, &mut str)
        .expect("Write Failed");
    String::from_utf8(str).expect("Found invalid UTF-8")
}

pub fn get_dest_cells<'a, I>(iter: I) -> Vec<RRC<ir::Cell>>
where
    I: Iterator<Item = &'a ir::Assignment>,
{
    let mut assign_set: HashSet<*const ir::Cell> = HashSet::new();
    iter.filter_map(|assign| {
        match &assign.dst.borrow().parent {
            ir::PortParent::Cell(c) => {
                match &c.upgrade().borrow().prototype {
                    ir::CellType::Primitive { .. }
                    | ir::CellType::Constant { .. } => {
                        let const_cell: *const ir::Cell = c.upgrade().as_ptr();
                        if assign_set.contains(&const_cell) {
                            None //b/c we don't want duplicates
                        } else {
                            assign_set.insert(const_cell);
                            Some(c.upgrade())
                        }
                    }
                    ir::CellType::Component { .. } => {
                        // TODO (griffin): We'll need to handle this case at some point
                        todo!()
                    }
                    ir::CellType::ThisComponent => None,
                }
            }
            ir::PortParent::Group(_) => None,
        }
    })
    .collect()
}
pub fn control_is_empty(control: &ir::Control) -> bool {
    match control {
        ir::Control::Seq(s) => s.stmts.iter().all(control_is_empty),
        ir::Control::Par(p) => p.stmts.iter().all(control_is_empty),
        ir::Control::If(_) => false,
        ir::Control::While(_) => false,
        ir::Control::Invoke(_) => false,
        ir::Control::Enable(_) => false,
        ir::Control::Empty(_) => true,
    }
}

pub enum ReferenceHolder<'a, T> {
    Ref(Ref<'a, T>),
    Borrow(&'a T),
}

impl<'a, T> From<&'a T> for ReferenceHolder<'a, T> {
    fn from(input: &'a T) -> Self {
        Self::Borrow(input)
    }
}

impl<'a, T> From<Ref<'a, T>> for ReferenceHolder<'a, T> {
    fn from(input: Ref<'a, T>) -> Self {
        Self::Ref(input)
    }
}

impl<'a, T> Deref for ReferenceHolder<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            ReferenceHolder::Ref(r) => r,
            ReferenceHolder::Borrow(b) => *b,
        }
    }
}
