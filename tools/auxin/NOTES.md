# design comments

- numbers being human-readable *and* arbitrarily precise was not prioritised. 
  - doing text -> text conversion and expecting fidelity seems largely pointless *unless* from json -> untyped
- under current representation scheme, types only matter for checking, they don't change the value of ``data``
- permit future expansion, but do not necessarily require it.

## ops

ops will be stored in the typeprops table: enum of different op types

basic ops (below) will always be supported, blanket implementation provided
- truncate_bits, where itype.width > otype.width
- bitcast, where itype.width <= otype.width. end padded with zeroes
- sign-extend, where itype.width <= otype.width

otherwise, add a field to typeprops for recording

saturating / wrapping / overflowing / value-preserving / generally 'different' operations

conversion types (add to enum: possibly more in the future)
(binrep, itype, otype) -> Result<binrep, err>
(binrep, itype, otype) -> binrep

for future: want ways to specify:
- functions from domain X to Y, for specific segments of domain X and domain Y.
- multiple possible functions which map from X to Y (different notions of granularity)


# legacy elements
## hex directory reference

hex directories / 'dat dir' is currently a special case bolted to the cider data interface.

the format is a directory containing one file per memory. each file contains the memory contents as hexadecimal strings (i.e. ``0x1234``), with one line per 'word'. it seems like everything is zero-padded out to the 'word width'

these seem largely to interact with ``readmemh`` and ``writememh``.

relevant files:
- ``fud2/rsrc/json-dat.py``
- ``fud/fud/stages/verilator/json_to_dat.py``

while the 'interpreter' python equivalent to ``json_to_dat.py`` is still in-tree, it seems that it is no longer used.

## (eventually) standardising number parsing

- there were two 'data converter' looking things in-tree, one for the data directories, another for cider.
  - cocotb/yxi kind of requires 'half' of one, as it reads the json
- in terms of related problem of number parsing, there are a few elements to standardise:
  - ``calyx/utils``: contains ``std_float_constant`` parsing. also contains ``bits_needed_for``, ``get_bit_width_from``, which are functions that a 'number-y' library should *probably* also implement?
  - there doesn't really seem to be a 'canonical' source for fixed-point conversions: python scripts don't actually read literals, there is not a unique 'fixed-point constant' primitive. fixed-point primitives are instead represented inside bitnums.
  - not sure if fixed-point parsing exists within the calyx compiler frontend.

# future work

- functionality
  - add 'real' cast functionality via ops: other than bitcasts, also support some value-preserving casts.
  - add equivalents of ``write_string`` for ``std::format::Formatter`` (basically a display-like)

- library work
  - add 'guarded' constructors for TypeClass, fixed-point, which checks if values are permitted
  - some redundancy between the ``vbfp`` 'definition' of fixed-point and TypeSpec. probably get rid of this
  - endianness support is shoddy, ``Endian`` is not ``Copy``.
  - deprecate the ``bits`` typeclass or properly define it
  - solidify dimension handling rules (see flattened output section)

- optimisation
  - formats / ``num_ir``: there is still an excessive amount of string cleaning for hexstrings
    - we can also use the width of a type to hint how many characters to read.
  - unfortunately data still gets copied around. when serializing in particular there are areas where ``Strings`` are probably used unnecessarily.
  - many other places

- baa improvements / extensions:
  - mutable iterator to elements in the bitvec arrays? (or generally, more intuitive approaches to working with array values)
  - ``Vec<BitVecValue>`` duplicates metadata for each ``BitVecValue``. find an alternative.
  - smarter approaches to 'batching' an op over an array: how can we permit some data sharing while also limiting what a mutator can do to the array?
  - upstream a ``to_dec_str_signed`` function

- json interface
  - ``struson`` is streaming. better for memory efficiency when reading files. however, currently, reading json requires two passes through: one to read types, one to read data. it'd be nice to figure out an overall 'better' way of managing this
  - ``struson``'s ``next_number_as_str`` is still pretty slow, not necessarily due to validity checks. i *think* it's because it goes byte-by-byte?
  - find a better solution to ``stdin`` reads than copying it. 

- plain csv format
  - building off of the directory format, we could have CSVs to represent each memory rather than the hex ``.dat`` format
  - this is very possible, but isn't a priority.

## a note on flattened output

(discuss why shaped output is bad. thus, will *read* shaped json, but will not produce it.)
(shape will be retained when reading cider information to IR. ir also has support for writing it out. this is *not* necessary, though)

(shape not retained when reading / writing dir, always 1d)
