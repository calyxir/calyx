use bitvec::prelude::*;
use std::convert::TryInto;

// Lsb0 means [10010] gives 0 at index 0, 1 at index 1, 0 at index 2, etc
// from documentation, usize is the best data type to use in bitvec.
#[derive(Debug)]
pub struct ValueError {}

#[derive(Clone, Debug, Default)]
/// The type of all inputs and outputs to all components in Calyx.
/// Wraps a BitVector.
pub struct Value {
    // Lsb0 means the 0th index contains the LSB. This is useful because
    // a 7-bit bitvector and 17-bit bitvector representing the number 6 have
    // ones in the same index.
    pub vec: BitVec<Lsb0, u64>,
}

impl Value {
    /// Creates a Value with the specified bandwidth.
    ///
    /// # Example:
    /// ```
    /// use interp::values::*;
    /// let empty_val = Value::new(2 as usize);
    /// ```
    pub fn new(bitwidth: usize) -> Value {
        Value::zeroes(bitwidth)
    }

    /// Creates a new Value initialized to all 0s given a bitwidth.
    ///
    /// # Example:
    /// ```
    /// use interp::values::*;
    /// let zeroed_val = Value::zeroes(2 as usize);
    /// ```
    pub fn zeroes(bitwidth: usize) -> Value {
        Value {
            vec: bitvec![Lsb0, u64; 0; bitwidth],
        }
    }

    /// Creates a new Value of a given bitwidth out of an initial_val. It's
    /// safer to use [try_from_init] followed by [unwrap].
    ///
    /// # Example:
    /// ```
    /// use interp::values::*;
    /// let val_16_16 = Value::from_init(16 as u64, 16 as usize);
    /// ```
    pub fn from_init<T1: Into<u64>, T2: Into<usize>>(
        initial_val: T1,
        bitwidth: T2,
    ) -> Self {
        let mut vec = BitVec::from_element(initial_val.into());
        vec.resize(bitwidth.into(), false);
        Value { vec }
    }

    /// Create a new Value of a given bitwidth out of an initial_val. You do
    /// not have to guarantee initial_val satisifies Into<u64>, or bitwidth
    /// satisfies Into<usize>.
    ///
    /// # Example:
    /// ```
    /// use interp::values::*;
    /// let val_16_16 = Value::try_from_init(16, 16).unwrap();
    /// ```
    pub fn try_from_init<T1, T2>(
        initial_val: T1,
        bitwidth: T2,
    ) -> Result<Self, ValueError>
    where
        T1: TryInto<u64>,
        T2: TryInto<usize>,
    {
        let (val, width): (u64, usize) =
            match (initial_val.try_into(), bitwidth.try_into()) {
                (Ok(v1), Ok(v2)) => (v1, v2),
                _ => return Err(ValueError {}),
            };

        let mut vec = BitVec::from_element(val);
        vec.resize(width, false);
        Ok(Value { vec })
    }

    /// Returns a Value containing a vector of length 0, effectively returning
    /// a cleared vector.
    pub fn clear(&self) -> Self {
        let mut vec = self.vec.clone();
        vec.truncate(0);
        Value { vec }
    }

    /// Returns a Value truncated to length [new_size].
    ///
    /// # Example
    /// ```
    /// use interp::values::*;
    /// let val_4_4 = (Value::try_from_init(4, 16).unwrap()).truncate(4);
    /// ```
    pub fn truncate(&self, new_size: usize) -> Value {
        let mut vec = self.vec.clone();
        vec.truncate(new_size);
        Value { vec }
    }

    /// Zero-extend the vector to length [ext].
    ///
    /// # Example:
    /// ```
    /// use interp::values::*;
    /// let val_4_16 = (Value::try_from_init(4, 4).unwrap()).ext(16);
    /// ```
    pub fn ext(&self, ext: usize) -> Value {
        let mut vec = self.vec.clone();
        for _x in 0..(ext - vec.len()) {
            vec.push(false);
        }
        Value { vec }
    }

    /// Sign-extend the vector to length [ext].
    ///
    /// # Example:
    /// ```
    /// use interp::values::*;
    /// // [1111] -> [11111]. In 2'sC these are both -1
    /// let val_31_5 = (Value::try_from_init(15, 4).unwrap()).sext(5);
    /// ```
    pub fn sext(&self, ext: usize) -> Value {
        let mut vec = self.vec.clone();
        let sign = vec[vec.len() - 1];
        for _x in 0..(ext - vec.len()) {
            vec.push(sign);
        }
        Value { vec }
    }

    /// Converts value into u64 type. Vector within Value can be of any width.
    ///
    /// # Example
    /// ```
    /// use interp::values::*;
    /// let unsign_64_16 = (Value::try_from_init(16, 16).unwrap()).as_u64();
    /// ```
    pub fn as_u64(&self) -> u64 {
        let mut val: u64 = 0;
        for (index, bit) in self.vec.iter().by_ref().enumerate() {
            val += u64::pow(2, (index as usize).try_into().unwrap())
                * (*bit as u64);
        }
        val
    }
}

/* ============== Impls for Values to make them easier to use ============= */
#[allow(clippy::from_over_into)]
impl Into<u64> for Value {
    fn into(self) -> u64 {
        let mut val: u64 = 0;
        for (index, bit) in self.vec.into_iter().enumerate() {
            val += u64::pow(2, (index as usize).try_into().unwrap())
                * (bit as u64);
        }
        val
    }
}

impl std::fmt::Display for Value {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> Result<(), std::fmt::Error> {
        let mut vec_rev = self.vec.clone();
        vec_rev.reverse();
        write!(f, "{}", vec_rev)
    }
}

/// A TimeLockedValue represents the return of a non-combinational component,
/// such as a register. Since a register only updates with the value of [in] by the next
/// clock cycle, it returns a TimeLockedValue at the end of [execute_mut] that
/// has a [count] of 1, [value] being the new value, and [old_value] being the previous value
/// (undetermined what goes into old_value if the register wasn't previously initialized)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeLockedValue {
    value: Value,
    count: u64,
    pub old_value: Option<Value>,
}

impl TimeLockedValue {
    /// Create a new TimeLockedValue
    /// # Example
    /// use interp::values::*;
    /// TimeLockedValue::new(
    ///                 Value::from_init(1: u16, 1: u16),
    ///                 1,
    ///                 Some(Value::zeroes(1))
    ///             )
    pub fn new(
        value: Value,
        count: u64,
        old_value: Option<Value>,
    ) -> TimeLockedValue {
        TimeLockedValue {
            value,
            count,
            old_value, //what is this again? if a read is requested at time T the value read is the value before time T
        }
    }

    /// Decrease the counter in the TLV. Once this counter is 0, the TLV is unlockable
    /// and its value can be read
    pub fn dec_count(&mut self) {
        if self.count > 0 {
            self.count -= 1
        }
    }

    /// If [self] is unlockable then [self.unlock] will guaranteed return
    /// [value].
    pub fn unlockable(&self) -> bool {
        self.count == 0
    }

    /// If [self] is unlockable then returns [value] else panics
    pub fn unlock(self) -> Value {
        if self.unlockable() {
            self.value
        } else {
            panic!("Value cannot be unlocked")
        }
    }

    /// Safer version of [unlock]. Returns an OutputValue. Returns
    /// ImmediateValue(self.value) if [self] is unlockable, else returns
    /// LockedValue(self).
    pub fn try_unlock(self) -> OutputValue {
        if self.unlockable() {
            OutputValue::ImmediateValue(self.value)
        } else {
            OutputValue::LockedValue(self)
        }
    }

    /// Mainly for testing. Gets the value of the [count] in [self]
    pub fn get_count(&self) -> u64 {
        self.count
    }
}

/// The return type for all primitive components. Combinational components
/// return [ImmediateValue], which is a wrapper for [Value]. Sequential components
/// such as registers and memories return [LockedValue], which contains a TimeLockedValue
/// within it.
#[derive(Clone, Debug)]
pub enum OutputValue {
    ImmediateValue(Value),
    LockedValue(TimeLockedValue),
    PulseValue(PulseValue),
}

impl OutputValue {
    /// Returns the Value contained within an ImmediateValue. Panics if
    /// called on a LockedValue
    pub fn unwrap_imm(self) -> Value {
        match self {
            OutputValue::ImmediateValue(val) => val,
            _ => panic!("not an immediate value, cannot unwrap_imm"),
        }
    }
    /// Returns the TimeLockedValue contained within a LockedValue. Panics if
    /// called on a ImmediateValue
    pub fn unwrap_tlv(self) -> TimeLockedValue {
        match self {
            OutputValue::LockedValue(tlv) => tlv,
            _ => panic!("not a TimeLockedValue value, cannot unwrap_tlv"),
        }
    }

    pub fn is_imm(&self) -> bool {
        matches!(self, OutputValue::ImmediateValue(_))
    }
    pub fn is_tlv(&self) -> bool {
        matches!(self, OutputValue::LockedValue(_))
    }
}

impl From<Value> for OutputValue {
    fn from(input: Value) -> Self {
        OutputValue::ImmediateValue(input)
    }
}

impl From<TimeLockedValue> for OutputValue {
    fn from(input: TimeLockedValue) -> Self {
        OutputValue::LockedValue(input)
    }
}

impl From<PulseValue> for OutputValue {
    fn from(input: PulseValue) -> Self {
        OutputValue::PulseValue(input)
    }
}

/// Returns an uninitialized immediate value.
impl Default for OutputValue {
    fn default() -> Self {
        Value::default().into()
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.vec.len() == other.vec.len() && self.vec == other.vec
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        assert!(self.vec.len() == other.vec.len());
        Some(self.vec.cmp(&other.vec))
    }
}

pub trait ReadableValue {
    fn get_val(&self) -> &Value;
}

pub trait TickableValue {
    fn tick(&mut self);
    fn do_tick(self) -> OutputValue;
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PulseState {
    Low,  // inital state of a pulse
    High, // the moment the pulse is high
}

/// A return type for primitive components which marks a value with three
/// states, and initial low value, a high value held for the pulse length, and
/// finally returning to the original low value. This is similar to
/// TimeLockedValue but returns to the original value when done, rather than
/// replacing it.
//
// This is used primarially for outputs like "done" which have a fixed time in
// which they are held high.
//
// As a note, the high and low values don't need to have any explicit ordering
// and, if so desired, this may be used to pulse a lower value rather than a
// high value
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PulseValue {
    high_val: Value,
    low_val: Value,
    state: PulseState,
    pulse_length: u64, // how long the value is high
    current_length: u64,
}

impl PulseValue {
    /// Returns a new PulseValue in the inital low state
    pub fn new(high_val: Value, low_val: Value, pulse_length: u64) -> Self {
        Self {
            high_val,
            low_val,
            state: PulseState::Low,
            pulse_length,
            current_length: 0,
        }
    }

    /// Consumes the PulesValue and returns the value appropriate for the
    /// current state
    pub fn take_val(self) -> Value {
        match self.state {
            PulseState::Low => self.low_val,
            PulseState::High => self.high_val,
        }
    }

    /// A convenience constructor which automatically initializes the pulse with
    /// a length of 1
    pub fn one_cycle_pulse(high_val: Value, low_val: Value) -> Self {
        Self::new(high_val, low_val, 1)
    }

    /// A convenience constructor for the common use of representing "done"
    /// signals
    pub fn one_cycle_one_bit_pulse() -> Self {
        Self::one_cycle_pulse(Value::from_init(1_u16, 1_u16), Value::zeroes(1))
    }
}

impl TickableValue for PulseValue {
    fn tick(&mut self) {
        match &self.state {
            PulseState::Low => self.state = PulseState::High,
            PulseState::High => self.current_length += 1,
        }
    }

    fn do_tick(mut self) -> OutputValue {
        self.tick();
        if self.pulse_length == self.current_length {
            let v = self.low_val.into();
            v
        } else {
            self.into()
        }
    }
}

impl ReadableValue for PulseValue {
    fn get_val(&self) -> &Value {
        match &self.state {
            PulseState::Low => &self.low_val,
            PulseState::High => &self.high_val,
        }
    }
}

impl ReadableValue for Value {
    fn get_val(&self) -> &Value {
        &self
    }
}

impl ReadableValue for TimeLockedValue {
    fn get_val(&self) -> &Value {
        match &self.old_value {
            Some(v) => v,
            None => panic!("Trying to read invalid value"),
        }
    }
}

impl TickableValue for TimeLockedValue {
    fn tick(&mut self) {
        self.dec_count()
    }

    fn do_tick(mut self) -> OutputValue {
        self.tick();
        self.try_unlock()
    }
}

impl ReadableValue for OutputValue {
    fn get_val(&self) -> &Value {
        match &self {
            OutputValue::ImmediateValue(iv) => iv.get_val(),
            OutputValue::LockedValue(tlv) => tlv.get_val(),
            OutputValue::PulseValue(pv) => pv.get_val(),
        }
    }
}

impl OutputValue {
    pub fn do_tick(self) -> OutputValue {
        match self {
            OutputValue::ImmediateValue(v) => v.into(),
            OutputValue::LockedValue(v) => v.do_tick(),
            OutputValue::PulseValue(v) => v.do_tick(),
        }
    }
}
