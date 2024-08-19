use ibig::ops::RemEuclid;

use crate::{
    flatten::{
        flat_ir::prelude::*,
        primitives::{
            declare_ports,
            utils::{floored_division, int_sqrt, ShiftBuffer},
        },
    },
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

#[derive(Debug, Clone)]
pub struct Sqrt<const IS_FIXED_POINT: bool> {
    base_port: GlobalPortIdx,
    output: PortValue,
    done_is_high: bool,
    width: u32,
    frac_width: Option<u32>,
}

impl<const IS_FIXED_POINT: bool> Sqrt<IS_FIXED_POINT> {
    declare_ports!(_CLK: 0, RESET: 1, GO: 2, IN: 3, OUT: 4, DONE: 5);
    pub fn new(
        base_port: GlobalPortIdx,
        width: u32,
        frac_width: Option<u32>,
    ) -> Self {
        Self {
            base_port,
            output: PortValue::new_undef(),
            done_is_high: false,
            width,
            frac_width,
        }
    }
}

impl<const IS_FIXED_POINT: bool> Primitive for Sqrt<IS_FIXED_POINT> {
    fn exec_comb(&self, port_map: &mut PortMap) -> UpdateResult {
        ports![&self.base_port; out: Self::OUT, done: Self::DONE];

        let done_changed = port_map.set_done(done, self.done_is_high)?;
        let out_changed =
            port_map.write_exact_unchecked(out, self.output.clone());

        Ok(out_changed | done_changed)
    }

    fn exec_cycle(&mut self, port_map: &mut PortMap) -> UpdateResult {
        ports![&self.base_port;
            reset: Self::RESET,
            go: Self::GO,
            done: Self::DONE,
            in_val: Self::IN,
            out: Self::OUT
        ];

        if port_map[reset].as_bool().unwrap_or_default() {
            self.done_is_high = false;
            self.output = PortValue::new_cell(Value::zeroes(self.width));
        } else if port_map[go].as_bool().unwrap_or_default() {
            let input = port_map[in_val].as_option();
            if let Some(input) = input {
                self.output = if IS_FIXED_POINT {
                    let val = int_sqrt(
                        &(input.val().as_unsigned()
                            << (self.frac_width.unwrap() as usize)),
                    );
                    PortValue::new_cell(Value::from(val, self.width))
                } else {
                    let val = int_sqrt(&input.val().as_unsigned());
                    PortValue::new_cell(Value::from(val, self.width))
                };
            } else {
                // TODO griffin: should probably put an error or warning here?
                self.output = PortValue::new_undef();
            }
            self.done_is_high = true;
        } else {
            self.done_is_high = false;
        }

        let done_signal = port_map.set_done(done, self.done_is_high)?;
        let out_changed =
            port_map.write_exact_unchecked(out, self.output.clone());

        Ok(out_changed | done_signal)
    }
}

pub struct FxpMultPipe<const DEPTH: usize> {
    base_port: GlobalPortIdx,
    pipeline: ShiftBuffer<(PortValue, PortValue), DEPTH>,
    current_output: PortValue,
    int_width: u32,
    frac_width: u32,
    done_is_high: bool,
}

impl<const DEPTH: usize> FxpMultPipe<DEPTH> {
    declare_ports![_CLK: 0, RESET: 1, GO: 2, LEFT: 3, RIGHT: 4, OUT: 5, DONE: 6];
    pub fn new(
        base_port: GlobalPortIdx,
        int_width: u32,
        frac_width: u32,
    ) -> Self {
        Self {
            base_port,
            pipeline: ShiftBuffer::default(),
            current_output: PortValue::new_cell(Value::zeroes(
                int_width + frac_width,
            )),
            int_width,
            frac_width,
            done_is_high: false,
        }
    }
}

impl<const DEPTH: usize> Primitive for FxpMultPipe<DEPTH> {
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
            self.current_output = PortValue::new_cell(Value::zeroes(
                self.int_width + self.frac_width,
            ));
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
                            2 * (self.frac_width + self.int_width),
                        )
                        .slice_out(
                            self.frac_width as usize,
                            (2 * self.frac_width + self.int_width) as usize,
                        )
                    })
                });
                self.current_output =
                    out_val.map_or(PortValue::new_undef(), PortValue::new_cell);
                self.done_is_high = true;
            } else {
                self.current_output = PortValue::new_cell(Value::zeroes(
                    self.frac_width + self.int_width,
                ));
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

pub struct FxpDivPipe<const DEPTH: usize, const SIGNED: bool> {
    base_port: GlobalPortIdx,
    pipeline: ShiftBuffer<(PortValue, PortValue), DEPTH>,
    output_quotient: PortValue,
    output_remainder: PortValue,
    int_width: u32,
    frac_width: u32,
    done_is_high: bool,
}

impl<const DEPTH: usize, const SIGNED: bool> FxpDivPipe<DEPTH, SIGNED> {
    declare_ports![_CLK: 0, RESET: 1, GO: 2, LEFT: 3, RIGHT: 4, OUT_REMAINDER: 5, OUT_QUOTIENT: 6, DONE: 7];
    pub fn new(
        base_port: GlobalPortIdx,
        int_width: u32,
        frac_width: u32,
    ) -> Self {
        Self {
            base_port,
            pipeline: ShiftBuffer::default(),
            output_quotient: PortValue::new_cell(Value::zeroes(int_width)),
            output_remainder: PortValue::new_cell(Value::zeroes(
                frac_width + int_width,
            )),
            int_width,
            frac_width,
            done_is_high: false,
        }
    }

    fn width(&self) -> u32 {
        self.int_width + self.frac_width
    }
}

impl<const DEPTH: usize, const SIGNED: bool> Primitive
    for FxpDivPipe<DEPTH, SIGNED>
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
                PortValue::new_cell(Value::zeroes(self.width()));
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
                                    ((left.val().as_unsigned()
                                        << self.frac_width as usize)
                                        / right.val().as_unsigned())
                                    .into()
                                } else {
                                    ((left.val().as_signed()
                                        << self.frac_width as usize)
                                        / right.val().as_signed())
                                    .into()
                                },
                                self.width(),
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
                                self.width(),
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
                    PortValue::new_cell(Value::zeroes(self.width()));
                self.output_remainder =
                    PortValue::new_cell(Value::zeroes(self.width()));
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
