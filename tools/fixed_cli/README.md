# a cli to ``num_ir``

a simple 'REPL' to the functionality of ``auxin``'s ``num_ir``, suitable for determining numerical values for testing / to set constants / etc.

## usage

to start the repl,
```
  cargo run -p fixed_cli
```

valid entries:
- ``help``: displays a simple help mesasge.
- ``info <type>``: displays the type decoded through ``num_ir``. prints out the total bit width, signedness, and 'type of number'. can be used to check whether a type string is valid.
- ``<type> <number>``: read ``number`` as a number of ``type``. display both a hexadecimal and pretty-printed representation of the number.
  - ``number`` can be a hexadecimal string prefixed by ``0x``, i.e. ``0xff``.

ctrl+c or ctrl+d can be used to exit.


## type format

the repl uses ``num_ir``'s short type representation scheme, which is as follows:
- ints: ``[u/i]<SIZE>``
  - example: ``i30`` (30 bit, signed), ``u64`` (64 bit, signed).
- ieee 754 floats: ``f<SIZE>``
  - example: ``f32`` (32-bit, float), ``f64`` (64 bit, float)
  - only the 32-bit and 64-bit floating-point types are considered valid.
- fixed-point: ``d[u/i]<SIZE>:<EXP_BITS>``
  - example: ``du16:8`` (16 bit total width, 8 bits of exponent, unsigned)
  - the decimal value represented is obtained through ``decimal = (n) * 2^(-exp_bits)``, where ``n`` is the bits of the fixed-point number interpreted as an integer of width ``size``
    - example: ``di16:8 0x40 = 64 * 2^(-8) = 2^(-2) = 0.25``
  - ``exp_bits`` can be negative. however, ``num_ir``'s fixed-point library does not support these values. they can still be parsed as a valid type, but cannot be used to read numbers.
