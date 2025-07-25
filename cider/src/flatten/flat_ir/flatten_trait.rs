use std::collections::VecDeque;

use cider_idx::{IndexRef, maps::IndexedMap};

/// A handle bundling a queue of nodes to be processed and a vector of nodes that
/// have already been processed. The vec itself is not owned by the handle.
///
/// This is used by the flatten tree trait and cannot be constructed normally
/// Only uses one lifetime for the moment, but this may change in the future.
#[derive(Debug)]
struct VecHandle<'outer, In, Idx, Out>
where
    Idx: IndexRef,
{
    vec: &'outer mut IndexedMap<Idx, Out>,
    queue: VecDeque<&'outer In>,
    base: Option<Idx>,
}

impl<'outer, In, Idx, Out> VecHandle<'outer, In, Idx, Out>
where
    Idx: IndexRef,
{
    fn new(
        vec: &'outer mut IndexedMap<Idx, Out>,
        root_node: &'outer In,
        base: Option<Idx>,
    ) -> Self {
        Self {
            vec,
            queue: VecDeque::from([root_node]),
            base,
        }
    }

    fn enqueue(&mut self, item: &'outer In) -> Idx {
        self.queue.push_back(item);

        // assumes the current node is to be pushed into the finalized list
        // at some point before getting the next node for processing, thus the
        // offset calculation is (base + 1) + (vec.len() + queue.len() - 1)
        Idx::new(
            self.base.map_or(0, |x| x.index())
                + self.vec.len()
                + self.queue.len(),
        )
    }

    fn finish_processing(&mut self, result: Out) -> Idx {
        self.vec.push(result);
        Idx::new(self.base.map_or(0, |x| x.index()) + self.vec.len() - 1)
    }

    fn next_element(&mut self) -> Option<&'outer In> {
        self.queue.pop_front()
    }

    fn produce_limited_handle(
        &mut self,
    ) -> SingleHandle<'_, 'outer, In, Idx, Out> {
        SingleHandle { handle: self }
    }

    fn next_idx(&self) -> Idx {
        Idx::new(self.base.map_or(0, |x| x.index()) + self.vec.len())
    }
}

/// A limited handle which can only process a single element
/// This is only meant to be used when implementing the `FlattenTree` trait
#[derive(Debug)]
pub struct SingleHandle<'a, 'outer, In, Idx, Out>
where
    Idx: IndexRef,
{
    handle: &'a mut VecHandle<'outer, In, Idx, Out>,
}

impl<'outer, In, Idx, Out> SingleHandle<'_, 'outer, In, Idx, Out>
where
    Idx: IndexRef,
{
    pub fn enqueue(&mut self, item: &'outer In) -> Idx {
        self.handle.enqueue(item)
    }

    pub fn next_idx(&self) -> Idx {
        self.handle.next_idx()
    }
}

/// This whole trait was a deranged attempt to make some of the logic for
/// flattening a tree structure generic. I frankly recommend avoiding looking at
/// this code because it is kinda a nightmare and in hindsight should've
/// probably been designed a bit differently.
pub trait FlattenTree: Sized {
    type Output;
    type IdxType: IndexRef;
    type AuxiliaryData;
    type MutAuxiliaryData;

    fn process_element<'data>(
        &'data self,
        handle: SingleHandle<'_, 'data, Self, Self::IdxType, Self::Output>,
        aux: &Self::AuxiliaryData,
        mut_aux: &mut Self::MutAuxiliaryData,
    ) -> Self::Output;
}

pub fn flatten_tree<In, Idx, Out, Aux, MutAux>(
    root_node: &In,
    base: Option<Idx>,
    vec: &mut IndexedMap<Idx, Out>,
    aux: &Aux,
    mut_aux: &mut MutAux,
) -> Idx
where
    Idx: IndexRef,
    In: FlattenTree<
            Output = Out,
            IdxType = Idx,
            AuxiliaryData = Aux,
            MutAuxiliaryData = MutAux,
        >,
{
    let mut handle = VecHandle::new(vec, root_node, base);
    let mut root_node_idx: Option<Idx> = None;

    while let Some(node) = handle.next_element() {
        let res =
            node.process_element(handle.produce_limited_handle(), aux, mut_aux);
        root_node_idx.get_or_insert(handle.finish_processing(res));
    }

    root_node_idx.unwrap()
}
