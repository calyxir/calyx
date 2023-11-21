use crate::flatten::{
    flat_ir::prelude::{Assignment, AssignmentIdx, GlobalCellId},
    structures::context::Context,
};

use super::env::AssignmentRange;

/// A collection of assignments represented using a series of half-open ranges
/// via [AssignmentRange]
#[derive(Debug)]
pub(crate) struct AssignmentBundle {
    assigns: Vec<(GlobalCellId, AssignmentRange)>,
}

impl AssignmentBundle {
    pub fn new() -> Self {
        Self {
            assigns: Vec::new(),
        }
    }

    fn with_capacity(size: usize) -> Self {
        Self {
            assigns: Vec::with_capacity(size),
        }
    }

    #[inline]
    pub fn push(&mut self, value: (GlobalCellId, AssignmentRange)) {
        self.assigns.push(value)
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = &(GlobalCellId, AssignmentRange)> {
        self.assigns.iter()
    }

    pub fn iter_over_indices(
        &self,
    ) -> impl Iterator<Item = (GlobalCellId, AssignmentIdx)> + '_ {
        self.assigns
            .iter()
            .flat_map(|(c, x)| x.iter().map(|y| (*c, y)))
    }

    pub fn iter_over_assignments<'a>(
        &'a self,
        ctx: &'a Context,
    ) -> impl Iterator<Item = (GlobalCellId, &'a Assignment)> {
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

impl FromIterator<(GlobalCellId, AssignmentRange)> for AssignmentBundle {
    fn from_iter<T: IntoIterator<Item = (GlobalCellId, AssignmentRange)>>(
        iter: T,
    ) -> Self {
        Self {
            assigns: iter.into_iter().collect(),
        }
    }
}
