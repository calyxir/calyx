# Core Library

This library defines a standard set of components used in most Calyx programs
such as registers and basic bitwise operations.

## Numerical Operators

### `std_reg<WIDTH>`

A `WIDTH`-wide register.

**Inputs:**

- `in: width` - An input value to the register `WIDTH`-bits.
- `write_en: 1` - The one bit write enabled signal. Indicates that the register
  should store the value on the in wire.

**Outputs:**

- `out: width` - The value contained in the register.
- `done: 1` - The register's done signal. Set high for one cycle after writing a
  new value.

---

### `std_const<WIDTH,VAL>`

A constant WIDTH-bit value with value VAL.

**Inputs:** None

**Outputs:**

- `out:width` - The value of the constant (i.e. `VAL`)

---

### `std_lsh<WIDTH>`

A left bit shift. Performs `LEFT << RIGHT`. This component is combinational.

**Inputs:**

- `left: width` - A WIDTH-bit value to be shifted
- `right: width` - A WIDTH-bit value representing the shift amount

**Outputs:**

- `out: width` - A WIDTH-bit value equivalent to `LEFT << RIGHT`

---

### `std_rsh<WIDTH>`

A right bit shift. Performs `LEFT >> RIGHT`. This component is combinational.

**Inputs:**

- `left: width` - A WIDTH-bit value to be shifted
- `right: width` - A WIDTH-bit value representing the shift amount

**Outputs:**

- `out: width` - A WIDTH-bit value equivalent to `LEFT >> RIGHT`

---

### `std_add<WIDTH>`

Bitwise addition without a carry flag. Performs `LEFT + RIGHT`. This component
is combinational.

**Inputs:**

- `left: width` - A WIDTH-bit value
- `right: width` - A WIDTH-bit value

**Outputs:**

- `out: width` - A WIDTH-bit value equivalent to `LEFT + RIGHT`

---

### `std_sub<WIDTH>`

Bitwise subtraction. Performs `LEFT - RIGHT`. This component is combinational.

**Inputs:**

- `left: width` - A WIDTH-bit value
- `right: width` - A WIDTH-bit value

**Outputs:**

- `out: width` - A WIDTH-bit value equivalent to `LEFT - RIGHT`

---

### `std_slice<IN_WIDTH, OUT_WIDTH>`

Slice out the lower OUT_WIDTH bits of an IN_WIDTH-bit value. Computes
`in[out_width - 1 : 0]`. This component is combinational.

**Inputs:**

- `in: in_width` - An IN_WIDTH-bit value

**Outputs:**

- `out: out_width` - The lower OUT_WIDTH bits of `in`

---

### `std_pad<IN_WIDTH, OUT_WIDTH>`

Given an IN_WIDTH-bit input, zero pad from the left to an output of
OUT_WIDTH-bits. This component is combinational.

**Inputs:**

- `in: in_width` - An IN_WIDTH-bit value to be padded

**Outputs:**

- `out: out_width` - The padded value

---

## Logical Operators

### `std_not<WIDTH>`

Bitwise NOT. This component is combinational.

**Inputs:**

- `in: width` - A WIDTH-bit input.

**Outputs:**

- `out: width` - The bitwise NOT of the input (`~in`)

---

### `std_and<WIDTH>`

Bitwise AND. This component is combinational.

**Inputs:**

- `left: width` - A WIDTH-bit argument
- `right: width` - A WIDTH-bit argument

**Outputs:**

- `out: width` - The bitwise AND of the arguments (`left & right`)

---

### `std_or<WIDTH>`

Bitwise OR. This component is combinational.

**Inputs:**

- `left: width` - A WIDTH-bit argument
- `right: width` - A WIDTH-bit argument

**Outputs:**

- `out: width` - The bitwise OR of the arguments (`left | right`)

---

### `std_xor<WIDTH>`

Bitwise XOR. This component is combinational.

**Inputs:**

- `left: width` - A WIDTH-bit argument
- `right: width` - A WIDTH-bit argument

**Outputs:**

- `out: width` - The bitwise XOR of the arguments (`left ^ right`)

---

## Comparison Ops

### `std_gt<WIDTH>`

Greater than. This component is combinational.

**Inputs:**

- `left: width` - A WIDTH-bit argument
- `right: width` - A WIDTH-bit argument

**Outputs:**

- `out: 1` - A single bit output. 1 if `left > right` else 0.

---

### `std_lt<WIDTH>`

Less than. This component is combinational.

**Inputs:**

- `left: width` - A WIDTH-bit argument
- `right: width` - A WIDTH-bit argument

**Outputs:**

- `out: 1` - A single bit output. 1 if `left < right` else 0.

---

### `std_eq<WIDTH>`

Equality comparison. This component is combinational.

**Inputs:**

- `left: width` - A WIDTH-bit argument
- `right: width` - A WIDTH-bit argument

**Outputs:**

- `out: 1` - A single bit output. 1 if `left = right` else 0.

---

### `std_neq<WIDTH>`

Not equal. This component is combinational.

**Inputs:**

- `left: width` - A WIDTH-bit argument
- `right: width` - A WIDTH-bit argument

**Outputs:**

- `out: 1` - A single bit output. 1 if `left != right` else 0.

---

### `std_ge<WIDTH>`

Greater than or equal. This component is combinational.

**Inputs:**

- `left: width` - A WIDTH-bit argument
- `right: width` - A WIDTH-bit argument

**Outputs:**

- `out: 1` - A single bit output. 1 if `left >= right` else 0.

---

### `std_le<WIDTH>`

Less than or equal. This component is combinational.

**Inputs:**

- `left: width` - A WIDTH-bit argument
- `right: width` - A WIDTH-bit argument

**Outputs:**

- `out: 1` - A single bit output. 1 if `left <= right` else 0.

---

## Memories

### `std_mem_d1`

A one-dimensional memory.

**Parameters:**

- `WIDTH` - Size of an individual memory slot.
- `SIZE` - Number of slots in the memory.
- `IDX_SIZE` - The width of the index given to the memory.

**Inputs:**

- `addr0: idx_size` - The index to be accessed or updated
- `write_data: width` - Data to be written to the selected memory slot
- `write_en: 1` - One bit write enabled signal, causes the memory to write `write_data` to the slot indexed by `addr0`

**Outputs:**

- `read_data: width` - The value stored at `addr0`. This value is combinational with respect to `addr0`.
- `done: 1`: The done signal for the memory. This signal goes high for one cycle after finishing a write to the memory.

---

### `std_mem_d2`

A two-dimensional memory.

**Parameters:**

- `WIDTH` - Size of an individual memory slot.
- `D0_SIZE` - Number of memory slots for the first index.
- `D1_SIZE` - Number of memory slots for the second index.
- `D0_IDX_SIZE` - The width of the first index.
- `D1_IDX_SIZE` - The width of the second index.

**Inputs:**

- `addr0: d0_idx_size` - The first index into the memory
- `addr1: d1_idx_size` - The second index into the memory
- `write_data: width` - Data to be written to the selected memory slot
- `write_en: 1` - One bit write enabled signal, causes the memory to write `write_data` to the slot indexed by `addr0` and `addr1`

**Outputs:**

- `read_data: width` - The value stored at `mem[addr0][addr1]`. This value is combinational with respect to `addr0` and `addr1`.
- `done: 1`: The done signal for the memory. This signal goes high for one cycle after finishing a write to the memory.

---

### `std_mem_d3`

A three-dimensional memory.

**Parameters:**

- `WIDTH` - Size of an individual memory slot.
- `D0_SIZE` - Number of memory slots for the first index.
- `D1_SIZE` - Number of memory slots for the second index.
- `D2_SIZE` - Number of memory slots for the second index.
- `D0_IDX_SIZE` - The width of the first index.
- `D1_IDX_SIZE` - The width of the second index.
- `D2_IDX_SIZE` - The width of the third index.

**Inputs:**

- `addr0: d0_idx_size` - The first index into the memory
- `addr1: d1_idx_size` - The second index into the memory
- `addr2: d1_idx_size` - The second index into the memory
- `write_data: width` - Data to be written to the selected memory slot
- `write_en: 1` - One bit write enabled signal, causes the memory to write `write_data` to the slot indexed by `addr0`, `addr1`, and `addr2`

**Outputs:**

- `read_data: width` - The value stored at `mem[addr0][addr1][addr2]`. This value is combinational with respect to `addr0`, `addr1`, and `addr2`.
- `done: 1`: The done signal for the memory. This signal goes high for one cycle after finishing a write to the memory.
