mod primitive;
pub use primitive::Entry;
pub use primitive::Primitive;
pub use primitive::Serializeable;

pub mod combinational;
pub mod stateful;

mod prim_utils {
    use crate::values::Value;
    use calyx::ir;

    pub(super) fn get_param<S>(params: &ir::Binding, target: S) -> Option<u64>
    where
        S: AsRef<str>,
    {
        params.iter().find_map(|(id, x)| {
            if id == target.as_ref() {
                Some(*x)
            } else {
                None
            }
        })
    }

    pub(super) fn get_input<'a, S>(
        inputs: &[(calyx::ir::Id, &'a Value)],
        target: S,
    ) -> Option<&'a Value>
    where
        S: AsRef<str>,
    {
        inputs
            .iter()
            .find(|(id, _)| id == target.as_ref())
            .map(|(_, v)| *v)
    }

    pub(super) fn get_input_unwrap<'a, S>(
        inputs: &[(calyx::ir::Id, &'a Value)],
        target: S,
    ) -> &'a Value
    where
        S: AsRef<str>,
    {
        get_input(inputs, target).unwrap()
    }
}
