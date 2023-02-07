use std::{collections::VecDeque, marker::PhantomData, process::Output};

use super::structures::{index_trait::IndexRef, indexed_map::IndexedMap};

#[derive(Debug)]
pub struct VecHandle<'a, 'outer, In, Out, Idx>
where
    Idx: IndexRef,
{
    vec: &'a mut IndexedMap<Out, Idx>,
    queue: VecDeque<&'outer In>,
    base: Option<Idx>,
}

impl<'a, 'outer, In, Out, Idx> VecHandle<'a, 'outer, In, Out, Idx>
where
    Idx: IndexRef,
{
    pub(crate) fn new(
        vec: &'a mut IndexedMap<Out, Idx>,
        root_node: &'outer In,
        base: Option<Idx>,
    ) -> Self {
        Self {
            vec,
            queue: VecDeque::from([root_node]),
            base,
        }
    }

    pub fn enqueue(&mut self, item: &'outer In) -> Idx {
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

    pub fn finish_processing(&mut self, result: Out) -> Idx {
        self.vec.push(result);
        Idx::new(self.base.map_or(0, |x| x.index()) + self.vec.len() - 1)
    }

    pub fn next_element(&mut self) -> Option<&'outer In> {
        self.queue.pop_front()
    }

    fn produce_limited_handle(
        &mut self,
    ) -> SingleHandle<'_, 'a, 'outer, In, Out, Idx> {
        SingleHandle { handle: self }
    }
}

/// A limited handle which can only process a single element
#[derive(Debug)]
pub struct SingleHandle<'a, 'b, 'outer, In, Out, Idx>
where
    Idx: IndexRef,
{
    handle: &'a mut VecHandle<'b, 'outer, In, Out, Idx>,
}

impl<'a, 'b, 'outer, In, Out, Idx> SingleHandle<'a, 'b, 'outer, In, Out, Idx>
where
    Idx: IndexRef,
{
    pub fn enqueue(&mut self, item: &'outer In) -> Idx {
        self.handle.enqueue(item)
    }
}

pub trait FlattenTree: Sized {
    type Output;
    type IdxType: IndexRef;
    type AuxillaryData;

    fn process_element<'a: 'data, 'handle, 'vec, 'data>(
        &'a self,
        handle: SingleHandle<
            'handle,
            'vec,
            'data,
            Self,
            Self::Output,
            Self::IdxType,
        >,
        aux: &Self::AuxillaryData,
    ) -> Self::Output;
}

pub fn flatten_tree<In, Out, Idx, Aux>(
    root_node: &In,
    base: Option<Idx>,
    vec: &mut IndexedMap<Out, Idx>,
    aux: &Aux,
) -> Idx
where
    Idx: IndexRef,
    In: FlattenTree<Output = Out, IdxType = Idx, AuxillaryData = Aux>,
{
    let mut handle = VecHandle::new(vec, root_node, base);
    let mut node_idx: Option<Idx> = None;

    while let Some(node) = handle.next_element() {
        let res = node.process_element(handle.produce_limited_handle(), aux);
        node_idx.get_or_insert(handle.finish_processing(res));
    }

    node_idx.unwrap()
}
