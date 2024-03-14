use crate::flatten::{
    flat_ir::prelude::{
        Assignment, AssignmentIdx, GlobalCellIdx, LocalPortOffset,
    },
    structures::context::Context,
};

use super::env::AssignmentRange;

#[derive(Debug)]
pub struct GroupInterfacePorts {
    pub go: LocalPortOffset,
    pub done: LocalPortOffset,
}

/// A collection of assignments represented using a series of half-open ranges
/// via [AssignmentRange]
#[derive(Debug)]
pub(crate) struct AssignmentBundle {
    assigns: Vec<(GlobalCellIdx, AssignmentRange, Option<GroupInterfacePorts>)>,
}

// TODO griffin: remove the dead stuff later
#[allow(dead_code)]
impl AssignmentBundle {
    pub fn new() -> Self {
        Self {
            assigns: Vec::new(),
        }
    }

    pub fn with_capacity(size: usize) -> Self {
        Self {
            assigns: Vec::with_capacity(size),
        }
    }

    #[inline]
    pub fn push(
        &mut self,
        value: (GlobalCellIdx, AssignmentRange, Option<GroupInterfacePorts>),
    ) {
        self.assigns.push(value)
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<
        Item = &(GlobalCellIdx, AssignmentRange, Option<GroupInterfacePorts>),
    > {
        self.assigns.iter()
    }

    /// The total number of assignments. Not the total number of index ranges!
    pub fn len(&self) -> usize {
        self.assigns
            .iter()
            .fold(0, |acc, (_, range, _)| acc + range.size())
    }
}

impl FromIterator<(GlobalCellIdx, AssignmentRange, Option<GroupInterfacePorts>)>
    for AssignmentBundle
{
    fn from_iter<
        T: IntoIterator<
            Item = (
                GlobalCellIdx,
                AssignmentRange,
                Option<GroupInterfacePorts>,
            ),
        >,
    >(
        iter: T,
    ) -> Self {
        Self {
            assigns: iter.into_iter().collect(),
        }
    }
}
