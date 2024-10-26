# Core Library

This library defines a standard set of components used in most Calyx programs
such as registers and basic bitwise operations.

## Contents

- [Numerical Operators](#numerical-operators)
- [Logical Operators](#logical-operators)
- [Comparison Operators](#comparison-operators)
- [Floating Point](#floating-point)
- [Memories](#memories)

---

## Numerical Operators

### `std_reg<WIDTH>`

A `WIDTH`-wide register.

**Inputs:**

- `in: WIDTH` - An input value to the register `WIDTH`-bits.
- `write_en: 1` - The one bit write enabled signal. Indicates that the register
  should store the value on the `in` wire.

**Outputs:**

- `out: WIDTH` - The value contained in the register.
- `done: 1` - The register's done signal. Set high for one cycle after writing a
  new value.

---

### `std_const<WIDTH,VAL>`

A constant WIDTH-bit value with value VAL.

**Inputs:** None.

**Outputs:**

- `out: WIDTH` - The value of the constant (i.e. `VAL`).

---

### `std_lsh<WIDTH>`

A left bit shift. Performs `left << right`. This component is combinational.

**Inputs:**

- `left: WIDTH` - A WIDTH-bit value to be shifted.
- `right: WIDTH` - A WIDTH-bit value representing the shift amount.

**Outputs:**

- `out: WIDTH` - A WIDTH-bit value equivalent to `left << right`.

---

### `std_rsh<WIDTH>`

A right bit shift. Performs `left >> right`. This component is combinational.

**Inputs:**

- `left: WIDTH` - A WIDTH-bit value to be shifted.
- `right: WIDTH` - A WIDTH-bit value representing the shift amount.

**Outputs:**

- `out: WIDTH` - A WIDTH-bit value equivalent to `left >> right`.

---

### `std_cat<WIDTH0, WIDTH1>`

Concatenate two values. This component is combinational.

**Inputs:**

- `left: WIDTH0` - A WIDTH0-bit value
- `right: WIDTH1` - A WIDTH1-bit value

**Outputs:**

- `out: WIDTH0 + WIDTH1` - A WIDTH0 + WIDTH1-bit value equivalent to `(left << WIDTH1) || right`

---

### `std_add<WIDTH>`

Bitwise addition without a carry flag. Performs `left + right`. This component
is combinational.

**Inputs:**

- `left: WIDTH` - A WIDTH-bit value.
- `right: WIDTH` - A WIDTH-bit value.

**Outputs:**

- `out: WIDTH` - A WIDTH-bit value equivalent to `left + right`.

---

### `std_sub<WIDTH>`

Bitwise subtraction. Performs `left - right`. This component is combinational.

**Inputs:**

- `left: WIDTH` - A WIDTH-bit value.
- `right: WIDTH` - A WIDTH-bit value.

**Outputs:**

- `out: WIDTH` - A WIDTH-bit value equivalent to `left - right`.

---

### `std_slice<IN_WIDTH, OUT_WIDTH>`

Slice out the lower OUT_WIDTH bits of an IN_WIDTH-bit value. Computes
`in[OUT_WIDTH - 1 : 0]`. This component is combinational.

**Inputs:**

- `in: IN_WIDTH` - An IN_WIDTH-bit value.

**Outputs:**

- `out: OUT_WIDTH` - The lower OUT_WIDTH bits of `in`.

---
### `std_bit_slice<IN_WIDTH, START_IDX, END_IDX, OUT_WIDTH>`
Extract the bit-string starting at `START_IDX` and ending at `END_IDX - 1` from `in`.
This is computed as `in[END_IDX:START_IDX]`.`OUT_WIDTH` must be specified to
be `END_WIDTH - START_WITH` wide when instantiating the module.


**Inputs:**
- `in: IN_WIDTH` - An IN_WIDTH-bit value.

**Outputs:**

- `out: OUT_WIDTH` - The value of the bit-string `in[START_IDX:END_IDX]`.
---
### `std_pad<IN_WIDTH, OUT_WIDTH>`

Given an IN_WIDTH-bit input, zero pad from the left to an output of
OUT_WIDTH-bits. This component is combinational.

**Inputs:**

- `in: IN_WIDTH` - An IN_WIDTH-bit value to be padded.

**Outputs:**

- `out: OUT_WIDTH` - The padded value.

---

## Logical Operators

### `std_not<WIDTH>`

Bitwise NOT. This component is combinational.

**Inputs:**

- `in: WIDTH` - A WIDTH-bit input.

**Outputs:**

- `out: WIDTH` - The bitwise NOT of the input (`~in`).

---

### `std_and<WIDTH>`

Bitwise AND. This component is combinational.

**Inputs:**

- `left: WIDTH` - A WIDTH-bit argument.
- `right: WIDTH` - A WIDTH-bit argument.

**Outputs:**

- `out: WIDTH` - The bitwise AND of the arguments (`left & right`).

---

### `std_or<WIDTH>`

Bitwise OR. This component is combinational.

**Inputs:**

- `left: WIDTH` - A WIDTH-bit argument.
- `right: WIDTH` - A WIDTH-bit argument.

**Outputs:**

- `out: WIDTH` - The bitwise OR of the arguments (`left | right`).

---

### `std_xor<WIDTH>`

Bitwise XOR. This component is combinational.

**Inputs:**

- `left: WIDTH` - A WIDTH-bit argument.
- `right: WIDTH` - A WIDTH-bit argument.

**Outputs:**

- `out: WIDTH` - The bitwise XOR of the arguments (`left ^ right`).

---

## Comparison Operators

### `std_gt<WIDTH>`

Greater than. This component is combinational.

**Inputs:**

- `left: WIDTH` - A WIDTH-bit argument.
- `right: WIDTH` - A WIDTH-bit argument.

**Outputs:**

- `out: 1` - A single bit output. 1 if `left > right` else 0.

---

### `std_lt<WIDTH>`

Less than. This component is combinational.

**Inputs:**

- `left: WIDTH` - A WIDTH-bit argument.
- `right: WIDTH` - A WIDTH-bit argument.

**Outputs:**

- `out: 1` - A single bit output. 1 if `left < right` else 0.

---

### `std_eq<WIDTH>`

Equality comparison. This component is combinational.

**Inputs:**

- `left: WIDTH` - A WIDTH-bit argument.
- `right: WIDTH` - A WIDTH-bit argument.

**Outputs:**

- `out: 1` - A single bit output. 1 if `left = right` else 0.

---

### `std_neq<WIDTH>`

Not equal. This component is combinational.

**Inputs:**

- `left: WIDTH` - A WIDTH-bit argument.
- `right: WIDTH` - A WIDTH-bit argument.

**Outputs:**

- `out: 1` - A single bit output. 1 if `left != right` else 0.

---

### `std_ge<WIDTH>`

Greater than or equal. This component is combinational.

**Inputs:**

- `left: WIDTH` - A WIDTH-bit argument.
- `right: WIDTH` - A WIDTH-bit argument.

**Outputs:**

- `out: 1` - A single bit output. 1 if `left >= right` else 0.

---

### `std_le<WIDTH>`

Less than or equal. This component is combinational.

**Inputs:**

- `left: WIDTH` - A WIDTH-bit argument.
- `right: WIDTH` - A WIDTH-bit argument.

**Outputs:**

- `out: 1` - A single bit output. 1 if `left <= right` else 0.

---

## Floating Point

### `std_float_const`

A floating-point constant with a particular representation and bitwidth.
Floating-point values are specially parsed by the frontend and turned into the equivalent bit pattern (as dictated by the representation).
Similarly, the backend supports specialized printing for constants based on the representation

**Parameters:**

- `REP`: The representation to use. `0` corresponds to [IEEE-754 floating point][ieee754] numbers. No other representation is supported at this point.
- `WIDTH`: Bitwidth to use. Supported values: `32`, `64`.
- `VAL`: The floating-point value. Frontend converts this into a `u64` internally.


[ieee754]: https://en.wikipedia.org/wiki/IEEE_754

---

## Memories

Calyx features two flavors of memories: combinational and sequential.
Combinational memories promise that they will return `mem[addr]` in the same cycle that `addr` is provided.
Sequential memories, on the other hand, promise that they will return `mem[addr]` in the next cycle after `addr` is provided.
We generally encourage the use of sequential memories as they are more realistic.
Combinational memories are useful when the memory is known to be small, and the application is very performance-sensitive.

### `seq_mem_d1`

A one-dimensional memory with sequential reads.

**Parameters:**

- `WIDTH` - Size of an individual memory slot.
- `SIZE` - Number of slots in the memory.
- `IDX_SIZE` - The width of the index given to the memory.

**Inputs:**

- `addr0: IDX_SIZE` - The index to be accessed or updated.
- `write_data: WIDTH` - Data to be written to the selected memory slot.
- `write_en: 1` - One bit write enabled signal. Used in concert with `content_en`; see below.
- `content_en: 1` - One bit content enabled signal. When `content_en` is high and `write_en` is low, the memory reads the value stored at `addr0` and latches it. When `write_en` and `content_en` are both high, the memory writes `write_data` to the slot indexed by `addr0` and `read_data` is undefined.
- `reset: 1` - A reset signal that overrides all other interface signals and sets the latched output of the memory to `0`.

**Outputs:**

- `read_data: WIDTH` - The value stored at `addr0`. This value is available once `done` goes high.
- `done: 1`: The done signal for the memory. This signal goes high once a read or write operation is complete. In this case, this happens a cycle after the operation is requested.

---

### `seq_mem_d2`

A two-dimensional memory with sequential reads.

**Parameters:**
- `WIDTH` - Size of an individual memory slot.
- `D0_SIZE` - Number of memory slots for the first index.
- `D1_SIZE` - Number of memory slots for the second index.
- `D0_IDX_SIZE` - The width of the first index.
- `D1_IDX_SIZE` - The width of the second index.

**Inputs:**
- `addr0: D0_IDX_SIZE` - The first index into the memory.
- `addr1: D1_IDX_SIZE` - The second index into the memory.
- `write_data: WIDTH` - Data to be written to the selected memory slot.
- `write_en: 1` - One bit write enabled signal. Used in concert with `content_en`; see below.
- `content_en: 1` - One bit content enabled signal. When `content_en` is high, the memory reads the value stored at `addr0` and `addr1` and latches it. When `write_en` and `content_en` are both high, the memory writes `write_data` to the slot indexed by `addr0` and `addr1` and `read_data` is undefined.
- `reset: 1` - A reset signal that overrides all other interface signals and sets the latched output of the memory to `0`.

**Outputs:**
- `read_data: WIDTH` - The value stored at `mem[addr0][addr1]`. This value is available once `done` goes high.
- `done: 1`: The done signal for the memory. This signal goes high once a read or write operation is complete. In this case, this happens a cycle after the operation is requested.

---

### `seq_mem_d3`

A three-dimensional memory with sequential reads.

**Parameters:**
- `WIDTH` - Size of an individual memory slot.
- `D0_SIZE` - Number of memory slots for the first index.
- `D1_SIZE` - Number of memory slots for the second index.
- `D2_SIZE` - Number of memory slots for the third index.
- `D0_IDX_SIZE` - The width of the first index.
- `D1_IDX_SIZE` - The width of the second index.
- `D2_IDX_SIZE` - The width of the third index.

**Inputs:**
- `addr0: D0_IDX_SIZE` - The first index into the memory.
- `addr1: D1_IDX_SIZE` - The second index into the memory.
- `addr2: D2_IDX_SIZE` - The third index into the memory.
- `write_data: WIDTH` - Data to be written to the selected memory slot.
- `write_en: 1` - One bit write enabled signal. Used in concert with `content_en`; see below.
- `content_en: 1` - One bit content enabled signal. When `content_en` is high, the memory reads the value stored at `addr0`, `addr1`, and `addr2` and latches it. When `write_en` and `content_en` are both high, the memory writes `write_data` to the slot indexed by `addr0`, `addr1`, and `addr2` and `read_data` is undefined.
- `reset: 1` - A reset signal that overrides all other interface signals and sets the latched output of the memory to `0`.

**Outputs:**
- `read_data: WIDTH` - The value stored at `mem[addr0][addr1][addr2]`. This value is available once `done` goes high.
- `done: 1`: The done signal for the memory. This signal goes high once a read or write operation is complete. In this case, this happens a cycle after the operation is requested.

---

### `seq_mem_d4`

A four-dimensional memory with sequential reads.

**Parameters:**
- `WIDTH` - Size of an individual memory slot.
- `D0_SIZE` - Number of memory slots for the first index.
- `D1_SIZE` - Number of memory slots for the second index.
- `D2_SIZE` - Number of memory slots for the third index.
- `D3_SIZE` - Number of memory slots for the fourth index.
- `D0_IDX_SIZE` - The width of the first index.
- `D1_IDX_SIZE` - The width of the second index.
- `D2_IDX_SIZE` - The width of the third index.
- `D3_IDX_SIZE` - The width of the fourth index.

**Inputs:**
- `addr0: D0_IDX_SIZE` - The first index into the memory.
- `addr1: D1_IDX_SIZE` - The second index into the memory.
- `addr2: D2_IDX_SIZE` - The third index into the memory.
- `addr3: D3_IDX_SIZE` - The fourth index into the memory.
- `write_data: WIDTH` - Data to be written to the selected memory slot.
- `write_en: 1` - One bit write enabled signal. Used in concert with `content_en`; see below.
- `content_en: 1` - One bit content enabled signal. When `content_en` is high, the memory reads the value stored at `addr0`, `addr1`, `addr2`, and `addr3` and latches it. When `write_en` and `content_en` are both high, the memory writes `write_data` to the slot indexed by `addr0`, `addr1`, `addr2`, and `addr3` and `read_data` is undefined.
- `reset: 1` - A reset signal that overrides all other interface signals and sets the latched output of the memory to `0`.

**Outputs:**
- `read_data: WIDTH` - The value stored at `mem[addr0][addr1][addr2][addr3]`. This value is available once `done` goes high.
- `done: 1`: The done signal for the memory. This signal goes high once a read or write operation is complete. In this case, this happens a cycle after the operation is requested.

---

### `comb_mem_d1`

A one-dimensional memory with combinational reads.

**Parameters:**

- `WIDTH` - Size of an individual memory slot.
- `SIZE` - Number of slots in the memory.
- `IDX_SIZE` - The width of the index given to the memory.

**Inputs:**

- `addr0: IDX_SIZE` - The index to be accessed or updated.
- `write_data: WIDTH` - Data to be written to the selected memory slot.
- `write_en: 1` - One bit write enabled signal, causes the memory to write `write_data` to the slot indexed by `addr0`.

**Outputs:**

- `read_data: WIDTH` - The value stored at `addr0`. This value is combinational with respect to `addr0`.
- `done: 1`: The done signal for the memory. This signal goes high for one cycle after finishing a write to the memory.

---

### `comb_mem_d2`

A two-dimensional memory with combinational reads.

**Parameters:**

- `WIDTH` - Size of an individual memory slot.
- `D0_SIZE` - Number of memory slots for the first index.
- `D1_SIZE` - Number of memory slots for the second index.
- `D0_IDX_SIZE` - The width of the first index.
- `D1_IDX_SIZE` - The width of the second index.

**Inputs:**

- `addr0: D0_IDX_SIZE` - The first index into the memory.
- `addr1: D1_IDX_SIZE` - The second index into the memory.
- `write_data: WIDTH` - Data to be written to the selected memory slot.
- `write_en: 1` - One bit write enabled signal, causes the memory to write `write_data` to the slot indexed by `addr0` and `addr1`.

**Outputs:**

- `read_data: WIDTH` - The value stored at `mem[addr0][addr1]`. This value is combinational with respect to `addr0` and `addr1`.
- `done: 1`: The done signal for the memory. This signal goes high for one cycle after finishing a write to the memory.

---

### `comb_mem_d3`

A three-dimensional memory with combinational reads.

**Parameters:**

- `WIDTH` - Size of an individual memory slot.
- `D0_SIZE` - Number of memory slots for the first index.
- `D1_SIZE` - Number of memory slots for the second index.
- `D2_SIZE` - Number of memory slots for the third index.
- `D0_IDX_SIZE` - The width of the first index.
- `D1_IDX_SIZE` - The width of the second index.
- `D2_IDX_SIZE` - The width of the third index.

**Inputs:**

- `addr0: D0_IDX_SIZE` - The first index into the memory.
- `addr1: D1_IDX_SIZE` - The second index into the memory.
- `addr2: D2_IDX_SIZE` - The third index into the memory.
- `write_data: WIDTH` - Data to be written to the selected memory slot.
- `write_en: 1` - One bit write enabled signal, causes the memory to write `write_data` to the slot indexed by `addr0`, `addr1`, and `addr2`.

**Outputs:**

- `read_data: WIDTH` - The value stored at `mem[addr0][addr1][addr2]`. This value is combinational with respect to `addr0`, `addr1`, and `addr2`.
- `done: 1`: The done signal for the memory. This signal goes high for one cycle after finishing a write to the memory.

---

### `comb_mem_d4`

A four-dimensional memory with combinational reads.

**Parameters:**

- `WIDTH` - Size of an individual memory slot.
- `D0_SIZE` - Number of memory slots for the first index.
- `D1_SIZE` - Number of memory slots for the second index.
- `D2_SIZE` - Number of memory slots for the third index.
- `D3_SIZE` - Number of memory slots for the fourth index.
- `D0_IDX_SIZE` - The width of the first index.
- `D1_IDX_SIZE` - The width of the second index.
- `D2_IDX_SIZE` - The width of the third index.
- `D3_IDX_SIZE` - The width of the fourth index.

**Inputs:**

- `addr0: D0_IDX_SIZE` - The first index into the memory.
- `addr1: D1_IDX_SIZE` - The second index into the memory.
- `addr2: D2_IDX_SIZE` - The third index into the memory.
- `addr3: D3_IDX_SIZE` - The fourth index into the memory.
- `write_data: WIDTH` - Data to be written to the selected memory slot.
- `write_en: 1` - One bit write enabled signal, causes the memory to write `write_data` to the slot indexed by `addr0`, `addr1`, `addr2`, and `addr3`.

**Outputs:**

- `read_data: WIDTH` - The value stored at `mem[addr0][addr1][addr2][addr3]`. This value is combinational with respect to `addr0`, `addr1`, `addr2`, and `addr3`.
- `done: 1`: The done signal for the memory. This signal goes high for one cycle after finishing a write to the memory.
