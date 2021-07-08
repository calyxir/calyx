use super::{Primitive, Serializeable};
use crate::values::{PulseValue, TimeLockedValue, Value};
use calyx::ir;

/// A register.
#[derive(Default)]
pub struct StdReg {
    pub width: u64,
    pub data: [Value; 1],
    update: Option<Value>,
}

impl StdReg {
    pub fn new(params: ir::Binding) -> Self {
        let width = params
            .iter()
            .find(|(n, _)| n.as_ref() == "WIDTH")
            .expect("Missing `width` param from std_reg binding")
            .1;
        StdReg {
            width,
            data: [Value::new(width as usize)],
            update: None,
        }
    }
}

impl Primitive for StdReg {
    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(calyx::ir::Id, &Value)]) {
        todo!()
    }

    fn execute(
        &mut self,
        inputs: &[(calyx::ir::Id, &Value)],
        done_val: Option<&Value>,
    ) -> Vec<(calyx::ir::Id, crate::values::OutputValue)> {
        //unwrap the arguments
        let (_, input) = inputs.iter().find(|(id, _)| id == "in").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        //write the input to the register
        if write_en.as_u64() == 1 {
            self.update = Some((*input).clone());
            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new register data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("out"),
                    TimeLockedValue::new(
                        (*input).clone(),
                        1,
                        Some(self.data[0].clone()),
                    )
                    .into(),
                ),
                (
                    "done".into(),
                    PulseValue::new(
                        // XXX(rachit): Do we always expect done_val to exist
                        // here?
                        done_val.unwrap().clone(),
                        Value::bit_high(),
                        Value::bit_low(),
                        1,
                    )
                    .into(),
                ),
            ]
        } else {
            // if write_en was low, so done is 0 b/c nothing was written here
            // in this vector i
            // OUT: the old value in the register, b/c we couldn't write
            // DONE: not TimeLockedValue, b/c it's just 0, b/c our write was unsuccessful
            vec![(ir::Id::from("out"), self.data[0].clone().into())]
        }
    }

    fn reset(
        &mut self,
        _: &[(calyx::ir::Id, &Value)],
    ) -> Vec<(calyx::ir::Id, crate::values::OutputValue)> {
        vec![
            (ir::Id::from("out"), self.data[0].clone().into()),
            (ir::Id::from("done"), Value::zeroes(1).into()),
        ]
    }

    fn commit_updates(&mut self) {
        if let Some(val) = self.update.take() {
            self.data[0] = val;
        }
    }

    fn clear_update_buffer(&mut self) {
        self.update = None;
    }

    fn serialize(&self) -> Serializeable {
        Serializeable::Val(self.data[0].as_u64())
    }
}
