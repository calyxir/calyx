use crate::flatten::{
    flat_ir::prelude::{Assignment, AssignmentIdx, GlobalCellIdx},
    structures::context::Context,
};

use super::env::AssignmentRange;

/// A collection of assignments represented using a series of half-open ranges
/// via [AssignmentRange]
#[derive(Debug)]
pub(crate) struct AssignmentBundle {
    assigns: Vec<(GlobalCellIdx, AssignmentRange)>,
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
    pub fn push(&mut self, value: (GlobalCellIdx, AssignmentRange)) {
        self.assigns.push(value)
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = &(GlobalCellIdx, AssignmentRange)> {
        self.assigns.iter()
    }

    pub fn iter_over_indices(
        &self,
    ) -> impl Iterator<Item = (GlobalCellIdx, AssignmentIdx)> + '_ {
        self.assigns
            .iter()
            .flat_map(|(c, x)| x.iter().map(|y| (*c, y)))
    }

    pub fn iter_over_assignments<'a>(
        &'a self,
        ctx: &'a Context,
    ) -> impl Iterator<Item = (GlobalCellIdx, &'a Assignment)> {
        self.iter_over_indices()
            .map(|(c, idx)| (c, &ctx.primary[idx]))
    }

    /// The total number of assignments. Not the total number of index ranges!
    pub fn len(&self) -> usize {
        self.assigns
            .iter()
            .fold(0, |acc, (_, range)| acc + range.size())
    }
}

impl FromIterator<(GlobalCellIdx, AssignmentRange)> for AssignmentBundle {
    fn from_iter<T: IntoIterator<Item = (GlobalCellIdx, AssignmentRange)>>(
        iter: T,
    ) -> Self {
        Self {
            assigns: iter.into_iter().collect(),
        }
    }
}
