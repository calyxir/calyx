#[macro_export]
/// This macro is used to implement the IndexRef trait for a type that wraps an
/// unsigned integer value. By default, the macro will implement the trait using
/// a [`u32`](std::u32) as the backing type. However, if a different backing type
/// is desired, it can be specified as the second argument.
macro_rules! impl_index {
    ($struct_name: ident) => {
        impl_index!($struct_name, u32);
    };

    ($struct_name: ident, $backing_ty: ty) => {
        impl $crate::IndexRef for $struct_name {
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

        impl From<usize> for $struct_name {
            fn from(input: usize) -> Self {
                $crate::IndexRef::new(input)
            }
        }
    };
}

#[macro_export]
/// This macro is used to implement the IndexRef trait for a type that wraps a
/// NonZero value. By default, the macro will implement the trait using a
/// [`NonZeroU32`](std::num::NonZeroU32) as the backing type. However, if a
/// different backing type is desired, it can be specified as the second
/// argument to the macro.
macro_rules! impl_index_nonzero {
    // Cool and normal stuff here
    ($struct_name: ident) => {
        impl_index_nonzero!($struct_name, std::num::NonZeroU32, u32);
    };

    ($struct_name: ident, NonZeroU8) => {
        impl_index_nonzero!($struct_name, std::num::NonZeroU8, u8);
    };

    ($struct_name: ident, NonZeroU16) => {
        impl_index_nonzero!($struct_name, std::num::NonZeroU16, u16);
    };

    ($struct_name: ident, NonZeroU32) => {
        impl_index_nonzero!($struct_name, std::num::NonZeroU32, u32);
    };

    ($struct_name: ident, $non_zero_type:ty, $normal_type:ty) => {
        impl $crate::IndexRef for $struct_name {
            fn index(&self) -> usize {
                self.0.get() as usize - 1
            }

            fn new(input: usize) -> Self {
                Self(
                    <$non_zero_type>::new((input + 1) as $normal_type).unwrap(),
                )
            }
        }

        impl From<$non_zero_type> for $struct_name {
            fn from(input: $non_zero_type) -> Self {
                $struct_name(input)
            }
        }

        impl From<usize> for $struct_name {
            fn from(input: usize) -> Self {
                $crate::IndexRef::new(input)
            }
        }
    };
}
