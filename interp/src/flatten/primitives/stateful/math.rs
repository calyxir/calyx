use ibig::ops::RemEuclid;

use crate::{
    flatten::{flat_ir::prelude::*, primitives::declare_ports},
    primitives::{prim_utils::ShiftBuffer, stateful::floored_division},
    values::InputNumber,
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

        let out_changed =
            port_map.write_exact_unchecked(out, self.current_output.clone());

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

pub struct StdDivPipe<const DEPTH: usize, const SIGNED: bool> {
    base_port: GlobalPortIdx,
    pipeline: ShiftBuffer<(PortValue, PortValue), DEPTH>,
    output_quotient: PortValue,
    output_remainder: PortValue,
    width: u32,
    done_is_high: bool,
}

impl<const DEPTH: usize, const SIGNED: bool> StdDivPipe<DEPTH, SIGNED> {
    declare_ports![_CLK: 0, RESET: 1, GO: 2, LEFT: 3, RIGHT: 4, OUT_QUOTIENT: 5, OUT_REMAINDER: 6, DONE: 7];
    pub fn new(base_port: GlobalPortIdx, width: u32) -> Self {
        Self {
            base_port,
            pipeline: ShiftBuffer::default(),
            output_quotient: PortValue::new_cell(Value::zeroes(width)),
            output_remainder: PortValue::new_cell(Value::zeroes(width)),
            width,
            done_is_high: false,
        }
    }
}

impl<const DEPTH: usize, const SIGNED: bool> Primitive
    for StdDivPipe<DEPTH, SIGNED>
{
    fn exec_comb(&self, port_map: &mut PortMap) -> UpdateResult {
        ports![&self.base_port;
               out_quot: Self::OUT_QUOTIENT,
               out_rem: Self::OUT_REMAINDER,
               done: Self::DONE];

        let quot_changed = port_map
            .write_exact_unchecked(out_quot, self.output_quotient.clone());
        let rem_changed = port_map
            .write_exact_unchecked(out_rem, self.output_remainder.clone());

        let done_signal = port_map.set_done(done, self.done_is_high)?;

        Ok(quot_changed | rem_changed | done_signal)
    }

    fn exec_cycle(&mut self, port_map: &mut PortMap) -> UpdateResult {
        ports![&self.base_port;
            left: Self::LEFT,
            right: Self::RIGHT,
            reset: Self::RESET,
            go: Self::GO,
            out_quot: Self::OUT_QUOTIENT,
            out_rem: Self::OUT_REMAINDER,
            done: Self::DONE
        ];

        if port_map[reset].as_bool().unwrap_or_default() {
            self.output_quotient =
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
                        (
                            Value::from::<InputNumber, _>(
                                if !SIGNED {
                                    (left.val().as_unsigned()
                                        / right.val().as_unsigned())
                                    .into()
                                } else {
                                    (left.val().as_signed()
                                        / right.val().as_signed())
                                    .into()
                                },
                                self.width,
                            ),
                            Value::from::<InputNumber, _>(
                                if !SIGNED {
                                    (left
                                        .val()
                                        .as_unsigned()
                                        .rem_euclid(right.val().as_unsigned()))
                                    .into()
                                } else {
                                    (left.val().as_signed()
                                        - right.val().as_signed()
                                            * floored_division(
                                                &left.val().as_signed(),
                                                &right.val().as_signed(),
                                            ))
                                    .into()
                                },
                                self.width,
                            ),
                        )
                    })
                });
                (self.output_quotient, self.output_remainder) = out_val.map_or(
                    (PortValue::new_undef(), PortValue::new_undef()),
                    |(q, r)| (PortValue::new_cell(q), PortValue::new_cell(r)),
                );
                self.done_is_high = true;
            } else {
                self.output_quotient =
                    PortValue::new_cell(Value::zeroes(self.width));
                self.output_remainder =
                    PortValue::new_cell(Value::zeroes(self.width));
                self.done_is_high = false;
            }
        } else {
            self.pipeline.reset();
            self.done_is_high = false;
        }

        let done_signal = port_map.set_done(done, self.done_is_high)?;
        let quot_changed = port_map
            .write_exact_unchecked(out_quot, self.output_quotient.clone());
        let rem_changed = port_map
            .write_exact_unchecked(out_rem, self.output_remainder.clone());

        Ok(quot_changed | rem_changed | done_signal)
    }
}
