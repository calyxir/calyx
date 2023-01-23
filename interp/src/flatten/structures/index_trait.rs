// TODO(griffin): Replace with cranelift_entity if this ends up being the same
pub trait IndexRef: Copy + Eq {
    fn index(&self) -> usize;
    fn new(input: usize) -> Self;
}

macro_rules! impl_index {
    ($v: vis $struct_name: ident) => {
        impl_index!($v $struct_name, u32);
    };

    ( $v:vis $struct_name: ident, $backing_ty: ty) => {
        #[derive(Debug, Eq, Copy, Clone, PartialEq, Hash)]
        $v struct $struct_name($backing_ty);

        impl $crate::flatten::structures::index_trait::IndexRef for $struct_name {
            fn index(&self) -> usize {
                self.0 as usize
            }

            fn new(input: usize) -> Self {
                Self(input as $backing_ty)
            }
        }

        impl From<$backing_ty> for $struct_name {
            fn from(input: $backing_ty) -> Self {
                $struct_name(input)
            }
        }
    };
}

pub(crate) use impl_index;

#[derive(Debug)]
pub struct IndexRange<I>
where
    I: IndexRef,
{
    /// The start of the range (inclusive).
    start: I,
    /// The end of the range (inclusive).
    end: I,
}
