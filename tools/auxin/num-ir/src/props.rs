use baa::BitVecValue;
use proptest::prelude::*;

use crate::typing::{TypeClass, TypeSpec};

// generate a typeclass which can be of arbitrary width
pub fn st_arb_width_classes() -> impl Strategy<Value = TypeClass> {
    prop_oneof![Just(TypeClass::Bits), Just(TypeClass::Int)]
}

prop_compose! {
    pub fn st_int_or_bits()(width in (1..512_usize), signed in any::<bool>(), class in st_arb_width_classes()) -> TypeSpec {
           TypeSpec{width, signed, class}

    }
}

prop_compose! {
    pub fn st_int()
    (width in (1..512_usize), signed in any::<bool>()) -> TypeSpec {
           TypeSpec{width, signed, class: TypeClass::Int}

    }

}

pub fn st_float() -> impl Strategy<Value = TypeSpec> {
    prop_oneof![
        Just(TypeSpec {
            width: 32,
            signed: false,
            class: TypeClass::Float
        }),
        Just(TypeSpec {
            width: 64,
            signed: false,
            class: TypeClass::Float
        })
    ]
}

prop_compose! {
    pub fn st_fixed(width: usize)(signed in any::<bool>(), exp_size in 0..width) -> TypeSpec {
        TypeSpec { width, signed, class: TypeClass::Fixed { exp_mag: exp_size as i32} }
    }
}

pub fn arb_type() -> BoxedStrategy<TypeSpec> {
    prop_oneof![
        st_int_or_bits(),
        st_float(),
        (1..512).prop_flat_map(|e| st_fixed(e as usize)) // generally assume types within 512 bits
    ]
    .boxed()
}

pub fn arb_type_excl_bits() -> BoxedStrategy<TypeSpec> {
    prop_oneof![
        st_int(),
        st_float(),
        (1..512).prop_flat_map(|e| st_fixed(e as usize)) // generally assume types within 512 bits
    ]
    .boxed()
}

// preferably don't use this with very large widths

pub fn bitvec_of_width(width: u32) -> impl Strategy<Value = BitVecValue> {
    Just(width)
        .prop_perturb(|centre, mut rng| BitVecValue::random(&mut rng, centre))
}

pub fn bitvec_width_max(
    max_width: u32,
) -> impl Strategy<Value = (BitVecValue, u32)> {
    let chosen_width = 1..max_width;
    chosen_width.prop_flat_map(|w| (bitvec_of_width(w), Just(w)))
}

pub fn compare_typeclasses(
    expc: &TypeSpec,
    got: &TypeSpec,
) -> Result<(), TestCaseError> {
    prop_assert_eq!(expc.width, got.width);
    // compare extra fields only on classes where it matters
    match expc.class {
        TypeClass::Int => {
            prop_assert_eq!(expc.signed, got.signed)
        }
        TypeClass::Fixed { exp_mag: _ } => {
            prop_assert_eq!(expc.signed, got.signed)
        }
        _ => (),
    }

    prop_assert_eq!(expc.class.clone(), got.class.clone());
    Ok(())
}
