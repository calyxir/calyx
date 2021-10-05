mod component;
mod control;

pub use component::Component;
pub use control::{Control, Empty, Enable, If, Invoke, Par, While};

use std::rc::Rc;
pub type ComponentCtx = Rc<Vec<Rc<component::Component>>>;
pub type ContinuousAssignments = Rc<Vec<calyx::ir::Assignment>>;
