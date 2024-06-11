use crate::{
    flatten::{flat_ir::prelude::*, primitives::declare_ports},
    primitives::prim_utils::ShiftBuffer,
};
use crate::{
    flatten::{
        primitives::{ports, prim_trait::*},
        structures::environment::PortMap,
    },
    values::Value,
};

pub struct StdMultPipe<const DEPTH: usize> {
    base_port: GlobalPortIdx,
    pipeline: ShiftBuffer<(PortValue, PortValue), DEPTH>,
    current_output: PortValue,
    width: u32,
    done_is_high: bool,
}

impl<const DEPTH: usize> StdMultPipe<DEPTH> {
    declare_ports![_CLK: 0, RESET: 1, GO: 2, LEFT: 3, RIGHT: 4, OUT: 5, DONE: 6];
    pub fn new(base_port: GlobalPortIdx, width: u32) -> Self {
        Self {
            base_port,
            pipeline: ShiftBuffer::default(),
            current_output: PortValue::new_cell(Value::zeroes(width)),
            width,
            done_is_high: false,
        }
    }
}

impl<const DEPTH: usize> Primitive for StdMultPipe<DEPTH> {
    fn exec_comb(&self, port_map: &mut PortMap) -> UpdateResult {
        ports![&self.base_port; out: Self::OUT, done: Self::DONE];

        let out_changed = if self.current_output.is_def() {
            port_map.insert_val(
                out,
                self.current_output.as_option().unwrap().clone(),
            )?
        } else {
            UpdateStatus::Unchanged
        };

        let done_signal = port_map.insert_val(
            done,
            AssignedValue::cell_value(if self.done_is_high {
                Value::bit_high()
            } else {
                Value::bit_low()
            }),
        )?;

        Ok(out_changed | done_signal)
    }

    fn exec_cycle(&mut self, port_map: &mut PortMap) -> UpdateResult {
        ports![&self.base_port;
            left: Self::LEFT,
            right: Self::RIGHT,
            reset: Self::RESET,
            go: Self::GO,
            out: Self::OUT,
            done: Self::DONE
        ];

        if port_map[reset].as_bool().unwrap_or_default() {
            self.current_output =
                PortValue::new_cell(Value::zeroes(self.width));
            self.done_is_high = false;
            self.pipeline.reset();
        } else if port_map[go].as_bool().unwrap_or_default() {
            let output = self
                .pipeline
                .shift(Some((port_map[left].clone(), port_map[right].clone())));
            if let Some((l, r)) = output {
                let out_val = l.as_option().and_then(|left| {
                    r.as_option().map(|right| {
                        Value::from(
                            left.val().as_unsigned()
                                * right.val().as_unsigned(),
                            self.width,
                        )
                    })
                });
                self.current_output =
                    out_val.map_or(PortValue::new_undef(), PortValue::new_cell);
                self.done_is_high = true;
            } else {
                self.current_output =
                    PortValue::new_cell(Value::zeroes(self.width));
                self.done_is_high = false;
            }
        } else {
            self.pipeline.reset();
            self.done_is_high = false;
        }

        let done_signal = port_map.insert_val(
            done,
            AssignedValue::cell_value(if self.done_is_high {
                Value::bit_high()
            } else {
                Value::bit_low()
            }),
        )?;

        Ok(
            port_map.write_exact_unchecked(out, self.current_output.clone())
                | done_signal,
        )
    }
}
