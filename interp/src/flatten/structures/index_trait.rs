// TODO(griffin): Replace with cranelift_entity if this ends up being the same
pub trait IndexRef: Copy + Eq {
    fn index(&self) -> usize;
    fn new(_input: usize) -> Self;
}

macro_rules! impl_index {
    ( $struct_name: ident, $backing_ty: ty) => {
        #[derive(Debug, Eq, Copy, Clone, PartialEq)]
        struct $struct_name($backing_ty);

        impl IndexRef for $struct_name {
            fn index(&self) -> usize {
                self.0 as usize
            }

            fn new(input: usize) -> Self {
                Self(input as $backing_ty)
            }
        }
    };

    ($struct_name: ident) => {
        impl_index!($struct_name, u32);
    };
}

pub(crate) use impl_index;
